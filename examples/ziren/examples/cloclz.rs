use std::collections::HashSet;
use std::fs;
use std::io;
use std::mem::transmute;

use itertools::Itertools;

use p3_koala_bear::KoalaBear;

use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::alu::CloClzCols;
use zkm_core_machine::alu::NUM_ADD_SUB_COLS;
use zkm_core_machine::alu::NUM_BITWISE_COLS;
use zkm_core_machine::alu::NUM_CLOCLZ_COLS;
use zkm_core_machine::control_flow::BranchColumns;
use zkm_core_machine::memory::MemoryLocalChip;
use zkm_core_machine::AddSubChip;
use zkm_core_machine::BitwiseChip;
use zkm_core_machine::BranchChip;
use zkm_core_machine::CloClzChip;
use zkm_stark::MachineProver;

use latticevm::interval::AbstractInterval;
use latticevm::quick::quick_api;
use latticevm::smt::expr_to_smt_bv;
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn, RangeType};
use latticevm::symbolic::LatticeVMSymbolicEntry;
use latticevm::symbolic::LatticeVMSymbolicExpr;
use latticevm::symbolic::LatticeVMSymbolicVal;
use latticevm::ui::save_repr_if_unique;
use latticevm::ui::UiState;
use latticevm::utils::trace_fmt_with_idxs;
use latticevm::utils::{create_or_clear_dir, indices_arr};
use latticevm::{symbolic::AbstractTrace, symbolic::LatticeVMConstraints};

use latticevm_ziren::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

// ############## Final Check Function ##############################
fn final_check(
    trace: &AbstractTrace,
    _num_trial: usize,
    _prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    let string_representation = format!(
        "input0: [{}], output: [{}]",
        trace_fmt_with_idxs(trace, 0, &[6, 7, 8, 9]),
        trace_fmt_with_idxs(trace, 0, &[2, 3, 4, 5]),
    );

    save_repr_if_unique(&string_representation, known_reprt, ui);
}

const fn make_col_map() -> CloClzCols<usize> {
    let indices_arr = indices_arr::<{ NUM_CLOCLZ_COLS }>();
    unsafe { transmute::<[usize; NUM_CLOCLZ_COLS], CloClzCols<usize>>(indices_arr) }
}

pub fn target_program(opcode: Opcode, pc_start: u32, pc_base: u32, x: u32, y: u32) -> Program {
    let instructions = vec![Instruction::new(opcode, 1, x, y, true, true)];
    Program::new(instructions, pc_start, pc_base)
}

pub fn get_opcode_addsub(target_opcode: &str) -> Opcode {
    match target_opcode {
        "CLO" => Opcode::CLO,
        "CLZ" => Opcode::CLZ,
        _ => panic!("unsupported instruction"),
    }
}

fn main() -> Result<(), io::Error> {
    let target_opcode = "CLZ";

    create_or_clear_dir("voutput")?;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;

    // ######################## Solver Parameters ###############################
    let max_iteration = 100000000000;
    let min_row_id = 0;
    let max_row_id = 0;
    let num_extracted_rows = 1;
    let seed = 41;

    // ######################## Extract CPU Constraints ##########################
    let air = CloClzChip::default();
    let air_name = "CloClz";
    let colmap = make_col_map();
    println!("{:?}", colmap);

    let (
        mut air_constraints,
        lookup_constraints,
        mut refinable_cols,
        mut range_types,
        general_lookup_info,
    ) = extract_constraints_and_range::<KoalaBear, CloClzChip>(&air, NUM_CLOCLZ_COLS, prime);
    refinable_cols.extend(&[2, 3, 4, 5]); // output

    let nc = vec![LatticeVMSymbolicExpr::Sub(
        Box::new(LatticeVMSymbolicExpr::Variable(LatticeVMSymbolicVal {
            entry: LatticeVMSymbolicEntry::Main { is_curr: true },
            index: 2,
        })),
        Box::new(LatticeVMSymbolicExpr::Constant(AbstractInterval::from_i64(
            32,
        ))),
    )];

    let constraints = LatticeVMConstraints {
        air_constraints,
        lookup_constraints,
        pv_pos_constraints: vec![],
        pv_neg_constraints: nc,
    };
    let minimum_num_taregt_cols = refinable_cols.len();
    for c in &constraints.air_constraints {
        println!("-- {}", c);
    }
    for c in &constraints.lookup_constraints {
        println!("** {}", c);
    }

    // ######################## Program Initialization ###########################
    let program = target_program(get_opcode_addsub(&target_opcode), 4, 4, 0, 0);
    let base_abs_main_trace_data =
        generate_abstract_trace(&program, air_name.to_string(), num_extracted_rows);

    let mut constants: Vec<(usize, usize, AbstractInterval)> = vec![];
    let mut neg_constants: Vec<(usize, usize, AbstractInterval)> = vec![];
    for j in 0..NUM_CLOCLZ_COLS {
        if !refinable_cols.contains(&j) {
            constants.push((0, j, base_abs_main_trace_data[0][j].clone()));
        }
    }
    for j in &vec![6, 7, 8, 9] {
        neg_constants.push((0, *j, base_abs_main_trace_data[0][*j].clone()));
    }

    let smt_str = expr_to_smt_bv(
        &constraints,
        &constants,
        &neg_constants,
        &range_types,
        1,
        NUM_CLOCLZ_COLS,
        0,
        prime,
    );
    println!("{}", smt_str);
    println!("rr: {:?}", range_types);
    println!("rr: {:?}", general_lookup_info);

    // ######################## Solve ############################################
    quick_api(
        get_program_str(&program),
        &constraints,
        &refinable_cols,
        &range_types,
        &vec![],
        &base_abs_main_trace_data,
        vec![],
        max_iteration,
        minimum_num_taregt_cols,
        min_row_id,
        max_row_id,
        program.instructions.len(),
        dummy_program_counter_refine_fn,
        dummy_adjust_pc_program,
        final_check,
        prime,
        seed,
    )
}
