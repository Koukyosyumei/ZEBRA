use core::mem::transmute;
use itertools::Itertools;
use std::collections::HashSet;
use std::fs;
use std::io;

use p3_koala_bear::KoalaBear;

use pico_vm::chips::chips::alu::sll::SLLChip;
use pico_vm::chips::chips::alu::sll::{ShiftLeftCols, NUM_SLL_COLS};
use pico_vm::compiler::riscv::program::Program;
use pico_vm::compiler::riscv::{instruction::Instruction, opcode::Opcode, register::Register};

use latticevm::interval::AbstractInterval;
use latticevm::quick::quick_api;
use latticevm::smt::expr_to_smt_bv;
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn, RangeType};
use latticevm::ui::{generate_alu_final_checker, UiState};
use latticevm::utils::create_or_clear_dir;
use latticevm::utils::indices_arr;
use latticevm::{symbolic::AbstractTrace, symbolic::LatticeVMConstraints};

use latticevm_pico::lookup::get_symbolic_lookup_constraints;
use latticevm_pico::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

const fn make_col_map() -> ShiftLeftCols<usize> {
    let indices_arr = indices_arr::<{ NUM_SLL_COLS }>();
    unsafe { transmute::<[usize; NUM_SLL_COLS], ShiftLeftCols<usize>>(indices_arr) }
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
    let max_iteration = 100000000;
    let min_row_id = 0;
    let max_row_id = 0;
    let num_extracted_rows = 1;
    let seed = 41;

    // ######################## Extract CPU Constraints ##########################
    let air: SLLChip<KoalaBear> = SLLChip::default();
    let air_name = "ShiftLeft";
    let _colmap = make_col_map();

    let (
        air_constraints,
        lookup_constraints,
        mut refinable_cols,
        mut range_types,
        general_lookup_info,
    ) = extract_constraints_and_range::<KoalaBear, SLLChip<KoalaBear>>(&air, NUM_SLL_COLS, prime);
    let final_check = generate_alu_final_checker(general_lookup_info.clone());
    refinable_cols.extend(&general_lookup_info.alu_output);

    let constraints = LatticeVMConstraints {
        air_constraints,
        lookup_constraints,
        pv_pos_constraints: vec![],
        pv_neg_constraints: vec![],
    };
    let minimum_num_taregt_cols = refinable_cols.len();

    // ######################## Program Initialization ###########################
    let program = target_program(4, 4, 1, 2);
    let base_abs_main_trace_data =
        generate_abstract_trace(&program, air_name.to_string(), num_extracted_rows);

    let mut constants: Vec<(usize, usize, AbstractInterval)> = vec![];
    let mut neg_constants: Vec<(usize, usize, AbstractInterval)> = vec![];
    for j in 0..NUM_SLL_COLS {
        if !refinable_cols.contains(&j) {
            constants.push((0, j, base_abs_main_trace_data[0][j].clone()));
        }
    }
    for j in general_lookup_info.alu_output {
        neg_constants.push((0, j, base_abs_main_trace_data[0][j].clone()));
    }
    let smt_str = expr_to_smt_bv(
        &constraints,
        &constants,
        &neg_constants,
        &range_types,
        1,
        NUM_SLL_COLS,
        0,
        prime,
    );
    println!("{}", smt_str);
    println!("rr: {:?}", range_types);

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
