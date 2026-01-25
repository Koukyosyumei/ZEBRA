use core::mem::transmute;
use itertools::Itertools;
use std::collections::HashSet;
use std::fs;
use std::io;

use p3_koala_bear::KoalaBear;

use pico_vm::chips::chips::riscv_memory::read_write::{
    columns::{MemoryChipCols, MemoryChipValueCols, NUM_MEMORY_CHIP_COLS},
    MemoryReadWriteChip,
};
use pico_vm::compiler::riscv::program::Program;
use pico_vm::compiler::riscv::{instruction::Instruction, opcode::Opcode, register::Register};

use latticevm::quick::quick_api;
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn, RangeType};
use latticevm::ui::save_repr_if_unique;
use latticevm::ui::UiState;
use latticevm::utils::{create_or_clear_dir, trace_fmt_with_idxs};
use latticevm::{symbolic::AbstractTrace, symbolic::LatticeVMConstraints};

use latticevm_pico::lookup::get_symbolic_lookup_constraints;
use latticevm_pico::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str, indices_arr,
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
        "prev_value: [{}]\nop_b_access: [{}]\nop_c_access: [{}]\nop_a_access: {}\nmem_access: [{}]",
        trace_fmt_with_idxs(trace, &[24, 25, 26, 27]),
        trace_fmt_with_idxs(trace, &[77, 78, 79, 80]),
        trace_fmt_with_idxs(trace, &[86, 87, 88, 89]),
        format!(
            "prev_value: [{}], value: [{}]",
            trace_fmt_with_idxs(trace, &[64, 65, 66, 67]),
            trace_fmt_with_idxs(trace, &[68, 69, 70, 71])
        ),
        trace_fmt_with_idxs(trace, &[28, 29, 30, 31]),
    );

    save_repr_if_unique(&string_representation, known_reprt, ui);
}

const fn make_col_map() -> MemoryChipCols<usize> {
    let indices_arr = indices_arr::<{ NUM_MEMORY_CHIP_COLS }>();
    unsafe { transmute::<[usize; NUM_MEMORY_CHIP_COLS], MemoryChipCols<usize>>(indices_arr) }
}

pub fn target_program(pc_start: u32, pc_base: u32) -> Program {
    let instructions = vec![
        Instruction::new(Opcode::ADD, 29, 0, 0x12348765, false, true),
        Instruction::new(Opcode::SW, 29, 0, 0x27654320, false, true),
        Instruction::new(Opcode::LB, 29, 0, 0x27654320, false, true),
    ];
    Program::new(instructions, pc_start, pc_base)
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;

    // ######################## Solver Parameters ###############################
    let max_iteration = 10000;
    let min_row_id = 1;
    let max_row_id = 1;
    let num_extracted_rows = 2;
    let seed = 41;

    // ######################## Extract CPU Constraints ##########################
    let air: MemoryReadWriteChip<KoalaBear> = MemoryReadWriteChip::default();
    let air_name = "MemoryReadWrite";
    let _colmap = make_col_map();

    let (tv_constraints, mut refinable_cols, mut range_types, general_lookup_info) =
        extract_constraints_and_range::<KoalaBear, MemoryReadWriteChip<KoalaBear>>(
            &air,
            NUM_MEMORY_CHIP_COLS,
            prime,
        );
    let mut semantic_inputs = vec![
        0, 1, 24, 25, 26, 27, 32, 33, 55, 64, 65, 66, 67, 72, 73, 77, 78, 79, 80, 81, 82, 86, 87,
        88, 89, 90, 91,
    ];
    semantic_inputs.extend(&[28, 29, 30, 31]);
    refinable_cols.retain(|c| !semantic_inputs.contains(c));
    refinable_cols.extend(&[83, 84, 85, 92, 93, 94]);

    let constraints = LatticeVMConstraints {
        air_constraints: tv_constraints.clone(),
        pv_pos_constraints: vec![],
        pv_neg_constraints: vec![],
    };
    let minimum_num_taregt_cols = 1;

    // ######################## Program Initialization ###########################
    let program = target_program(4, 4);
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
        final_check,
        prime,
        seed,
    )
}

/*
opcode = LB
addr   = 0x1000
offset = 0
prev_value = 0x807F01FF

[1, 32, 34, 35, 52, 54, 60, 63]

MemoryChipCols { values: [MemoryChipValueCols {
    chunk: 0, clk: 1,
    addr_word: Word([2, 3, 4, 5]),
    addr_word_range_checker: FieldWordRangeChecker { most_sig_byte_decomp: [6, 7, 8, 9, 10, 11, 12, 13],
    upper_all_one: IsZeroGadget { inverse: 14, result: 15 } },
    addr_aligned: 16, aa_least_sig_byte_decomp: [17, 18, 19, 20, 21, 22],
    addr_offset: 23,
    memory_access: MemoryReadWriteCols {
        prev_value: Word([24, 25, 26, 27]),
        access: MemoryAccessCols { value: Word([28, 29, 30, 31]),
        prev_chunk: 32,
        prev_clk: 33,
        compare_clk: 34,
        diff_16bit_limb: 35, diff_8bit_limb: 36 } },
        offset_is_one: 37,
        offset_is_two: 38,
        offset_is_three: 39,
        most_sig_byte_decomp: [40, 41, 42, 43, 44, 45, 46, 47],
        unsigned_mem_val: Word([48, 49, 50, 51]),
        mem_value_is_pos_not_x0: 52,
        mem_value_is_neg_not_x0: 53,
        instruction: MemoryInstructionCols {
            opcode: 54, op_a_0: 55, is_lb: 56, is_lbu: 57, is_lh: 58, is_lhu: 59, is_lw: 60, is_sb: 61, is_sh: 62, is_sw: 63,
            op_a_access: MemoryReadWriteCols {
                prev_value: Word([64, 65, 66, 67]),
                 access: MemoryAccessCols { value: Word([68, 69, 70, 71]), prev_chunk: 72, prev_clk: 73, compare_clk: 74, diff_16bit_limb: 75, diff_8bit_limb: 76 }
                 },
            op_b_access: MemoryReadCols { access: MemoryAccessCols { value: Word([77, 78, 79, 80]),
                                            prev_chunk: 81, prev_clk: 82, compare_clk: 83, diff_16bit_limb: 84, diff_8bit_limb: 85 }
                 },
            op_c_access: MemoryReadCols { access: MemoryAccessCols { value: Word([86, 87, 88, 89]),
                                            prev_chunk: 90, prev_clk: 91, compare_clk: 92, diff_16bit_limb: 93, diff_8bit_limb: 94 }
            } } }] }
*/
