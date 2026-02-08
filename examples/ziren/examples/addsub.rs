use core::mem::transmute;
use std::collections::HashSet;
use std::io;

use p3_koala_bear::KoalaBear;

use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::alu::{AddSubCols, NUM_ADD_SUB_COLS};
use zkm_core_machine::AddSubChip;
use zkm_stark::MachineProver;

use latticevm::quick::{experiment_harness, Args, ConstraintInfo, ProgramInfo, SearchConfig};
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn};
use latticevm::ui::{save_repr_if_unique, UiState};
use latticevm::utils::{create_or_clear_dir, indices_arr, trace_fmt_with_idxs};
use latticevm::{symbolic::AbstractTrace, symbolic::LatticeVMConstraints};

use latticevm_ziren::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

fn cr_add(trace: &AbstractTrace) -> String {
    format!(
        "input0: [{}], input1: [{}], output: [{}]",
        trace_fmt_with_idxs(trace, 0, &[9, 10, 11, 12]),
        trace_fmt_with_idxs(trace, 0, &[13, 14, 15, 16]),
        trace_fmt_with_idxs(trace, 0, &[2, 3, 4, 5]),
    )
}

fn cr_sub(trace: &AbstractTrace) -> String {
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

pub fn get_opcode(opcode_str: &str) -> Opcode {
    match opcode_str {
        "ADD" => Opcode::ADD,
        "SUB" => Opcode::SUB,
        _ => panic!("unsupported instruction"),
    }
}

fn main() -> Result<(), io::Error> {
    let opcode_str = "ADD";

    create_or_clear_dir("voutput")?;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;

    // ######################## Canonicalization ##################################
    let cr = if opcode_str == "ADD" { cr_add } else { cr_sub };
    let final_check =
        |at: &AbstractTrace, _n: usize, _p: u32, kr: &mut HashSet<String>, ui: &mut UiState| {
            save_repr_if_unique(&cr(at), kr, ui);
        };

    // ######################## Extract CPU Constraints ##########################
    let air = AddSubChip::default();
    let air_name = "AddSub";
    let _colmap = make_col_map();

    let output_columns = if opcode_str == "ADD" {
        vec![2, 3, 4, 5]
    } else {
        vec![9, 10, 11, 12]
    };

    let (mut constraint_info, general_lookup_info) =
        extract_constraints_and_range::<KoalaBear, AddSubChip>(&air, NUM_ADD_SUB_COLS, prime);
    constraint_info
        .refinable_cols
        .extend(&output_columns.clone());
    constraint_info.output_columns = output_columns.clone();

    // ######################## Program Initialization ###########################
    let program = target_program(get_opcode(&opcode_str), 4, 4, 2, 3);

    // ######################## Set Info ##########################################
    let program_info = ProgramInfo {
        program_str: get_program_str(&program),
        program_len: program.instructions.len(),
    };
    let base_abs_main_trace_data = generate_abstract_trace(&program, air_name.to_string(), 1);
    let search_config = SearchConfig::new(constraint_info.refinable_cols.len());

    // ######################## Solve ############################################
    experiment_harness(
        &program_info,
        &mut constraint_info,
        &search_config,
        &base_abs_main_trace_data,
        vec![],
        &vec![], // vec![0],
        dummy_program_counter_refine_fn,
        dummy_adjust_pc_program,
        final_check,
    )
}
