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

use p3_air::Air;
use p3_air::PairCol;
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
use zkm_core_machine::alu::LtCols;
use zkm_core_machine::alu::NUM_ADD_SUB_COLS;
use zkm_core_machine::alu::NUM_LT_COLS;
use zkm_core_machine::memory::MemoryLocalChip;
use zkm_core_machine::AddSubChip;
use zkm_core_machine::LtChip;
use zkm_core_machine::MulChip;
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

use latticevm::quick::quick_api;
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
use latticevm_ziren::p3_to_tv::{convert_p3_expr, convert_p3_virtual_pair_col};
use latticevm_ziren::pv_constraints::get_pv_constraints;
use latticevm_ziren::state::ziren_abstract_trace_to_abstract_state;
use latticevm_ziren::utils::get_program_str;

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
        trace.data[0][4],
        trace.data[0][5],
        trace.data[0][6],
        trace.data[0][7],
        trace.data[0][8],
        trace.data[0][9],
        trace.data[0][10],
        trace.data[0][11],
        trace.data[0][12],
        trace.data[0][13],
        trace.data[0][14],
        trace.data[0][15],
    );

    // 2130706432

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

pub const fn indices_arr<const N: usize>() -> [usize; N] {
    let mut indices_arr = [0; N];
    let mut i = 0;
    while i < N {
        indices_arr[i] = i;
        i += 1;
    }
    indices_arr
}

const fn make_col_map() -> LtCols<usize> {
    let indices_arr = indices_arr::<{ NUM_LT_COLS }>();
    unsafe { transmute::<[usize; NUM_LT_COLS], LtCols<usize>>(indices_arr) }
}

pub fn add_program(pc_start: u32, pc_base: u32) -> Program {
    let mut instructions = vec![Instruction::new(Opcode::SLT, 1, 2, 3, true, true)];
    /*
    instructions.extend(vec![
        Instruction::new(Opcode::ADD, 2, 0, SyscallCode::HALT as u32, false, true),
        Instruction::new(Opcode::ADD, 4, 0, 0, false, true),
        Instruction::new(Opcode::SYSCALL, 2, 4, 5, false, false),
    ]);*/

    Program::new(instructions, pc_start, pc_base)
}

pub fn extract_constraints_and_range<F, A>(
    air: &A,
    num_cols: usize,
    prime: u32,
) -> (
    Vec<LatticeVMSymbolicExpr>,
    Vec<usize>,
    HashMap<usize, RangeType>,
)
where
    F: p3_field::PrimeField32,
    A: Air<LookupBuilder<F>> + Air<SymbolicAirBuilder<F>>,
{
    let mut u8_cols = vec![];
    let mut multiplicities = HashSet::new();
    let mut lookup_symbolic_constraints = Vec::new();
    let mut received_vars_from_cpu = HashSet::new();

    let symbolic_constraints: Vec<SymbolicExpression<F>> =
        get_symbolic_constraints(air, 0, ZKM_PROOF_NUM_PV_ELTS);
    get_symbolic_lookup_constraints::<F, A>(
        air,
        0,
        ZKM_PROOF_NUM_PV_ELTS,
        &mut u8_cols,
        &mut multiplicities,
        &mut lookup_symbolic_constraints,
        &mut received_vars_from_cpu,
        prime,
    );

    let mut refinable_cols: Vec<usize> = (0..num_cols).collect();
    refinable_cols.retain(|c| !multiplicities.contains(c));
    refinable_cols.retain(|c| !received_vars_from_cpu.contains(c));

    let mut tv_constraints = symbolic_constraints
        .iter()
        .map(|sc| convert_p3_expr::<F>(&sc))
        .collect::<Vec<_>>();
    tv_constraints.extend(lookup_symbolic_constraints);

    let potential_boolean_vars = gather_boolean_variables(&tv_constraints, &multiplicities);
    let mut range_types: HashMap<usize, RangeType> = potential_boolean_vars
        .iter()
        .map(|k| (*k, RangeType::Bool))
        .collect();
    for c in &u8_cols {
        range_types.insert(*c, RangeType::U8);
    }

    (tv_constraints, refinable_cols, range_types)
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;
    // Columns reserved for program counters / instructions

    // ######################## Extract CPU Constraints ##########################
    let air = LtChip::default();
    let colmap = make_col_map();
    println!("a: {:?}", colmap.a);
    println!("b: {:?}", colmap.b);
    println!("c: {:?}", colmap.c);
    println!("{:?}", colmap.byte_equality_check);
    println!("{:?}", NUM_LT_COLS);

    let (tv_constraints, mut refinable_cols, range_types) =
        extract_constraints_and_range::<KoalaBear, LtChip>(&air, NUM_LT_COLS, prime);
    refinable_cols.extend(&[4]);

    // # Additional Public Value Verification
    //let (pv_pos_constraints, pv_neg_constraints) = get_pv_constraints();
    let pv_pos_constraints = vec![];
    let pv_neg_constraints = vec![];

    // # Gather Symbolic Constraints
    let constraints = LatticeVMConstraints {
        air_constraints: tv_constraints.clone(),
        pv_pos_constraints,
        pv_neg_constraints,
    };

    // ######################## Auxiliary ALU Constraints #######################
    let aux_tg_fns: Vec<_> = vec![dummy_table_deriver];

    // ######################## Solver Parameters ###############################
    // 16 17 19 20 21 25 26 28 29 30 31 32 33 4 5 6 7
    let max_iteration = 100000000;
    let minimum_num_taregt_cols = refinable_cols.len();
    let min_row_id = 0;
    let max_row_id = 0;
    let num_extracted_rows = 1;
    let seed = 41;

    // ######################## Program Initialization ###########################
    let program = add_program(4, 4);

    let (true_abstract_states, true_abstract_traces) = run_ziren_program(&program);
    let mut base_abs_main_trace_data = vec![];
    for st in &true_abstract_traces {
        if st.0 == "Lt" {
            base_abs_main_trace_data = st.1[..num_extracted_rows].to_vec();
        }
    }

    quick_api(
        get_program_str(&program),
        &constraints,
        &refinable_cols,
        &range_types,
        &vec![],
        &aux_tg_fns,
        &vec![],
        &base_abs_main_trace_data,
        vec![],
        max_iteration,
        minimum_num_taregt_cols,
        min_row_id,
        max_row_id,
        program.instructions.len(),
        program_counter_refine_fn,
        adjust_pc_program,
        final_check,
        prime,
        seed,
    )
}
