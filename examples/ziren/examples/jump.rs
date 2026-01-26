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
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn, RangeType};
use latticevm::ui::save_repr_if_unique;
use latticevm::ui::{generate_alu_final_checker, UiState};
use latticevm::utils::trace_fmt_with_idxs;
use latticevm::utils::{create_or_clear_dir, indices_arr};
use latticevm::{symbolic::AbstractTrace, symbolic::LatticeVMConstraints};

use latticevm_ziren::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
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
        "pc: {}, next_pc: [{}], next_next_pc: [{}], op_a_value: [{}], op_b_value: [{}], op_c_value: [{}]",
        trace.data[0][0],
        trace_fmt_with_idxs(trace, 0, &[1, 2, 3, 4]),
        trace_fmt_with_idxs(trace, 0, &[19, 20, 21, 22]),
        trace_fmt_with_idxs(trace, 0, &[37, 38, 39, 40]),
        trace_fmt_with_idxs(trace, 0, &[41, 42, 43, 44]),
        trace_fmt_with_idxs(trace, 0, &[45, 46, 47, 48]),
    );
    save_repr_if_unique(&string_representation, known_reprt, ui);
}

const fn make_col_map() -> JumpColumns<usize> {
    let indices_arr = indices_arr::<{ NUM_JUMP_COLS }>();
    unsafe { transmute::<[usize; NUM_JUMP_COLS], JumpColumns<usize>>(indices_arr) }
}

pub fn target_program(pc_start: u32, pc_base: u32, y: u32) -> Program {
    let instructions = vec![
        Instruction::new(Opcode::ADD, 1, 0, y, false, true),
        Instruction::new(Opcode::Jump, 0, 1, 0, false, true),
    ];
    Program::new(instructions, pc_start, pc_base)
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;

    // ######################## Solver Parameters ###############################
    let max_iteration = 10000000000000000;
    let min_row_id = 0;
    let max_row_id = 0;
    let num_extracted_rows = 1;
    let seed = 41;

    // ######################## Extract CPU Constraints ##########################
    let air = JumpChip::default();
    let air_name = "Jump";
    let _colmap = make_col_map();

    let (tv_constraints, mut refinable_cols, mut range_types, general_lookup_info) =
        extract_constraints_and_range::<KoalaBear, JumpChip>(&air, NUM_JUMP_COLS, prime);
    refinable_cols.extend(&[19, 20, 21, 22, 37, 38, 39, 40]); // output

    for t in &tv_constraints {
        println!("{}", t);
    }

    let constraints = LatticeVMConstraints {
        air_constraints: tv_constraints[..6].to_vec(),
        pv_pos_constraints: vec![],
        pv_neg_constraints: vec![],
    };
    let minimum_num_taregt_cols = refinable_cols.len();

    // ######################## Program Initialization ###########################
    let program = target_program(4, 4, 32);
    let base_abs_main_trace_data =
        generate_abstract_trace(&program, air_name.to_string(), num_extracted_rows);

    for row in &base_abs_main_trace_data {
        for v in row {
            print!("{}, ", v);
        }
        println!("");
    }
    println!("{:?}", refinable_cols);
    println!("{}", tv_constraints[5]);

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
