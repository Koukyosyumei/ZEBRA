use clap::Parser;
use core::mem::transmute;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::mem::transmute;

use itertools::Itertools;

use p3_koala_bear::KoalaBear;

use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::memory::{
    columns::MemoryInstructionsColumns, columns::NUM_MEMORY_INSTRUCTIONS_COLUMNS,
};
use zkm_core_machine::MemoryInstructionsChip;
use zkm_stark::MachineProver;

use latticevm::quick::quick_api;
use latticevm::quick::{experiment_harness, ProgramInfo, SearchConfig};
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn};
use latticevm::ui::save_repr_if_unique;
use latticevm::ui::UiState;
use latticevm::utils::trace_fmt_with_idxs;
use latticevm::utils::{create_or_clear_dir, indices_arr};
use latticevm::{symbolic::AbstractTrace, symbolic::LatticeVMConstraints};

use latticevm_ziren::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

fn final_check(
    trace: &AbstractTrace,
    _num_trial: usize,
    _prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    let mut string_representation = String::new();
    for i in 0..trace.data.len() {
        let row_string_representation = format!(
            "clk: {}\nprev_value: [{}]\nop_b_access: [{}]\nop_c_access: [{}]\nop_a_access: {}\nmem_access: [{}]",
            trace.data[i][3],
            trace_fmt_with_idxs(trace, i, &[53, 54, 55, 56]),
            trace_fmt_with_idxs(trace, i, &[8, 9, 10, 11]),
            trace_fmt_with_idxs(trace, i, &[12, 13, 14, 15]),
            format!(
                "prev_value: [{}], value: [{}]",
                trace_fmt_with_idxs(trace, i, &[66, 67, 68, 69]),
                trace_fmt_with_idxs(trace, i, &[4, 5, 6, 7])
            ),
            trace_fmt_with_idxs(trace, i, &[57, 58, 59, 60]),
        );
        string_representation.push_str(&row_string_representation);
        string_representation.push_str("\n--------------\n");
    }

    save_repr_if_unique(&string_representation, known_reprt, ui);
}

const fn make_col_map() -> MemoryInstructionsColumns<usize> {
    let indices_arr = indices_arr::<{ NUM_MEMORY_INSTRUCTIONS_COLUMNS }>();
    unsafe {
        transmute::<[usize; NUM_MEMORY_INSTRUCTIONS_COLUMNS], MemoryInstructionsColumns<usize>>(
            indices_arr,
        )
    }
}

pub fn target_program_load(opcode: Opcode, pc_start: u32, pc_base: u32) -> Program {
    let instructions = vec![
        Instruction::new(Opcode::ADD, 29, 0, 0x12348765, false, true),
        Instruction::new(Opcode::SW, 29, 0, 0x27654320, false, true),
        Instruction::new(opcode, 29, 0, 0x27654320, false, true),
    ];
    Program::new(instructions, pc_start, pc_base)
}

pub fn target_program_store(opcode: Opcode, pc_start: u32, pc_base: u32) -> Program {
    let instructions = vec![
        Instruction::new(Opcode::ADD, 29, 0, 0x12348765, false, true),
        Instruction::new(opcode, 29, 0, 0x27654320, false, true),
    ];
    Program::new(instructions, pc_start, pc_base)
}

pub fn get_opcode(opcode_str: &str) -> (Opcode, bool) {
    match opcode_str {
        "LB" => (Opcode::LB, true),
        "LBU" => (Opcode::LBU, true),
        "LH" => (Opcode::LH, true),
        "LHU" => (Opcode::LHU, true),
        "LW" => (Opcode::LW, true),
        "SB" => (Opcode::SB, false),
        "SH" => (Opcode::SH, false),
        "SW" => (Opcode::SW, false),
        "SC" => (Opcode::SC, false),
        "SWL" => (Opcode::SWL, false),
        "SWR" => (Opcode::SWR, false),
        _ => panic!("unsupported instruction"),
    }
}

fn main() -> Result<(), io::Error> {
    let opcode_str = "SC";
    let (opcode, is_load) = get_opcode(opcode_str);

    create_or_clear_dir("voutput")?;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;

    // ######################## Extract CPU Constraints ##########################
    let air = MemoryInstructionsChip::default();
    let air_name = "MemoryInstrs";
    let _colmap = make_col_map();

    let (mut constraint_info, general_lookup_info) = extract_constraints_and_range::<
        KoalaBear,
        MemoryInstructionsChip,
    >(
        &air, NUM_MEMORY_INSTRUCTIONS_COLUMNS, prime
    );
    let mut semantic_inputs = vec![
        0, 1, 2, 3, 8, 9, 10, 11, 12, 13, 14, 15, 53, 54, 55, 56, 66, 67, 68, 69,
    ];
    if is_load {
        semantic_inputs.extend(&[57, 58, 59, 60]);
    } else {
        semantic_inputs.extend(&[4, 5, 6, 7]);
    }
    constraint_info
        .refinable_cols
        .retain(|c| !semantic_inputs.contains(c));

    // ######################## Program Initialization ###########################
    let program = if is_load {
        target_program_load(opcode, 4, 4)
    } else {
        target_program_store(opcode, 4, 4)
    };

    // ######################## Set Info ##########################################
    let program_info = ProgramInfo {
        program_str: get_program_str(&program),
        program_len: program.instructions.len(),
    };

    let num_extracted_rows = if is_load { 2 } else { 1 };
    let base_abs_main_trace_data =
        generate_abstract_trace(&program, air_name.to_string(), num_extracted_rows);

    let mut search_config = SearchConfig::new(constraint_info.refinable_cols.len());
    search_config.max_expansions = 1000000000000;
    search_config.min_row_id = if is_load { 1 } else { 0 };
    search_config.max_row_id = if is_load { 1 } else { 0 };
    //search_config.minimum_num_taregt_cols = 3;

    // ######################## Solve ############################################
    experiment_harness(
        &program_info,
        &mut constraint_info,
        &search_config,
        &base_abs_main_trace_data,
        vec![],
        &vec![],
        dummy_program_counter_refine_fn,
        dummy_adjust_pc_program,
        final_check,
    )
}
