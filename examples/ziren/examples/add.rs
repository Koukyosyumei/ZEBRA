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

use p3_air::Air;
use p3_field::Field;
use p3_koala_bear::KoalaBear;
use p3_matrix::dense::RowMajorMatrix;
use p3_mersenne_31::Mersenne31;
use p3_uni_stark::SymbolicAirBuilder;
use p3_uni_stark::{get_symbolic_constraints, SymbolicExpression};

use zkm_core_executor::syscalls::SyscallCode;
use zkm_core_executor::ExecutionRecord;
use zkm_core_executor::Executor;
use zkm_core_executor::MipsAirId::MemoryLocal;
use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::alu::NUM_ADD_SUB_COLS;
use zkm_core_machine::memory::MemoryLocalChip;
use zkm_core_machine::AddSubChip;
use zkm_core_machine::{
    cpu::columns::{CPU_COL_MAP, NUM_CPU_COLS},
    CpuChip,
};
use zkm_stark::LookupBuilder;
use zkm_stark::LookupKind;
use zkm_stark::MachineAir;
use zkm_stark::MachineProver;
use zkm_stark::ZKMCoreOpts;
use zkm_stark::ZKM_PROOF_NUM_PV_ELTS;

use latticevm::smt::expr_to_smt;
use latticevm::solver::RangeType;
use latticevm::symbolic::eval_constraints;
use latticevm::symbolic::LatticeVMSymbolicEntry;
use latticevm::symbolic::LatticeVMSymbolicExpr;
use latticevm::symbolic::LatticeVMSymbolicVal;
use latticevm::ui::UiState;
use latticevm::utils::create_or_clear_dir;
use latticevm::{
    interval::AbstractInterval, solver::run_solver, symbolic::gather_boolean_variables,
    symbolic::AbstractTrace, symbolic::LatticeVMConstraints,
};

use latticevm_ziren::executor::run_ziren_program;
use latticevm_ziren::lookup::get_symbolic_lookup_constraints;
use latticevm_ziren::p3_to_tv::convert_p3_expr;
use latticevm_ziren::pv_constraints::get_pv_constraints;
use latticevm_ziren::state::ziren_abstract_trace_to_abstract_state;

fn program_counter_refine_fn(
    abs_main_trace_data: &mut Vec<Vec<AbstractInterval>>,
    program_len: usize,
    i: usize,
    j: usize,
) {
}

fn adjust_pc_program(main_trace: &mut AbstractTrace, prime: u32) {}

pub fn dummy_table_deriver(
    cpu_main_trace: &Vec<Vec<AbstractInterval>>,
    range_types: &HashMap<usize, RangeType>,
    prime: u32,
) -> Vec<Vec<AbstractInterval>> {
    let out = vec![];
    out
}

// ############## Final Check Function ##############################
fn final_check(
    trace: &AbstractTrace,
    num_trial: usize,
    prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    let string_representation = format!(
        "input0: [{}, {}, {}, {}], input1: [{}, {}, {}, {}], output: [{}, {}, {}, {}]",
        trace.data[0][9],
        trace.data[0][10],
        trace.data[0][11],
        trace.data[0][12],
        trace.data[0][13],
        trace.data[0][14],
        trace.data[0][15],
        trace.data[0][16],
        trace.data[0][2],
        trace.data[0][3],
        trace.data[0][4],
        trace.data[0][5],
    );

    if !known_reprt.contains(&string_representation) {
        known_reprt.insert(string_representation.clone());
        ui.recovered = string_representation;

        fs::write(
            format!("voutput/{}_states.txt", known_reprt.len()),
            ui.recovered.clone(),
        )
        .unwrap();
        fs::write(
            format!("voutput/{}_assignments.txt", known_reprt.len()),
            ui.logs.clone(),
        )
        .unwrap();
    }
}

pub fn target_program(pc_start: u32, pc_base: u32) -> Program {
    let mut instructions = vec![Instruction::new(Opcode::ADD, 1, 2, 3, true, true)];
    Program::new(instructions, pc_start, pc_base)
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;
    // Columns reserved for program counters / instructions

    // ######################## Extract CPU Constraints ##########################
    let air = AddSubChip::default();
    let air_name = "AddSub";

    let mut u8_cols = vec![];
    let mut multiplicities = HashSet::new();
    let mut lookup_symbolic_constraints = Vec::new();
    let mut received_vars_from_cpu = HashSet::new();

    let symbolic_constraints: Vec<SymbolicExpression<KoalaBear>> =
        get_symbolic_constraints(&air, 0, ZKM_PROOF_NUM_PV_ELTS);
    get_symbolic_lookup_constraints::<KoalaBear, AddSubChip>(
        &air,
        0,
        ZKM_PROOF_NUM_PV_ELTS,
        &mut u8_cols,
        &mut multiplicities,
        &mut lookup_symbolic_constraints,
        &mut received_vars_from_cpu,
    );

    let tv_constraints = symbolic_constraints
        .iter()
        .map(|sc| convert_p3_expr::<KoalaBear>(&sc))
        .collect::<Vec<_>>();
    let potential_boolean_vars = gather_boolean_variables(&tv_constraints, &multiplicities);

    // Columns available for refinement (excluding reserved program columns)
    let mut refinable_cols: Vec<usize> = (0..NUM_ADD_SUB_COLS).collect();
    refinable_cols.retain(|c| !multiplicities.contains(c));
    refinable_cols.retain(|c| !received_vars_from_cpu.contains(c));
    refinable_cols.extend(&[2, 3, 4, 5]); // output
    refinable_cols.extend(&[9, 13]); // input

    let mut range_types: HashMap<usize, RangeType> = potential_boolean_vars
        .iter()
        .map(|k| (*k, RangeType::Bool))
        .collect();
    for c in &u8_cols {
        range_types.insert(*c, RangeType::U8);
    }
    for c in &potential_boolean_vars {
        range_types.insert(*c, RangeType::Bool);
    }
    range_types.insert(9, RangeType::U4);
    range_types.insert(13, RangeType::U4);

    println!("{:?}", refinable_cols);
    println!("{:?}", range_types);

    // # Gather Symbolic Constraints
    let pv_pos_constraints = vec![];
    let pv_neg_constraints = vec![];
    let constraints = LatticeVMConstraints {
        air_constraints: tv_constraints.clone(),
        pv_pos_constraints,
        pv_neg_constraints,
    };

    // ######################## Auxiliary ALU Constraints #######################
    //let alu_constraints = get_alu_constraints();
    let aux_objs: Vec<_> = vec![];
    let aux_tg_fns: Vec<_> = vec![dummy_table_deriver];

    // ######################## Solver Parameters ###############################
    let max_iteration = 100000000;
    let minimum_num_taregt_cols = refinable_cols.len();
    let min_row_id = 0;
    let max_row_id = 0;
    let num_extracted_rows = 1;
    let seed = 41;

    // Public trace values (example: program start, memory base, initial step)
    let mut public_vals = vec![AbstractInterval::zero(); ZKM_PROOF_NUM_PV_ELTS];
    public_vals[40] = AbstractInterval::i4();
    public_vals[41] = AbstractInterval::bool();
    public_vals[44] = AbstractInterval::one();
    let refinment_target_indicies_pv: Vec<usize> = vec![];

    // ######################## Program Initialization ###########################
    let program = target_program(4, 4);
    let program_len = program.instructions.len();

    // Convert program to string for UI display
    let program_str = program
        .instructions
        .iter()
        .map(|inst| format!("{:?}\n", inst))
        .collect::<String>();

    let (true_abstract_states, true_abstract_traces) = run_ziren_program(&program);
    let mut base_abs_main_trace_data = vec![];
    for st in &true_abstract_traces {
        println!("{}", st.0);
        if st.0 == air_name {
            base_abs_main_trace_data = st.1[..num_extracted_rows].to_vec();
        }
    }
    println!("{:?}", base_abs_main_trace_data);

    // ######################## UI Initialization ################################

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
        &aux_objs,
        &aux_tg_fns,
        &refinment_target_indicies_pv,
        &base_abs_main_trace_data,
        public_vals,
        max_iteration,
        minimum_num_taregt_cols,
        min_row_id,
        max_row_id,
        program_len,
        program_counter_refine_fn,
        adjust_pc_program,
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
