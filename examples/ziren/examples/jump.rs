use std::collections::HashSet;
use std::fs;
use std::io;
use std::mem::transmute;

use itertools::Itertools;

use p3_koala_bear::KoalaBear;

use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::alu::NUM_ADD_SUB_COLS;
use zkm_core_machine::alu::NUM_BITWISE_COLS;
use zkm_core_machine::control_flow::BranchColumns;
use zkm_core_machine::control_flow::JumpColumns;
use zkm_core_machine::control_flow::NUM_JUMP_COLS;
use zkm_core_machine::memory::MemoryLocalChip;
use zkm_core_machine::misc::MovCondCols;
use zkm_core_machine::misc::NUM_MOV_COND_COLS;
use zkm_core_machine::AddSubChip;
use zkm_core_machine::BitwiseChip;
use zkm_core_machine::BranchChip;
use zkm_core_machine::JumpChip;
use zkm_core_machine::MovCondChip;
use zkm_stark::MachineProver;

use latticevm::quick::quick_api;
use latticevm::solver::RangeType;
use latticevm::ui::UiState;
use latticevm::utils::create_or_clear_dir;
use latticevm::{symbolic::AbstractTrace, symbolic::LatticeVMConstraints};

use latticevm_ziren::utils::{
    dummy_adjust_pc_program, dummy_program_counter_refine_fn, dummy_table_deriver,
    extract_constraints_and_range, generate_abstract_trace, get_program_str, indices_arr,
};

// ############## Final Check Function ##############################
fn final_check(
    trace: &AbstractTrace,
    num_trial: usize,
    prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    let string_representation = format!(
        "next_pc: [{}, {}, {}, {}], next_next_pc: [{}, {}, {}, {}], op_a_value: [{}, {}, {}, {}], op_b_value: [{}, {}, {}, {}], op_c_value: [{}, {}, {}, {}]",
        trace.data[0][1],
        trace.data[0][2],
        trace.data[0][3],
        trace.data[0][4],
        trace.data[0][19],
        trace.data[0][20],
        trace.data[0][21],
        trace.data[0][22],
        trace.data[0][37],
        trace.data[0][38],
        trace.data[0][39],
        trace.data[0][40],
        trace.data[0][41],
        trace.data[0][42],
        trace.data[0][43],
        trace.data[0][44],
        trace.data[0][45],
        trace.data[0][46],
        trace.data[0][47],
        trace.data[0][48],
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

const fn make_col_map() -> JumpColumns<usize> {
    let indices_arr = indices_arr::<{ NUM_JUMP_COLS }>();
    unsafe { transmute::<[usize; NUM_JUMP_COLS], JumpColumns<usize>>(indices_arr) }
}

pub fn target_program(pc_start: u32, pc_base: u32) -> Program {
    let instructions = vec![
        Instruction::new(Opcode::ADD, 11, 0, 100, false, true),
        Instruction::new(Opcode::Jump, 0, 11, 0, false, true),
    ];
    Program::new(instructions, pc_start, pc_base)
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;

    // ######################## Solver Parameters ###############################
    let max_iteration = 10000000;
    let min_row_id = 0;
    let max_row_id = 0;
    let num_extracted_rows = 1;
    let seed = 41;
    let aux_tg_fns: Vec<_> = vec![dummy_table_deriver];

    // ######################## Extract CPU Constraints ##########################
    let air = JumpChip::default();
    let air_name = "Jump";
    let colmap = make_col_map();
    println!("next_pc: {:?}", colmap.next_pc);
    println!("next_next_pc: {:?}", colmap.next_next_pc);
    println!("op_a_value: {:?}", colmap.op_a_value);
    println!("op_b_value: {:?}", colmap.op_b_value);
    println!("op_c_value: {:?}", colmap.op_c_value);

    let (tv_constraints, mut refinable_cols, mut range_types) =
        extract_constraints_and_range::<KoalaBear, JumpChip>(&air, NUM_JUMP_COLS, prime);
    refinable_cols.extend(&[19, 20, 21, 22]); // output
                                              //refinable_cols.extend(&[10, 14]); // input
                                              //range_types.insert(6, RangeType::U4);
                                              //range_types.insert(10, RangeType::U4);
                                              //range_types.insert(14, RangeType::U4);
    for t in &tv_constraints {
        println!("{}", t);
    }

    println!("{:?}", refinable_cols);
    println!("{:?}", range_types);

    let constraints = LatticeVMConstraints {
        air_constraints: tv_constraints.clone(),
        pv_pos_constraints: vec![],
        pv_neg_constraints: vec![],
    };
    let minimum_num_taregt_cols = refinable_cols.len();

    // ######################## Program Initialization ###########################
    let program = target_program(4, 4);
    let base_abs_main_trace_data =
        generate_abstract_trace(&program, air_name.to_string(), num_extracted_rows);
    //println!("{:?}", base_abs_main_trace_data);

    // ######################## Solve ############################################
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
        dummy_program_counter_refine_fn,
        dummy_adjust_pc_program,
        final_check,
        prime,
        seed,
    )
}

/*
(curr[49] * (curr[49] - 1))
(curr[50] * (curr[50] - 1))
(curr[51] * (curr[51] - 1))
(((curr[49] + curr[50]) + curr[51]) * (((curr[49] + curr[50]) + curr[51]) - 1))
(((curr[49] + curr[50]) + curr[51]) * (((((1 * curr[37]) + (256 * curr[38])) + (65536 * curr[39])) + (16777216 * curr[40])) - (((((1 * curr[1]) + (256 * curr[2])) + (65536 * curr[3])) + (16777216 * curr[4])) + 4)))
(((curr[49] + curr[50]) + curr[51]) * (curr[52] * (curr[52] - 1)))
(((curr[49] + curr[50]) + curr[51]) * (curr[53] * (curr[53] - 1)))
(((curr[49] + curr[50]) + curr[51]) * (curr[54] * (curr[54] - 1)))
(((curr[49] + curr[50]) + curr[51]) * (curr[55] * (curr[55] - 1)))
(((curr[49] + curr[50]) + curr[51]) * (curr[56] * (curr[56] - 1)))
(((curr[49] + curr[50]) + curr[51]) * (curr[57] * (curr[57] - 1)))
(((curr[49] + curr[50]) + curr[51]) * (curr[58] * (curr[58] - 1)))
(((curr[49] + curr[50]) + curr[51]) * (curr[59] * (curr[59] - 1)))
(((curr[49] + curr[50]) + curr[51]) * (((((((((0 + (1 * curr[52])) + (2 * curr[53])) + (4 * curr[54])) + (8 * curr[55])) + (16 * curr[56])) + (32 * curr[57])) + (64 * curr[58])) + (128 * curr[59])) - curr[40]))
(((curr[49] + curr[50]) + curr[51]) * curr[59])
(((curr[49] + curr[50]) + curr[51]) * (curr[60] - (curr[52] * curr[53])))
(((curr[49] + curr[50]) + curr[51]) * (curr[61] - (curr[60] * curr[54])))
(((curr[49] + curr[50]) + curr[51]) * (curr[62] - (curr[61] * curr[55])))
(((curr[49] + curr[50]) + curr[51]) * (curr[63] - (curr[62] * curr[56])))
(((curr[49] + curr[50]) + curr[51]) * (curr[64] - (curr[63] * curr[57])))
(((curr[49] + curr[50]) + curr[51]) * (curr[65] - (curr[64] * curr[58])))
(((curr[49] + curr[50]) + curr[51]) * (curr[65] * ((curr[37] + curr[38]) + curr[39])))
(((curr[49] + curr[50]) + curr[51]) * (curr[5] * (curr[5] - 1)))
(((curr[49] + curr[50]) + curr[51]) * (curr[6] * (curr[6] - 1)))
(((curr[49] + curr[50]) + curr[51]) * (curr[7] * (curr[7] - 1)))
(((curr[49] + curr[50]) + curr[51]) * (curr[8] * (curr[8] - 1)))
(((curr[49] + curr[50]) + curr[51]) * (curr[9] * (curr[9] - 1)))
(((curr[49] + curr[50]) + curr[51]) * (curr[10] * (curr[10] - 1)))
(((curr[49] + curr[50]) + curr[51]) * (curr[11] * (curr[11] - 1)))
(((curr[49] + curr[50]) + curr[51]) * (curr[12] * (curr[12] - 1)))
(((curr[49] + curr[50]) + curr[51]) * (((((((((0 + (1 * curr[5])) + (2 * curr[6])) + (4 * curr[7])) + (8 * curr[8])) + (16 * curr[9])) + (32 * curr[10])) + (64 * curr[11])) + (128 * curr[12])) - curr[4]))
(((curr[49] + curr[50]) + curr[51]) * curr[12])
(((curr[49] + curr[50]) + curr[51]) * (curr[13] - (curr[5] * curr[6])))
(((curr[49] + curr[50]) + curr[51]) * (curr[14] - (curr[13] * curr[7])))
(((curr[49] + curr[50]) + curr[51]) * (curr[15] - (curr[14] * curr[8])))
(((curr[49] + curr[50]) + curr[51]) * (curr[16] - (curr[15] * curr[9])))
(((curr[49] + curr[50]) + curr[51]) * (curr[17] - (curr[16] * curr[10])))
(((curr[49] + curr[50]) + curr[51]) * (curr[18] - (curr[17] * curr[11])))
(((curr[49] + curr[50]) + curr[51]) * (curr[18] * ((curr[1] + curr[2]) + curr[3])))
(((curr[49] + curr[50]) + curr[51]) * (curr[23] * (curr[23] - 1)))
(((curr[49] + curr[50]) + curr[51]) * (curr[24] * (curr[24] - 1)))
(((curr[49] + curr[50]) + curr[51]) * (curr[25] * (curr[25] - 1)))
(((curr[49] + curr[50]) + curr[51]) * (curr[26] * (curr[26] - 1)))
(((curr[49] + curr[50]) + curr[51]) * (curr[27] * (curr[27] - 1)))
(((curr[49] + curr[50]) + curr[51]) * (curr[28] * (curr[28] - 1)))
(((curr[49] + curr[50]) + curr[51]) * (curr[29] * (curr[29] - 1)))
(((curr[49] + curr[50]) + curr[51]) * (curr[30] * (curr[30] - 1)))
(((curr[49] + curr[50]) + curr[51]) * (((((((((0 + (1 * curr[23])) + (2 * curr[24])) + (4 * curr[25])) + (8 * curr[26])) + (16 * curr[27])) + (32 * curr[28])) + (64 * curr[29])) + (128 * curr[30])) - curr[22]))
(((curr[49] + curr[50]) + curr[51]) * curr[30])
(((curr[49] + curr[50]) + curr[51]) * (curr[31] - (curr[23] * curr[24])))
(((curr[49] + curr[50]) + curr[51]) * (curr[32] - (curr[31] * curr[25])))
(((curr[49] + curr[50]) + curr[51]) * (curr[33] - (curr[32] * curr[26])))
(((curr[49] + curr[50]) + curr[51]) * (curr[34] - (curr[33] * curr[27])))
(((curr[49] + curr[50]) + curr[51]) * (curr[35] - (curr[34] * curr[28])))
(((curr[49] + curr[50]) + curr[51]) * (curr[36] - (curr[35] * curr[29])))
(((curr[49] + curr[50]) + curr[51]) * (curr[36] * ((curr[19] + curr[20]) + curr[21])))
((curr[49] + curr[50]) * (curr[19] - curr[41]))
((curr[49] + curr[50]) * (curr[20] - curr[42]))
((curr[49] + curr[50]) * (curr[21] - curr[43]))
((curr[49] + curr[50]) * (curr[22] - curr[44]))
*/
