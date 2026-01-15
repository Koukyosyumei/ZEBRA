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
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn, RangeType};
use latticevm::ui::UiState;
use latticevm::utils::create_or_clear_dir;
use latticevm::{symbolic::AbstractTrace, symbolic::LatticeVMConstraints};

use latticevm_ziren::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str, indices_arr,
};

// ############## Final Check Function ##############################
/*
colmap: MemoryInstructionsColumns { pc: 0, next_pc: 1, shard: 2, clk: 3,
op_a_value: Word([4, 5, 6, 7]),
op_b_value: Word([8, 9, 10, 11]),
op_c_value: Word([12, 13, 14, 15]), is_lb: 16, is_lbu: 17, is_lh: 18, is_lhu: 19, is_lw: 20, is_lwl: 21, is_lwr: 22, is_ll: 23, is_sb: 24, is_sh: 25, is_sw: 26, is_swl: 27, is_swr: 28, is_sc: 29,
addr_word: Word([30, 31, 32, 33]), addr_aligned: 34, addr_ls_two_bits: 35, ls_bits_is_one: 36, ls_bits_is_two: 37, ls_bits_is_three: 38,
addr_word_range_checker: KoalaBearWordRangeChecker { most_sig_byte_decomp: [39, 40, 41, 42, 43, 44, 45, 46],
                                                    and_most_sig_byte_decomp_0_to_2: 47,
                                                    and_most_sig_byte_decomp_0_to_3: 48,
                                                    and_most_sig_byte_decomp_0_to_4: 49,
                                                    and_most_sig_byte_decomp_0_to_5: 50, and_most_sig_byte_decomp_0_to_6: 51, and_most_sig_byte_decomp_0_to_7: 52 },
memory_access: MemoryReadWriteCols { prev_value: Word([53, 54, 55, 56]), access: MemoryAccessCols { value: Word([57, 58, 59, 60]),
prev_shard: 61, prev_clk: 62, compare_clk: 63, diff_16bit_limb: 64, diff_8bit_limb: 65 } },
prev_a_val: Word([66, 67, 68, 69]),
unsigned_mem_val: Word([70, 71, 72, 73]),
most_sig_bit: 74, most_sig_byte: 75, mem_value_is_neg: 76, most_sig_bytes_zero: IsZeroOperation { inverse: 77, result: 78 } }
*/

// [30, 31, 32, 33, 34, 35, 36, 37, 38, 61, 62, 63, 64, 65, 70, 71, 72, 73, 74, 75, 76, 78]

fn final_check(
    trace: &AbstractTrace,
    num_trial: usize,
    prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    let prev_state = format!(
        "prev_value: [{}, {}, {}, {}]",
        trace.data[1][53], trace.data[1][54], trace.data[1][55], trace.data[1][56],
    );
    let op_b_access = format!(
        "value: [{}, {}, {}, {}]",
        trace.data[1][8], trace.data[1][9], trace.data[1][10], trace.data[1][11],
    );
    let op_c_access = format!(
        "value: [{}, {}, {}, {}]",
        trace.data[1][12], trace.data[1][13], trace.data[1][14], trace.data[1][15],
    );
    let op_a_access = format!(
        "prev_value: [{}, {}, {}, {}], value: [{}, {}, {}, {}]",
        trace.data[1][66],
        trace.data[1][67],
        trace.data[1][68],
        trace.data[1][69],
        trace.data[1][4],
        trace.data[1][5],
        trace.data[1][6],
        trace.data[1][7],
    );
    let string_representation = format!(
        "prev state: {}\nop_b_access: {}\nop_c_access: {}\nop_a_access: {}\nmem_access: [{}, {}, {}, {}]",
        prev_state,
        op_b_access,
        op_c_access,
        op_a_access,
        trace.data[1][57],
        trace.data[1][58],
        trace.data[1][59],
        trace.data[1][60],
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

const fn make_col_map() -> MemoryInstructionsColumns<usize> {
    let indices_arr = indices_arr::<{ NUM_MEMORY_INSTRUCTIONS_COLUMNS }>();
    unsafe {
        transmute::<[usize; NUM_MEMORY_INSTRUCTIONS_COLUMNS], MemoryInstructionsColumns<usize>>(
            indices_arr,
        )
    }
}

pub fn target_program(pc_start: u32, pc_base: u32) -> Program {
    let instructions = vec![
        Instruction::new(Opcode::ADD, 29, 0, 0x12348765, false, true),
        Instruction::new(Opcode::SW, 29, 0, 0x27654320, false, true),
        Instruction::new(Opcode::LB, 28, 0, 0x27654320, false, true),
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
    let air = MemoryInstructionsChip::default();
    let air_name = "MemoryInstrs";
    let colmap = make_col_map();
    println!("colmap: {:?}", colmap);

    let (tv_constraints, mut refinable_cols, mut range_types, general_lookup_info) =
        extract_constraints_and_range::<KoalaBear, MemoryInstructionsChip>(
            &air,
            NUM_MEMORY_INSTRUCTIONS_COLUMNS,
            prime,
        );
    let mut semantic_inputs = vec![
        0, 1, 2, 3, 8, 9, 10, 11, 12, 13, 14, 15, 53, 54, 55, 56, 66, 67, 68, 69,
    ];
    semantic_inputs.extend(&[57, 58, 59, 60]);
    refinable_cols.retain(|c| !semantic_inputs.contains(c));
    //refinable_cols.extend(&[83, 84, 85, 92, 93, 94]);

    /*
    refinable_cols.extend(&[2, 3, 4, 5]); // output
                                          //refinable_cols.extend(&[10, 14]); // input
                                          //range_types.insert(6, RangeType::U4);
                                          //range_types.insert(10, RangeType::U4);
                                          //range_types.insert(14, RangeType::U4); */
    for t in &tv_constraints {
        println!("---- {}", t);
    }

    println!("{:?}", refinable_cols);
    println!("{:?}", range_types);

    let constraints = LatticeVMConstraints {
        air_constraints: tv_constraints,
        pv_pos_constraints: vec![],
        pv_neg_constraints: vec![],
    };
    let minimum_num_taregt_cols = 1; //refinable_cols.len();
    println!("{}", minimum_num_taregt_cols);

    // ######################## Program Initialization ###########################
    let program = target_program(4, 4);
    let base_abs_main_trace_data =
        generate_abstract_trace(&program, air_name.to_string(), num_extracted_rows);

    println!("");
    for row in &base_abs_main_trace_data {
        for v in row {
            print!("{}, ", v);
        }
        println!("\n----------------------------");
    }

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
