use clap::Parser;
use core::mem::transmute;
use std::io;

use p3_koala_bear::KoalaBear;

use pico_vm::chips::chips::alu::divrem::columns::{DivRemCols, NUM_DIVREM_COLS};
use pico_vm::chips::chips::alu::divrem::DivRemChip;
use pico_vm::compiler::riscv::program::Program;
use pico_vm::compiler::riscv::{instruction::Instruction, opcode::Opcode};

use latticevm::quick::{experiment_harness, load_config, Args, ProgramInfo};
use latticevm::solver::RangeType;
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn};
use latticevm::ui::generate_alu_final_checker;
use latticevm::utils::create_or_clear_dir;
use latticevm::utils::indices_arr;

use latticevm_pico::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

const fn make_col_map() -> DivRemCols<usize> {
    let indices_arr = indices_arr::<{ NUM_DIVREM_COLS }>();
    unsafe { transmute::<[usize; NUM_DIVREM_COLS], DivRemCols<usize>>(indices_arr) }
}

pub fn target_program(opcode: Opcode, pc_start: u32, pc_base: u32, x: u32, y: u32) -> Program {
    let instructions = vec![Instruction::new(opcode, 1, x, y, true, true)];
    Program::new(instructions, pc_start, pc_base)
}

pub fn get_opcode_addsub(target_opcode: &str) -> Opcode {
    match target_opcode {
        "DIV" => Opcode::DIV,
        "DIVU" => Opcode::DIVU,
        "REM" => Opcode::REM,
        "REMU" => Opcode::REMU,
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
    let air: DivRemChip<KoalaBear> = DivRemChip::default();
    let air_name = "DivRem";
    let _colmap = make_col_map();

    let (mut constraint_info, general_lookup_info) = extract_constraints_and_range::<
        KoalaBear,
        DivRemChip<KoalaBear>,
    >(&air, NUM_DIVREM_COLS, prime);
    let final_check = generate_alu_final_checker(general_lookup_info.clone());
    for i in &general_lookup_info.alu_output {
        constraint_info.range_types.insert(*i, RangeType::U8);
    }
    constraint_info
        .refinable_cols
        .extend(&general_lookup_info.alu_output);
    constraint_info.output_columns = general_lookup_info.alu_output;

    // ######################## Program Initialization ###########################
    let program = target_program(get_opcode_addsub(&opcode_str), 4, 4, 13, 3);
    let base_abs_main_trace_data = generate_abstract_trace(&program, air_name.to_string(), 1);

    // ######################## Set Info ##########################################
    let program_info = ProgramInfo {
        program_str: get_program_str(&program),
        program_len: program.instructions.len(),
    };
    if search_config.minimum_num_taregt_cols == 0 {
        search_config.minimum_num_taregt_cols = 3; //constraint_info.refinable_cols.len();
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
