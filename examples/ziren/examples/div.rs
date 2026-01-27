use core::mem::transmute;
use itertools::Itertools;
use std::collections::HashSet;
use std::fs;
use std::io;

use p3_koala_bear::KoalaBear;

use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::alu::DivRemCols;
use zkm_core_machine::alu::NUM_DIVREM_COLS;
use zkm_core_machine::alu::{AddSubCols, NUM_ADD_SUB_COLS};
use zkm_core_machine::AddSubChip;
use zkm_core_machine::DivRemChip;
use zkm_stark::MachineProver;

use latticevm::interval::AbstractInterval;
use latticevm::quick::quick_api;
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn, RangeType};
use latticevm::ui::save_repr_if_unique;
use latticevm::ui::UiState;
use latticevm::utils::trace_fmt_with_idxs;
use latticevm::utils::{create_or_clear_dir, indices_arr};
use latticevm::{symbolic::AbstractTrace, symbolic::LatticeVMConstraints};

use latticevm_ziren::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

fn canonical_repr_div(trace: &AbstractTrace) -> String {
    format!(
        "input0: [{}], input1: [{}], output: [{}]",
        trace_fmt_with_idxs(trace, 0, &[2, 3, 4, 5]),
        trace_fmt_with_idxs(trace, 0, &[6, 7, 8, 9]),
        trace_fmt_with_idxs(trace, 0, &[10, 11, 12, 13]),
    )
}

fn canonical_repr_rem(trace: &AbstractTrace) -> String {
    format!(
        "input0: [{}], input1: [{}], output: [{}]",
        trace_fmt_with_idxs(trace, 0, &[2, 3, 4, 5]),
        trace_fmt_with_idxs(trace, 0, &[6, 7, 8, 9]),
        trace_fmt_with_idxs(trace, 0, &[14, 15, 16, 17]),
    )
}

// ############## Final Check Function ##############################
fn final_check_div(
    trace: &AbstractTrace,
    _num_trial: usize,
    _prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    save_repr_if_unique(&canonical_repr_div(trace), known_reprt, ui);
}

fn final_check_rem(
    trace: &AbstractTrace,
    _num_trial: usize,
    _prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    save_repr_if_unique(&canonical_repr_rem(trace), known_reprt, ui);
}

const fn make_col_map() -> DivRemCols<usize> {
    let indices_arr = indices_arr::<{ NUM_DIVREM_COLS }>();
    unsafe { transmute::<[usize; NUM_DIVREM_COLS], DivRemCols<usize>>(indices_arr) }
}

pub fn target_program(opcode: Opcode, pc_start: u32, pc_base: u32, x: u32, y: u32) -> Program {
    let instructions = vec![Instruction::new(opcode, 1, x, y, true, true)];
    Program::new(instructions, pc_start, pc_base)
}

pub fn get_opcode_addsub(target_opcode: &str) -> Opcode {
    match target_opcode {
        "DIV" => Opcode::DIV,
        "DIVU" => Opcode::DIVU,
        "MOD" => Opcode::MOD,
        "MODU" => Opcode::MODU,
        _ => panic!("unsupported instruction"),
    }
}

fn main() -> Result<(), io::Error> {
    let target_opcode = "MOD";

    create_or_clear_dir("voutput")?;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;

    // ######################## Solver Parameters ###############################
    let max_iteration = 100000;
    let min_row_id = 0;
    let max_row_id = 0;
    let num_extracted_rows = 1;
    let seed = 41;

    // ######################## Extract CPU Constraints ##########################
    let air = DivRemChip::default();
    let air_name = "DivRem";
    let _colmap = make_col_map();

    let (tv_constraints, mut refinable_cols, mut range_types, general_lookup_info) =
        extract_constraints_and_range::<KoalaBear, DivRemChip>(&air, NUM_DIVREM_COLS, prime);
    refinable_cols.extend(&[10, 11, 12, 13, 14, 15, 16, 17]); // output

    let constraints = LatticeVMConstraints {
        air_constraints: tv_constraints.clone(),
        pv_pos_constraints: vec![],
        pv_neg_constraints: vec![],
    };
    let minimum_num_taregt_cols = 3; // refinable_cols.len();

    // ######################## Program Initialization ###########################
    let program = target_program(get_opcode_addsub(&target_opcode), 4, 4, 13, 3);
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
        if target_opcode == "DIV" || target_opcode == "DIVU" {
            final_check_div
        } else if target_opcode == "MOD" || target_opcode == "MODU" {
            final_check_rem
        } else {
            panic!("unsupported instruction")
        },
        prime,
        seed,
    )
}
