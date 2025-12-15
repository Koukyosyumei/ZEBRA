use core::mem::{size_of, transmute};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::rc::Rc;
use std::time;
use std::{io, thread, time::Duration};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use itertools::Itertools;
use rand::{rngs::StdRng, SeedableRng};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

use crate::smt::expr_to_smt;
use crate::solver::AbsConstraintObj;
use crate::solver::RangeType;
use crate::symbolic::eval_constraints;
use crate::symbolic::LatticeVMSymbolicEntry;
use crate::symbolic::LatticeVMSymbolicExpr;
use crate::symbolic::LatticeVMSymbolicVal;
use crate::ui::UiState;
use crate::utils::create_or_clear_dir;
use crate::{
    interval::AbstractInterval, solver::run_solver, symbolic::gather_boolean_variables,
    symbolic::AbstractTrace, symbolic::LatticeVMConstraints,
};

pub fn quick_api<ProgramCounterRefinFn, FinalCheckFn, AuxTableGenFn, AlignPcToProgramFn>(
    program_str: String,
    constraints: &LatticeVMConstraints,
    refinable_cols: &Vec<usize>,
    range_types: &HashMap<usize, RangeType>,
    aux_constraints_objs: &Vec<AbsConstraintObj>,
    aux_table_gen_fns: &Vec<AuxTableGenFn>,
    refinment_target_indicies_pv: &Vec<usize>,
    base_abs_main_trace_data: &Vec<Vec<AbstractInterval>>,
    public_vals: Vec<AbstractInterval>,
    max_expansions: usize,
    minimum_num_taregt_cols: usize,
    min_row_id: usize,
    max_row_id: usize,
    program_len: usize,
    program_counter_refine_fn: ProgramCounterRefinFn,
    align_pc_to_program: AlignPcToProgramFn,
    final_check: FinalCheckFn,
    prime: u32,
    seed: u64,
) -> Result<(), io::Error>
where
    ProgramCounterRefinFn: Fn(&mut Vec<Vec<AbstractInterval>>, usize, usize, usize),
    FinalCheckFn: Fn(&AbstractTrace, usize, u32, &mut HashSet<String>, &mut UiState),
    AuxTableGenFn: Fn(
        &Vec<Vec<AbstractInterval>>,
        &HashMap<usize, RangeType>,
        u32,
    ) -> Vec<Vec<AbstractInterval>>,
    AlignPcToProgramFn: Fn(&mut AbstractTrace, u32),
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
    let mut logs = Vec::new();
    let start_time = time::Instant::now();
    run_solver(
        &constraints,
        &refinable_cols,
        &range_types,
        &aux_constraints_objs,
        &aux_table_gen_fns,
        &refinment_target_indicies_pv,
        &base_abs_main_trace_data,
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
        &mut logs,
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
