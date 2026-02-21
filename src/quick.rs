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
use serde::Deserialize;

use crate::smt::expr_to_smt_bv;
use crate::solver::{run_parallel_solver, RangeType, VerificationStatus};
use crate::ui::UiState;
use crate::{
    constraint::{add_blocking_constraint, LatticeVMConstraints},
    interval::AbstractInterval,
    trace::AbstractTrace,
};

pub struct ProgramInfo {
    pub program_str: String,
    pub program_len: usize,
}

pub struct ConstraintInfo {
    pub constraints: LatticeVMConstraints,
    pub num_total_columns: usize,
    pub num_pv_columns: usize,
    pub output_columns: Vec<usize>,
    pub refinable_cols: Vec<usize>,
    pub range_types: HashMap<usize, RangeType>,
    pub prime: u32,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct SearchConfig {
    pub time_out_ms: u64,
    pub minimum_num_taregt_cols: usize,
    pub max_expansions: usize,
    pub min_row_id: usize,
    pub max_row_id: usize,
    pub seed: u64,
}

impl Default for SearchConfig {
    fn default() -> Self {
        SearchConfig {
            time_out_ms: 10000,
            minimum_num_taregt_cols: 0,
            max_expansions: 1000000000,
            min_row_id: 0,
            max_row_id: 0,
            seed: 41,
        }
    }
}

pub fn load_config(path: &std::path::Path) -> anyhow::Result<SearchConfig> {
    let text = fs::read_to_string(path)?;
    let cfg: SearchConfig = serde_yaml::from_str(&text)?;
    Ok(cfg)
}

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long)]
    pub config: PathBuf,
    #[arg(long, default_value = "output.yaml")]
    pub ouptput: PathBuf,
    #[arg(long, default_value = "bb")]
    pub method: String,
    #[arg(long, default_value = "none")]
    pub opcode_str: String,
}

#[derive(Debug)]
pub struct VerificationResult {
    pub status: VerificationStatus,
    pub num_solutions: usize,
    pub execution_time: std::time::Duration,
}

pub fn experiment_harness<ProgramCounterRefinFn, FinalCheckFn, AlignPcToProgramFn>(
    program_info: &ProgramInfo,
    constraint_info: &mut ConstraintInfo,
    search_config: &SearchConfig,
    base_abs_main_trace_data: &Vec<Vec<AbstractInterval>>,
    public_vals: Vec<AbstractInterval>,
    blocked_rows: &Vec<usize>,
    program_counter_refine_fn: ProgramCounterRefinFn,
    align_pc_to_program: AlignPcToProgramFn,
    final_check: FinalCheckFn,
    verification_method: &String,
) -> Result<(VerificationResult, usize), io::Error>
where
    ProgramCounterRefinFn: Fn(&mut Vec<Vec<AbstractInterval>>, usize, usize, usize),
    FinalCheckFn: Fn(&AbstractTrace, usize, u32, &mut HashSet<String>, &mut UiState) + Clone,
    AlignPcToProgramFn: Fn(&mut AbstractTrace, u32) + Clone + Send + Sync + 'static,
{
    let mut sleep_time = Duration::from_millis(0);
    let time_out = Duration::from_millis(search_config.time_out_ms);

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
            program_info.program_len,
            &constraint_info.constraints,
            &constraint_info.refinable_cols,
            &constraint_info.range_types,
            constraint_info.prime,
            &base_abs_main_trace_data,
            public_vals,
            search_config.max_expansions,
            search_config.minimum_num_taregt_cols,
            search_config.min_row_id,
            search_config.max_row_id,
            search_config.seed,
            program_counter_refine_fn,
            align_pc_to_program,
            final_check,
            &mut sleep_time,
            time_out,
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

        Ok((
            VerificationResult {
                status: VerificationStatus::Verified,
                num_solutions,
                execution_time: start_time.elapsed() - sleep_time,
            },
            0,
        ))
    }
}

pub fn quick_api<ProgramCounterRefinFn, FinalCheckFn, AlignPcToProgramFn>(
    program_str: String,
    program_len: usize,
    constraints: &LatticeVMConstraints,
    refinable_cols: &Vec<usize>,
    range_types: &HashMap<usize, RangeType>,
    prime: u32,
    base_abs_main_trace_data: &Vec<Vec<AbstractInterval>>,
    public_vals: Vec<AbstractInterval>,
    max_expansions: usize,
    minimum_num_taregt_cols: usize,
    min_row_id: usize,
    max_row_id: usize,
    seed: u64,
    program_counter_refine_fn: ProgramCounterRefinFn,
    align_pc_to_program: AlignPcToProgramFn,
    final_check: FinalCheckFn,
    sleep_time: &mut Duration,
    time_out: Duration,
) -> Result<(VerificationResult, usize), io::Error>
where
    ProgramCounterRefinFn: Fn(&mut Vec<Vec<AbstractInterval>>, usize, usize, usize),
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
        constraints,
        &refinable_cols,
        range_types,
        base_abs_main_trace_data,
        public_vals,
        max_expansions,
        minimum_num_taregt_cols,
        min_row_id,
        max_row_id,
        program_len,
        program_counter_refine_fn,
        align_pc_to_program,
        final_check,
        prime,
        seed,
        &mut known_solution,
        &mut ui,
        &mut terminal,
        sleep_time,
        time_out,
    );

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok((
        VerificationResult {
            status: verification_status,
            num_solutions: known_solution.len(),
            execution_time: start_time.elapsed(),
        },
        global_count,
    ))
}

pub fn mean_variance(durations: &[std::time::Duration]) -> (std::time::Duration, f64) {
    assert!(!durations.is_empty());
    let n = durations.len() as f64;
    let xs: Vec<f64> = durations.iter().map(|d| d.as_secs_f64()).collect();
    let mean = xs.iter().sum::<f64>() / n;
    let variance = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;

    (std::time::Duration::from_secs_f64(mean), variance)
}
