use core::mem::transmute;
use std::io;

use p3_koala_bear::KoalaBear;

use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::alu::ShiftLeftCols;
use zkm_core_machine::alu::NUM_SHIFT_LEFT_COLS;
use zkm_core_machine::ShiftLeft;
use zkm_stark::MachineProver;

use latticevm::interval::AbstractInterval;
use latticevm::quick::quick_api;
use latticevm::smt::expr_to_smt;
use latticevm::solver::RangeType;
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn};
use latticevm::symbolic::LatticeVMConstraints;
use latticevm::ui::generate_alu_final_checker;
use latticevm::utils::{create_or_clear_dir, indices_arr};

use latticevm_ziren::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

const fn make_col_map() -> ShiftLeftCols<usize> {
    let indices_arr = indices_arr::<{ NUM_SHIFT_LEFT_COLS }>();
    unsafe { transmute::<[usize; NUM_SHIFT_LEFT_COLS], ShiftLeftCols<usize>>(indices_arr) }
}

pub fn target_program(pc_start: u32, pc_base: u32) -> Program {
    let instructions = vec![Instruction::new(Opcode::SLL, 1, 2, 3, true, true)];
    Program::new(instructions, pc_start, pc_base)
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;

    // ######################## Solver Parameters ###############################
    let max_iteration = 1000000000;
    let min_row_id = 0;
    let max_row_id = 0;
    let num_extracted_rows = 1;
    let seed = 41;

    // ######################## Extract CPU Constraints ##########################
    let air = ShiftLeft::default();
    let air_name = "ShiftLeft";
    let _colmap = make_col_map();

    let (
        air_constraints,
        lookup_constraints,
        mut refinable_cols,
        mut range_types,
        general_lookup_info,
    ) = extract_constraints_and_range::<KoalaBear, ShiftLeft>(&air, NUM_SHIFT_LEFT_COLS, prime);
    let final_check = generate_alu_final_checker(general_lookup_info.clone());
    refinable_cols.extend(&general_lookup_info.alu_output);
    range_types.insert(2, RangeType::U8);
    range_types.insert(3, RangeType::U8);
    range_types.insert(4, RangeType::U8);
    range_types.insert(5, RangeType::U8);

    let constraints = LatticeVMConstraints {
        air_constraints,
        lookup_constraints,
        pv_pos_constraints: vec![],
        pv_neg_constraints: vec![],
    };
    let minimum_num_taregt_cols = refinable_cols.len(); // - 3;

    // ######################## Program Initialization ###########################
    let program = target_program(4, 4);
    let base_abs_main_trace_data =
        generate_abstract_trace(&program, air_name.to_string(), num_extracted_rows);

    let mut constants: Vec<(usize, usize, AbstractInterval)> = vec![];
    let mut neg_constants: Vec<(usize, usize, AbstractInterval)> = vec![];
    for j in 0..NUM_SHIFT_LEFT_COLS {
        if !refinable_cols.contains(&j) {
            constants.push((0, j, base_abs_main_trace_data[0][j].clone()));
        }
    }
    for j in &general_lookup_info.alu_output {
        neg_constants.push((0, *j, base_abs_main_trace_data[0][*j].clone()));
    }

    let smt_str = expr_to_smt(
        &constraints,
        &constants,
        &neg_constants,
        &range_types,
        1,
        NUM_SHIFT_LEFT_COLS,
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
