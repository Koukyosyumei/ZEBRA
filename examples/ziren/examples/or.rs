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
use zkm_core_machine::memory::MemoryLocalChip;
use zkm_core_machine::AddSubChip;
use zkm_core_machine::BitwiseChip;
use zkm_core_machine::BranchChip;
use zkm_stark::MachineProver;

use latticevm::quick::quick_api;
use latticevm::solver::RangeType;
use latticevm::ui::generate_alu_final_checker;
use latticevm::ui::UiState;
use latticevm::utils::create_or_clear_dir;
use latticevm::{symbolic::AbstractTrace, symbolic::LatticeVMConstraints};

use latticevm_ziren::utils::{
    dummy_adjust_pc_program, dummy_program_counter_refine_fn, dummy_table_deriver,
    extract_constraints_and_range, generate_abstract_trace, get_program_str, indices_arr,
};

pub fn target_program(pc_start: u32, pc_base: u32) -> Program {
    let mut instructions = vec![Instruction::new(Opcode::OR, 1, 7, 9, true, true)];
    Program::new(instructions, pc_start, pc_base)
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;

    // ######################## Solver Parameters ###############################
    let max_iteration = 100000000;
    let min_row_id = 0;
    let max_row_id = 0;
    let num_extracted_rows = 1;
    let seed = 41;
    let aux_tg_fns: Vec<_> = vec![dummy_table_deriver];

    // ######################## Extract CPU Constraints ##########################
    let air = BitwiseChip::default();
    let air_name = "Bitwise";
    //let colmap = make_col_map();
    //println!("map: {:?}", colmap);

    let (tv_constraints, mut refinable_cols, mut range_types, general_lookup_info) =
        extract_constraints_and_range::<KoalaBear, BitwiseChip>(&air, NUM_BITWISE_COLS, prime);
    let final_check = generate_alu_final_checker(general_lookup_info.clone());
    refinable_cols.extend(&general_lookup_info.alu_output);

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
