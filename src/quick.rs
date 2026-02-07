use std::collections::HashMap;
use std::collections::HashSet;
use std::io;
use std::time;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::smt::expr_to_smt_bv;
use crate::solver::{run_parallel_solver, RangeType};
use crate::symbolic::add_blocking_constraint;
use crate::ui::UiState;
use crate::{interval::AbstractInterval, symbolic::AbstractTrace, symbolic::LatticeVMConstraints};

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

pub struct SearchConfig {
    pub max_expansions: usize,
    pub minimum_num_taregt_cols: usize,
    pub min_row_id: usize,
    pub max_row_id: usize,
    pub seed: u64,
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
) -> Result<(), io::Error>
where
    ProgramCounterRefinFn: Fn(&mut Vec<Vec<AbstractInterval>>, usize, usize, usize),
    FinalCheckFn: Fn(&AbstractTrace, usize, u32, &mut HashSet<String>, &mut UiState) + Clone,
    AlignPcToProgramFn: Fn(&mut AbstractTrace, u32) + Clone + Send + Sync + 'static,
{
    // ######################## Blocking Closures ################################
    for i in blocked_rows {
        add_blocking_constraint(
            &constraint_info.output_columns,
            &mut constraint_info.constraints,
            &base_abs_main_trace_data,
            *i,
        );
    }

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
    let _smt_str = expr_to_smt_bv(
        &constraint_info.constraints,
        &constants,
        &neg_constants,
        &constraint_info.range_types,
        search_config.max_row_id - search_config.min_row_id + 1,
        constraint_info.num_total_columns,
        constraint_info.num_pv_columns,
        constraint_info.prime,
    );

    // ######################## Solve ############################################
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
    )
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
) -> Result<(), io::Error>
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

    run_parallel_solver(
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
    );

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    eprintln!("Execution Time    : {:?}", start_time.elapsed());
    eprintln!("#Unique Solution  : {}", known_solution.len());

    Ok(())
}
