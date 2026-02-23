use std::collections::{HashMap, HashSet};
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
    #[arg(long, default_value = "output.yaml")]
    pub ouptput_path: PathBuf,
    #[arg(long, default_value = "bb")]
    pub method: String,
    #[arg(long, default_value = "none")]
    pub opcode_str: String,
}

#[derive(Debug)]
pub struct VerificationResult {
    pub status: VerificationStatus,
    pub num_solutions: usize,
    pub num_total_trials: usize,
    pub execution_time: std::time::Duration,
}

pub fn experiment_harness<FinalCheckFn, AlignPcToProgramFn>(
    program_info: &ProgramInfo,
    constraint_info: &mut ConstraintInfo,
    search_config: &SearchConfig,
    base_abs_main_trace_data: &Vec<Vec<AbstractInterval>>,
    public_vals: Vec<AbstractInterval>,
    blocked_rows: &Vec<usize>,
    align_pc_to_program: AlignPcToProgramFn,
    final_check: FinalCheckFn,
    verification_method: &String,
) -> Result<VerificationResult, io::Error>
where
    FinalCheckFn: Fn(&AbstractTrace, usize, u32, &mut HashSet<String>, &mut UiState) + Clone,
    AlignPcToProgramFn: Fn(&mut AbstractTrace, u32) + Clone + Send + Sync + 'static,
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
        quick_api(
            program_info.program_str.clone(),
            constraint_info,
            &base_abs_main_trace_data,
            public_vals,
            &search_config,
            align_pc_to_program,
            final_check,
            &mut sleep_time,
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
        let smt_str = expr_to_smt_bv(
            &constraint_info.constraints,
            &constants,
            &neg_constants,
            &constraint_info.range_types,
            base_abs_main_trace_data.len(),
            constraint_info.num_total_columns,
            constraint_info.num_pv_columns,
            constraint_info.prime,
        );
        let smt_file_path = "voutput/smt_query.smt2";
        let mut file = File::create(smt_file_path).expect("Failed to create SMT file");
        file.write_all(smt_str.as_bytes())
            .expect("Failed to write SMT string to file");

        // ######################### Query SMT solver ###############################
        let start_time = time::Instant::now();
        let output = Command::new(verification_method)
            .arg(smt_file_path)
            .output()
            .expect("Failed to execute SMT solver");
        let stdout = String::from_utf8_lossy(&output.stdout);

        // ######################### Check the solutions ############################
        let num_solutions = if stdout.contains("unsat") {
            0
        } else if stdout.contains("sat") {
            1
        } else {
            panic!("error: {}", stdout)
        };

        Ok(VerificationResult {
            status: VerificationStatus::Verified,
            num_total_trials: 0,
            num_solutions,
            execution_time: start_time.elapsed() - sleep_time,
        })
    }
}

pub fn quick_api<FinalCheckFn, AlignPcToProgramFn>(
    program_str: String,
    constraint_info: &ConstraintInfo,
    base_abs_main_trace_data: &Vec<Vec<AbstractInterval>>,
    public_vals: Vec<AbstractInterval>,
    search_config: &SearchConfig,
    align_pc_to_program: AlignPcToProgramFn,
    final_check: FinalCheckFn,
    sleep_time: &mut Duration,
) -> Result<VerificationResult, io::Error>
where
    FinalCheckFn: Fn(&AbstractTrace, usize, u32, &mut HashSet<String>, &mut UiState) + Clone,
    AlignPcToProgramFn: Fn(&mut AbstractTrace, u32) + Clone + Send + Sync + 'static,
{
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut ui = UiState::new();
    ui.program = program_str;

    // ######################## Run Solver ######################################
    let mut known_solution = HashSet::<String>::new();
    let start_time = time::Instant::now();

    let (verification_status, global_count) = run_parallel_solver(
        &constraint_info,
        base_abs_main_trace_data,
        public_vals,
        search_config,
        align_pc_to_program,
        final_check,
        &mut known_solution,
        &mut ui,
        &mut terminal,
        sleep_time,
    );

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(VerificationResult {
        status: verification_status,
        num_total_trials: global_count,
        num_solutions: known_solution.len(),
        execution_time: start_time.elapsed(),
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

#[derive(Debug, Serialize)]
pub struct ResultReport {
    pub success_ratio: f64,
    pub exe_time_mean: f64,
    pub exe_time_variance: f64,
}

pub fn generate_report(results: &[VerificationResult]) -> ResultReport {
    assert!(!results.is_empty());
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
