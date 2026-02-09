use clap::Parser;
use core::mem::transmute;
use std::io;

use p3_koala_bear::KoalaBear;

use pico_vm::chips::chips::alu::mul::columns::{MulCols, NUM_MUL_COLS};
use pico_vm::chips::chips::alu::mul::MulChip;
use pico_vm::compiler::riscv::program::Program;
use pico_vm::compiler::riscv::{instruction::Instruction, opcode::Opcode};

use latticevm::interval::AbstractInterval;
use latticevm::quick::{experiment_harness, load_config, Args, ProgramInfo};
use latticevm::smt::expr_to_smt_bv;
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn};
use latticevm::symbolic::LatticeVMConstraints;
use latticevm::ui::generate_alu_final_checker;
use latticevm::utils::create_or_clear_dir;
use latticevm::utils::indices_arr;

use latticevm_pico::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

const fn make_col_map() -> MulCols<usize> {
    let indices_arr = indices_arr::<{ NUM_MUL_COLS }>();
    unsafe { transmute::<[usize; NUM_MUL_COLS], MulCols<usize>>(indices_arr) }
}

pub fn target_program(opcode: Opcode, pc_start: u32, pc_base: u32, x: u32, y: u32) -> Program {
    let instructions = vec![Instruction::new(opcode, 1, x, y, true, true)];
    Program::new(instructions, pc_start, pc_base)
}

pub fn get_opcode_addsub(target_opcode: &str) -> Opcode {
    match target_opcode {
        "MUL" => Opcode::MUL,
        "MULH" => Opcode::MULH,
        "MULHU" => Opcode::MULHU,
        "MULHSU" => Opcode::MULHSU,
        _ => panic!("unsupported instruction"),
    }
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    let args = Args::parse();
    let opcode_str = args.opcode_str;
    let mut search_config = load_config(&args.config).unwrap();

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;

    // ######################## Extract CPU Constraints ##########################
    let air: MulChip<KoalaBear> = MulChip::default();
    let air_name = "Mul";
    let _colmap = make_col_map();

    let (mut constraint_info, general_lookup_info) =
        extract_constraints_and_range::<KoalaBear, MulChip<KoalaBear>>(&air, NUM_MUL_COLS, prime);
    let final_check = generate_alu_final_checker(general_lookup_info.clone());
    constraint_info
        .refinable_cols
        .extend(&general_lookup_info.alu_output);
    constraint_info.output_columns = general_lookup_info.alu_output;

    // ######################## Program Initialization ###########################
    let program = target_program(get_opcode_addsub(&opcode_str), 4, 4, 2, 3);
    let base_abs_main_trace_data = generate_abstract_trace(&program, air_name.to_string(), 1);

    // ######################## Set Info ##########################################
    let program_info = ProgramInfo {
        program_str: get_program_str(&program),
        program_len: program.instructions.len(),
    };
    if search_config.minimum_num_taregt_cols == 0 {
        search_config.minimum_num_taregt_cols = constraint_info.refinable_cols.len();
    }

    // ######################## Solve ############################################
    let result = experiment_harness(
        &program_info,
        &mut constraint_info,
        &search_config,
        &base_abs_main_trace_data,
        vec![],
        &vec![], // vec![0],
        dummy_program_counter_refine_fn,
        dummy_adjust_pc_program,
        final_check,
        &args.method,
    );
    println!("{:?}", result);

    Ok(())
}
