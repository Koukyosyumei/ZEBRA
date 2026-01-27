use core::mem::transmute;
use std::io;


use p3_koala_bear::KoalaBear;

use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::alu::LtCols;
use zkm_core_machine::alu::NUM_LT_COLS;
use zkm_core_machine::LtChip;
use zkm_stark::MachineProver;

use latticevm::quick::quick_api;
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn};
use latticevm::ui::generate_alu_final_checker;
use latticevm::utils::create_or_clear_dir;
use latticevm::utils::indices_arr;
use latticevm::symbolic::LatticeVMConstraints;

use latticevm_ziren::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

const fn make_col_map() -> LtCols<usize> {
    let indices_arr = indices_arr::<{ NUM_LT_COLS }>();
    unsafe { transmute::<[usize; NUM_LT_COLS], LtCols<usize>>(indices_arr) }
}

pub fn target_program(pc_start: u32, pc_base: u32) -> Program {
    let instructions = vec![Instruction::new(Opcode::SLT, 1, 2, 3, true, true)];
    Program::new(instructions, pc_start, pc_base)
}

fn main() -> Result<(), io::Error> {
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
    let air = LtChip::default();
    let air_name = "Lt";
    let _colmap = make_col_map();

    let (tv_constraints, refinable_cols, range_types, general_lookup_info) =
        extract_constraints_and_range::<KoalaBear, LtChip>(&air, NUM_LT_COLS, prime);
    let final_check = generate_alu_final_checker(general_lookup_info.clone());

    println!("#### {}", tv_constraints[tv_constraints.len() - 7]);
    println!("{:?}", refinable_cols);
    println!("{:?}", range_types);

    for t in &tv_constraints {
        println!("{}", t);
    }

    let constraints = LatticeVMConstraints {
        air_constraints: tv_constraints,
        pv_pos_constraints: vec![],
        pv_neg_constraints: vec![],
    };
    let minimum_num_taregt_cols = 3; //refinable_cols.len();

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
