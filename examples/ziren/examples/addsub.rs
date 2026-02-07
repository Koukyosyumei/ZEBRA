use core::mem::transmute;
use std::collections::HashSet;
use std::io;

use p3_koala_bear::KoalaBear;

use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::alu::{AddSubCols, NUM_ADD_SUB_COLS};
use zkm_core_machine::AddSubChip;
use zkm_stark::MachineProver;

use latticevm::interval::AbstractInterval;
use latticevm::quick::quick_api;
use latticevm::smt::expr_to_smt_bv;
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn};
use latticevm::symbolic::add_blocking_constraint;
use latticevm::symbolic::LatticeVMSymbolicEntry;
use latticevm::symbolic::LatticeVMSymbolicExpr;
use latticevm::symbolic::LatticeVMSymbolicVal;
use latticevm::ui::{save_repr_if_unique, UiState};
use latticevm::utils::{create_or_clear_dir, indices_arr, trace_fmt_with_idxs};
use latticevm::{symbolic::AbstractTrace, symbolic::LatticeVMConstraints};

use latticevm_ziren::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

fn canonical_repr_add(trace: &AbstractTrace) -> String {
    format!(
        "input0: [{}], input1: [{}], output: [{}]",
        trace_fmt_with_idxs(trace, 0, &[9, 10, 11, 12]),
        trace_fmt_with_idxs(trace, 0, &[13, 14, 15, 16]),
        trace_fmt_with_idxs(trace, 0, &[2, 3, 4, 5]),
    )
}

fn canonical_repr_sub(trace: &AbstractTrace) -> String {
    format!(
        "input0: [{}], input1: [{}], output: [{}]",
        trace_fmt_with_idxs(trace, 0, &[2, 3, 4, 5]),
        trace_fmt_with_idxs(trace, 0, &[13, 14, 15, 16]),
        trace_fmt_with_idxs(trace, 0, &[9, 10, 11, 12]),
    )
}

const fn make_col_map() -> AddSubCols<usize> {
    let indices_arr = indices_arr::<{ NUM_ADD_SUB_COLS }>();
    unsafe { transmute::<[usize; NUM_ADD_SUB_COLS], AddSubCols<usize>>(indices_arr) }
}

pub fn target_program(opcode: Opcode, pc_start: u32, pc_base: u32, x: u32, y: u32) -> Program {
    let instructions = vec![Instruction::new(opcode, 1, 2, 3, true, true)];
    Program::new(instructions, pc_start, pc_base)
}

pub fn get_opcode_addsub(target_opcode: &str) -> Opcode {
    match target_opcode {
        "ADD" => Opcode::ADD,
        "SUB" => Opcode::SUB,
        _ => panic!("unsupported instruction"),
    }
}

fn main() -> Result<(), io::Error> {
    let target_opcode = "ADD";

    create_or_clear_dir("voutput")?;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;

    // ######################## Canonicalization ##################################
    let canonical_repr = if target_opcode == "ADD" {
        canonical_repr_add
    } else if target_opcode == "SUB" {
        canonical_repr_sub
    } else {
        panic!("unsupported instruction")
    };
    let final_check = |trace: &AbstractTrace,
                       _num_trial: usize,
                       _prime: u32,
                       known_reprt: &mut HashSet<String>,
                       ui: &mut UiState| {
        save_repr_if_unique(&canonical_repr(trace), known_reprt, ui);
    };

    // ######################## Solver Parameters ###############################
    let max_iteration = 100000000;
    let min_row_id = 0;
    let max_row_id = 0;
    let num_extracted_rows = 1;
    let seed = 41;

    // ######################## Extract CPU Constraints ##########################
    let air = AddSubChip::default();
    let air_name = "AddSub";
    let _colmap = make_col_map();

    let output_columns = if target_opcode == "ADD" {
        vec![2, 3, 4, 5]
    } else if target_opcode == "SUB" {
        vec![9, 10, 11, 12]
    } else {
        panic!("unsupported instruction")
    };

    let (air_constraints, lookup_constraints, mut refinable_cols, range_types, general_lookup_info) =
        extract_constraints_and_range::<KoalaBear, AddSubChip>(&air, NUM_ADD_SUB_COLS, prime);
    refinable_cols.extend(&output_columns.clone());
    let minimum_num_taregt_cols = refinable_cols.len();

    let mut constraints = LatticeVMConstraints {
        air_constraints,
        lookup_constraints,
        pv_pos_constraints: vec![],
        pv_neg_constraints: vec![],
        blocking_constraints: vec![],
    };

    // ######################## Program Initialization ###########################
    let program = target_program(get_opcode_addsub(&target_opcode), 4, 4, 2, 3);
    let base_abs_main_trace_data =
        generate_abstract_trace(&program, air_name.to_string(), num_extracted_rows);

    // ######################## Blocking Closures ################################
    add_blocking_constraint(
        &output_columns,
        &mut constraints,
        &base_abs_main_trace_data,
        0,
    );

    // ######################## Generating SMT Formula ##########################
    let constants: Vec<_> = (0..NUM_ADD_SUB_COLS)
        .filter(|j| !refinable_cols.contains(j))
        .map(|j| (0, j, base_abs_main_trace_data[0][j].clone()))
        .collect();
    let neg_constants: Vec<_> = output_columns
        .iter()
        .map(|&j| (0, j, base_abs_main_trace_data[0][j].clone()))
        .collect();
    let smt_str = expr_to_smt_bv(
        &constraints,
        &constants,
        &neg_constants,
        &range_types,
        max_row_id - min_row_id + 1,
        NUM_ADD_SUB_COLS,
        0,
        prime,
    );

    // ######################## Solve ############################################
    quick_api(
        get_program_str(&program),
        &constraints,
        &refinable_cols,
        &range_types,
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
