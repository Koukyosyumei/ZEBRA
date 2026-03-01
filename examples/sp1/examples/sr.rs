use clap::Parser;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::io;

use p3_baby_bear::BabyBear;

use sp1_core_executor::{Instruction, Opcode, Program};
use sp1_core_machine::alu::NUM_SHIFT_RIGHT_COLS;
use sp1_core_machine::riscv::ShiftRightChip;

use latticevm::canonicalizer::generate_alu_final_checker;
use latticevm::quick::{
    experiment_harness, generate_report, load_config, write_output, Args, ProgramInfo,
};
use latticevm::solver::nop_post_process;
use latticevm::utils::create_or_clear_dir;

use latticevm_sp1::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

pub fn target_program(opcode: Opcode, pc_start: u32, pc_base: u32, x: u32, y: u32) -> Program {
    let instructions = vec![Instruction::new(opcode, 1, x, y, true, true)];
    Program::new(instructions, pc_start, pc_base)
}

pub fn get_opcode(opcode_str: &str) -> Opcode {
    match opcode_str {
        "SRL" => Opcode::SRL,
        "SRA" => Opcode::SRA,
        _ => panic!("unsupported instruction"),
    }
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    let args = Args::parse();
    let opcode_str = args.opcode_str.clone();
    let mut search_config = load_config(&args.config).unwrap();

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(27) + 1;

    // ######################## Extract CPU Constraints ##########################
    let air = ShiftRightChip::default();
    let air_name = "ShiftRight";

    let (mut constraint_info, general_lookup_info) = extract_constraints_and_range::<
        BabyBear,
        ShiftRightChip,
    >(&air, NUM_SHIFT_RIGHT_COLS, prime);
    let final_check = generate_alu_final_checker(general_lookup_info.clone());
    println!("{:?}", general_lookup_info);

    constraint_info
        .refinable_cols
        .extend(&general_lookup_info.op_a.clone());
    constraint_info.output_columns = general_lookup_info.op_a.clone();
    if search_config.minimum_num_taregt_cols == 0 {
        search_config.minimum_num_taregt_cols = constraint_info.refinable_cols.len();
    }

    let mut rng = StdRng::seed_from_u64(search_config.seed);
    let mut ds = vec![];
    for _ in 0..args.num_trial {
        let x: u32 = rng.random();
        let y: u32 = rng.random();

        // ######################## Program Initialization ###########################
        let program = target_program(get_opcode(&opcode_str), 4, 4, x, y);
        let base_abs_main_trace_data = generate_abstract_trace(&program, air_name.to_string(), 1);

        // ######################## Set Info ##########################################
        let program_info = ProgramInfo {
            program_str: get_program_str(&program),
            program_len: program.instructions.len(),
        };

        // ######################## Solve ############################################
        let result = experiment_harness(
            &program_info,
            &mut constraint_info,
            &search_config,
            &base_abs_main_trace_data,
            vec![],
            &vec![], // vec![0],
            nop_post_process,
            &final_check,
            &args.method,
        );
        println!("({} {}), {:?}", x, y, result);
        ds.push(result.unwrap());
    }
    let report = generate_report(&ds);
    println!("{:?}", report);
    let _ = write_output(args, search_config, report);

    Ok(())
}
