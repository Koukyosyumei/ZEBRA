use core::mem::transmute;
use itertools::Itertools;
use std::collections::HashSet;
use std::fs;
use std::io;

use p3_koala_bear::KoalaBear;

use pico_vm::chips::chips::alu::add_sub::columns::{AddSubCols, NUM_ADD_SUB_COLS};
use pico_vm::chips::chips::alu::add_sub::AddSubChip;
use pico_vm::compiler::riscv::program::Program;
use pico_vm::compiler::riscv::{instruction::Instruction, opcode::Opcode, register::Register};

use latticevm::quick::quick_api;
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn, RangeType};
use latticevm::ui::{save_repr_if_unique, UiState};
use latticevm::utils::{create_or_clear_dir, trace_fmt_with_idxs};
use latticevm::{symbolic::AbstractTrace, symbolic::LatticeVMConstraints};

use latticevm_pico::lookup::get_symbolic_lookup_constraints;
use latticevm_pico::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str, indices_arr,
};

fn canonical_repr_add(trace: &AbstractTrace) -> String {
    format!(
        "input0: [{}], input1: [{}], output: [{}]",
        trace_fmt_with_idxs(trace, &[7, 8, 9, 10]),
        trace_fmt_with_idxs(trace, &[11, 12, 13, 14]),
        trace_fmt_with_idxs(trace, &[0, 1, 2, 3]),
    )
}

fn canonical_repr_sub(trace: &AbstractTrace) -> String {
    format!(
        "input0: [{}], input1: [{}], output: [{}]",
        trace_fmt_with_idxs(trace, &[0, 1, 2, 3]),
        trace_fmt_with_idxs(trace, &[11, 12, 13, 14]),
        trace_fmt_with_idxs(trace, &[7, 8, 9, 10]),
    )
}

// ############## Final Check Function ##############################
fn final_check_add(
    trace: &AbstractTrace,
    _num_trial: usize,
    _prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    save_repr_if_unique(&canonical_repr_add(trace), known_reprt, ui);
}

fn final_check_sub(
    trace: &AbstractTrace,
    _num_trial: usize,
    _prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    save_repr_if_unique(&canonical_repr_sub(trace), known_reprt, ui);
}

const fn make_col_map() -> AddSubCols<usize> {
    let indices_arr = indices_arr::<{ NUM_ADD_SUB_COLS }>();
    unsafe { transmute::<[usize; NUM_ADD_SUB_COLS], AddSubCols<usize>>(indices_arr) }
}

pub fn target_program(opcode: Opcode, pc_start: u32, pc_base: u32, x: u32, y: u32) -> Program {
    let instructions = vec![Instruction::new(opcode, 1, x, y, true, true)];
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

    // ######################## Solver Parameters ###############################
    let max_iteration = 10000000000;
    let min_row_id = 0;
    let max_row_id = 0;
    let num_extracted_rows = 1;
    let seed = 41;

    // ######################## Extract CPU Constraints ##########################
    let air: AddSubChip<KoalaBear> = AddSubChip::default();
    let air_name = "AddSub";
    let _colmap = make_col_map();

    let (tv_constraints, mut refinable_cols, mut range_types, general_lookup_info) =
        extract_constraints_and_range::<KoalaBear, AddSubChip<KoalaBear>>(
            &air,
            NUM_ADD_SUB_COLS,
            prime,
        );
    refinable_cols.extend(&[0, 1, 2, 3]);

    let constraints = LatticeVMConstraints {
        air_constraints: tv_constraints.clone(),
        pv_pos_constraints: vec![],
        pv_neg_constraints: vec![],
    };
    let minimum_num_taregt_cols = refinable_cols.len();

    // ######################## Program Initialization ###########################
    let program = target_program(get_opcode_addsub(&target_opcode), 4, 4, 2, 3);
    let base_abs_main_trace_data =
        generate_abstract_trace(&program, air_name.to_string(), num_extracted_rows);

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
        if target_opcode == "ADD" {
            final_check_add
        } else if target_opcode == "SUB" {
            final_check_sub
        } else {
            panic!("unsupported instruction")
        },
        prime,
        seed,
    )
}
