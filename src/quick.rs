use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::{self, Duration};
use std::{fs, io};

use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use serde::Serialize;

use crate::interval::MayBeFlag;
use crate::smt::expr_to_smt_bv;
use crate::solver::{run_parallel_solver, ConstraintInfo, SearchConfig, VerificationStatus};
use crate::ui::UiState;
use crate::{
    constraint::add_blocking_constraint, interval::AbstractInterval, trace::AbstractTrace,
};

pub struct ProgramInfo {
    pub program_str: String,
    pub program_len: usize,
}

pub fn load_config(path: &std::path::Path) -> anyhow::Result<SearchConfig> {
    let text = fs::read_to_string(path)?;
    let cfg: SearchConfig = serde_yaml::from_str(&text)?;
    Ok(cfg)
}

#[derive(Parser, Debug, Serialize)]
pub struct Args {
    #[arg(long)]
    pub config: PathBuf,
    #[arg(long, default_value = "30")]
    pub num_trial: usize,
    #[arg(long, default_value = "output.yaml")]
    pub ouptput_path: PathBuf,
    #[arg(long, default_value = "bb")]
    pub method: String,
    #[arg(long, default_value = "none")]
    pub opcode_str: String,
    #[arg(long, default_value = "0")]
    pub range_interval: usize,
    #[arg(long, default_value = "false")]
    pub blocking_closure: bool,
    /// Ablation: disable the heuristic score (use constant priority, pure DFS)
    #[arg(long, default_value = "false")]
    pub no_heuristic: bool,
    /// Ablation: disable constraint simplification (is_zero / word-range pattern rewriting)
    #[arg(long, default_value = "false")]
    pub no_simplify: bool,
    /// Ablation: disable interval refinement (ABIR / conditional-constraint back-propagation)
    #[arg(long, default_value = "false")]
    pub no_refinement: bool,
    /// Explicitly disable the terminal UI (useful for scripts and batch runs)
    #[arg(long, default_value = "false")]
    pub turn_off_ui: bool,
    /// Stop immediately after the first failed trial (useful for batch scripts)
    #[arg(long, default_value = "true")]
    pub fail_fast: bool,
}

#[derive(Debug)]
pub struct VerificationResult {
    pub status: VerificationStatus,
    pub num_solutions: usize,
    pub num_total_trials: usize,
    pub execution_time: std::time::Duration,
    pub area: i128,
}

impl VerificationResult {
    pub fn is_verified(&self) -> bool {
        matches!(self.status, VerificationStatus::Verified)
    }
}

pub fn experiment_harness<FinalCheckFn, PostProcessFn>(
    program_info: &ProgramInfo,
    constraint_info: &mut ConstraintInfo,
    search_config: &SearchConfig,
    base_abs_main_trace_data: &Vec<Vec<AbstractInterval>>,
    public_vals: Vec<AbstractInterval>,
    blocked_rows: &Vec<usize>,
    post_process: PostProcessFn,
    final_check: FinalCheckFn,
    verification_method: &String,
    known_solution: &mut HashSet<String>,
    turn_off_ui: bool,
) -> Result<VerificationResult, io::Error>
where
    FinalCheckFn:
        Fn(&AbstractTrace, usize, u32, &mut HashSet<String>, &mut UiState, &mut i128) + Clone,
    PostProcessFn: Fn(&mut AbstractTrace, u32) -> MayBeFlag + Clone + Send + Sync + 'static,
{
    let mut sleep_time = Duration::from_millis(0);

    // ######################## Blocking Closures ################################
    for i in blocked_rows {
        add_blocking_constraint(
            &constraint_info.output_columns,
            &mut constraint_info.constraints,
            &base_abs_main_trace_data,
            *i,
        );
    }

    // ######################## Solve ############################################
    if verification_method == "bb" {
        let mut known_solution_area = 0;
        quick_api(
            program_info.program_str.clone(),
            constraint_info,
            &base_abs_main_trace_data,
            public_vals,
            &search_config,
            post_process,
            final_check,
            &mut sleep_time,
            known_solution,
            &mut known_solution_area,
            turn_off_ui,
        )
    } else {
        // ######################## Generating SMT Formula ##########################
        let constants: Vec<_> = (0..constraint_info.num_total_columns)
            .filter(|j| !constraint_info.refinable_cols.contains(j))
            .map(|j| (0, j, base_abs_main_trace_data[0][j].clone()))
            .collect();
        let neg_constants: Vec<_> = constraint_info
            .output_columns
            .iter()
            .map(|&j| (0, j, base_abs_main_trace_data[0][j].clone()))
            .collect();

        // Columns to block after each found solution: the free input columns
        // (those given an Any range by --range-interval) so the solver is
        // forced to find a *different* input assignment on each iteration.
        // This replicates what B&B does by continuing its search after each
        // found trace.
        use crate::solver::RangeType;
        let block_cols: Vec<usize> = constraint_info
            .range_types
            .iter()
            .filter_map(|(j, k)| {
                if matches!(k, RangeType::Any(..)) {
                    Some(*j)
                } else {
                    None
                }
            })
            .collect();

        // Generate the base formula once; strip the trailing check-sat/get-model
        // so we can insert per-iteration blocking clauses before them.
        let base_smt = expr_to_smt_bv(
            &constraint_info.constraints,
            &constants,
            &neg_constants,
            &constraint_info.range_types,
            base_abs_main_trace_data.len(),
            constraint_info.num_total_columns,
            constraint_info.num_pv_columns,
            constraint_info.prime,
        );
        const CHECK_SUFFIX: &str = "(check-sat)\n(get-model)\n";
        let base_without_check = base_smt
            .strip_suffix(CHECK_SUFFIX)
            .unwrap_or(&base_smt);

        let smt_file_path = "voutput/smt_query.smt2";
        let start_time = time::Instant::now();
        let mut blocking_clauses = String::new();
        let mut num_solutions: usize = 0;

        // ######################### Enumeration loop ###############################
        // Each iteration queries the solver with all previously found solutions
        // blocked out.  We stop when the solver returns `unsat` (region exhausted)
        // or the wall-clock budget runs out, mirroring B&B's exhaustive search
        // within a bounded input region.
        loop {
            let elapsed_ms = start_time.elapsed().as_millis() as u64;
            if elapsed_ms >= search_config.time_out_ms {
                return Ok(VerificationResult {
                    status: VerificationStatus::TimedOut,
                    num_total_trials: num_solutions,
                    num_solutions,
                    execution_time: start_time.elapsed() - sleep_time,
                    area: 1,
                });
            }
            let remaining_ms = search_config.time_out_ms - elapsed_ms;

            // Write the formula (base + accumulated blocking clauses) to disk.
            let full_smt =
                format!("{}{}{}", base_without_check, blocking_clauses, CHECK_SUFFIX);
            let mut file = File::create(smt_file_path).expect("Failed to create SMT file");
            file.write_all(full_smt.as_bytes())
                .expect("Failed to write SMT string to file");

            let output = Command::new(verification_method)
                .arg(format!("-t:{}", remaining_ms))
                .arg(smt_file_path)
                .output()
                .expect("Failed to execute SMT solver");
            let stdout = String::from_utf8_lossy(&output.stdout);

            if stdout.contains("unsat") {
                // No more solutions exist in the region.
                return Ok(VerificationResult {
                    status: VerificationStatus::Verified,
                    num_total_trials: num_solutions,
                    num_solutions,
                    execution_time: start_time.elapsed() - sleep_time,
                    area: 1,
                });
            } else if stdout.contains("sat") {
                num_solutions += 1;

                // If there are no free input columns to block on (range_interval
                // was 0), we cannot enumerate further — treat this as a single
                // counterexample and exit.
                if block_cols.is_empty() {
                    return Ok(VerificationResult {
                        status: VerificationStatus::Verified,
                        num_total_trials: num_solutions,
                        num_solutions,
                        execution_time: start_time.elapsed() - sleep_time,
                        area: 1,
                    });
                }

                // Parse the model to extract concrete values for the free input
                // columns, then add a blocking clause preventing this exact
                // input combination from appearing in future iterations.
                let col_vals =
                    crate::smt::parse_bv_model(&stdout, 0, &block_cols);
                let clause = crate::smt::build_blocking_clause(0, &col_vals);
                if clause.is_empty() {
                    // Model parse failed — cannot make progress; bail out.
                    return Ok(VerificationResult {
                        status: VerificationStatus::TimedOut,
                        num_total_trials: num_solutions,
                        num_solutions,
                        execution_time: start_time.elapsed() - sleep_time,
                        area: 1,
                    });
                }
                blocking_clauses.push_str(&clause);
            } else {
                // Solver returned neither sat nor unsat (timeout / error).
                return Ok(VerificationResult {
                    status: VerificationStatus::TimedOut,
                    num_total_trials: num_solutions,
                    num_solutions,
                    execution_time: start_time.elapsed() - sleep_time,
                    area: 1,
                });
            }
        }
    }
}

pub fn quick_api<FinalCheckFn, PostProcessFn>(
    program_str: String,
    constraint_info: &ConstraintInfo,
    base_abs_main_trace_data: &Vec<Vec<AbstractInterval>>,
    public_vals: Vec<AbstractInterval>,
    search_config: &SearchConfig,
    align_pc_to_program: PostProcessFn,
    final_check: FinalCheckFn,
    sleep_time: &mut Duration,
    known_solution: &mut HashSet<String>,
    known_solution_area: &mut i128,
    turn_off_ui: bool,
) -> Result<VerificationResult, io::Error>
where
    FinalCheckFn:
        Fn(&AbstractTrace, usize, u32, &mut HashSet<String>, &mut UiState, &mut i128) + Clone,
    PostProcessFn: Fn(&mut AbstractTrace, u32) -> MayBeFlag + Clone + Send + Sync + 'static,
{
    // Attempt TUI setup; degrade gracefully when stdout is not a real
    // terminal (e.g. when the binary is invoked as a subprocess from
    // compare_experiments.py with stdout=DEVNULL).  Checking stdout here is
    // the right guard: enable_raw_mode() operates on stdin and can succeed
    // even when stdout is /dev/null, but terminal.draw() calls
    // crossterm::terminal::size() which queries stdout and panics on failure.
    use std::io::IsTerminal;
    let tui_available =
        !turn_off_ui && std::io::stdout().is_terminal() && enable_raw_mode().is_ok();
    let mut stdout = io::stdout();
    if tui_available {
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    }
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut ui = UiState::new();
    ui.program = program_str;

    // ######################## Run Solver ######################################
    let start_time = time::Instant::now();

    let (verification_status, global_count) = run_parallel_solver(
        &constraint_info,
        base_abs_main_trace_data,
        public_vals,
        search_config,
        align_pc_to_program,
        final_check,
        known_solution,
        known_solution_area,
        &mut ui,
        &mut terminal,
        sleep_time,
        tui_available,
    );

    if tui_available {
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
    }

    Ok(VerificationResult {
        status: verification_status,
        num_total_trials: global_count,
        num_solutions: known_solution.len(),
        execution_time: start_time.elapsed(),
        area: *known_solution_area,
    })
}

pub fn mean_variance(durations: &[std::time::Duration]) -> (std::time::Duration, f64) {
    assert!(!durations.is_empty());
    let n = durations.len() as f64;
    let xs: Vec<f64> = durations.iter().map(|d| d.as_secs_f64()).collect();
    let mean = xs.iter().sum::<f64>() / n;
    let variance = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;

    (std::time::Duration::from_secs_f64(mean), variance)
}

#[derive(Default, Debug, Serialize)]
pub struct ResultReport {
    pub success_ratio: f64,
    pub exe_time_mean: f64,
    pub exe_time_variance: f64,
    pub n_trials_run: usize,
}

pub fn generate_report(results: &[VerificationResult]) -> ResultReport {
    //assert!(!results.is_empty());
    if results.is_empty() {
        return ResultReport::default();
    }

    let n = results.len() as f64;

    let success_ratio = results
        .iter()
        .map(|d| {
            if let VerificationStatus::Verified = d.status {
                1.0
            } else {
                0.0
            }
        })
        .sum::<f64>()
        / n;

    let xs: Vec<f64> = results
        .iter()
        .map(|d| d.execution_time.as_secs_f64())
        .collect();
    let exe_time_mean = xs.iter().sum::<f64>() / n;
    let exe_time_variance = xs.iter().map(|x| (x - exe_time_mean).powi(2)).sum::<f64>() / n;

    ResultReport {
        success_ratio,
        exe_time_mean,
        exe_time_variance,
        n_trials_run: results.len(),
    }
}

#[derive(Debug, Serialize)]
pub struct OutputBundle {
    pub config: SearchConfig,
    pub args: Args,
    pub report: ResultReport,
}

pub fn write_output(args: Args, config: SearchConfig, report: ResultReport) -> anyhow::Result<()> {
    let mut file = File::create(&args.ouptput_path)?;
    let bundle = OutputBundle {
        config,
        args,
        report,
    };
    let yaml = serde_yaml::to_string(&bundle)?;
    file.write_all(yaml.as_bytes())?;

    Ok(())
}
