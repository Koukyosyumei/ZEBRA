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
use latticevm::quick::{experiment_harness, ProgramInfo, SearchConfig};
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

pub fn target_program(pc_start: u32, pc_base: u32, x: u32, y: u32) -> Program {
    let instructions = vec![Instruction::new(Opcode::SLL, 1, x, y, true, true)];
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

    let (mut constraint_info, general_lookup_info) =
        extract_constraints_and_range::<KoalaBear, ShiftLeft>(&air, NUM_SHIFT_LEFT_COLS, prime);
    let final_check = generate_alu_final_checker(general_lookup_info.clone());

    constraint_info
        .refinable_cols
        .extend(&general_lookup_info.alu_output.clone());
    constraint_info.output_columns = general_lookup_info.alu_output.clone();

    // ######################## Program Initialization ###########################
    let program = target_program(4, 4, 2, 3);

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
