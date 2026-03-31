use clap::Parser;
use core::mem::transmute;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashSet;
use std::io;

use p3_baby_bear::BabyBear;

use sp1_core_executor::{Instruction, Opcode, Program};
use sp1_core_machine::memory::MemoryInstructionsChip;
use sp1_core_machine::memory::{
    columns::MemoryInstructionsColumns, columns::NUM_MEMORY_INSTRUCTIONS_COLUMNS,
};

use zebra::canonicalizer::generate_memory_op_final_checker;
use zebra::quick::{
    experiment_harness, generate_report, load_config, write_output, Args, ProgramInfo,
};
use zebra::solver::nop_post_process;
use zebra::utils::{create_or_clear_dir, indices_arr};

use zebra_sp1::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

const fn make_col_map() -> MemoryInstructionsColumns<usize> {
    let indices_arr = indices_arr::<{ NUM_MEMORY_INSTRUCTIONS_COLUMNS }>();
    unsafe {
        transmute::<[usize; NUM_MEMORY_INSTRUCTIONS_COLUMNS], MemoryInstructionsColumns<usize>>(
            indices_arr,
        )
    }
}

pub fn target_program_load(
    opcode: Opcode,
    pc_start: u32,
    pc_base: u32,
    r1: u8,
    _r2: u8,
    x: u32,
    y: u32,
    _z: u32,
) -> Program {
    let instructions = vec![
        Instruction::new(Opcode::ADD, r1, 0, x, false, true),
        Instruction::new(Opcode::SW, r1, 0, y, false, true),
        Instruction::new(opcode, r1, 0, y, false, true),
    ];
    Program::new(instructions, pc_start, pc_base)
}

pub fn target_program_store(
    opcode: Opcode,
    pc_start: u32,
    pc_base: u32,
    r1: u8,
    r2: u8,
    x: u32,
    y: u32,
    z: u32,
) -> Program {
    let instructions = vec![
        Instruction::new(Opcode::ADD, r1, 0, x, false, true),
        Instruction::new(Opcode::ADD, r2, 0, y, false, true),
        Instruction::new(opcode, r1, r2 as u32, z, false, true),
    ];
    Program::new(instructions, pc_start, pc_base)
}

pub fn get_opcode(opcode_str: &str) -> (Opcode, bool) {
    match opcode_str {
        "LB" => (Opcode::LB, true),
        "LH" => (Opcode::LH, true),
        "LW" => (Opcode::LW, true),
        "LBU" => (Opcode::LBU, true),
        "LHU" => (Opcode::LHU, true),
        "SB" => (Opcode::SB, false),
        "SH" => (Opcode::SH, false),
        "SW" => (Opcode::SW, false),
        _ => panic!("unsupported instruction"),
    }
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    let args = Args::parse();
    let opcode_str = args.opcode_str.clone();
    let mut search_config = load_config(&args.config).unwrap();
    search_config.enable_heuristic = !args.no_heuristic;
    search_config.enable_interval_refinement = !args.no_refinement;
    let (opcode, is_load) = get_opcode(&opcode_str);

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;

    // ######################## Extract CPU Constraints ##########################
    let air = MemoryInstructionsChip::default();
    let air_name = "MemoryInstrs";
    let _colmap = make_col_map();
    //println!("{:?}", colmap);

    let (mut constraint_info, general_lookup_info) = extract_constraints_and_range::<
        BabyBear,
        MemoryInstructionsChip,
    >(
        &air, NUM_MEMORY_INSTRUCTIONS_COLUMNS, prime, args.method == "bb" && !args.no_simplify
    );
    //  println!("{:?}", general_lookup_info);

    let final_check = generate_memory_op_final_checker(
        2,                    // clk
        vec![3, 4, 5, 6],     // op_a
        vec![7, 8, 9, 10],    // op_b
        vec![11, 12, 13, 14], // op_c
        vec![38, 39, 40, 41], // mem
        general_lookup_info,
    );
    let mut semantic_inputs = vec![
        0, 1, 2, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
        34, 35, 36, 37, 42, 43,
    ];
    if is_load {
        semantic_inputs.extend(&[38, 39, 40, 41]);
        constraint_info.output_columns = vec![3, 4, 5, 6];
    } else {
        semantic_inputs.extend(&[3, 4, 5, 6]);
        constraint_info.output_columns = vec![38, 39, 40, 41];
    }
    constraint_info
        .refinable_cols
        .retain(|c| !semantic_inputs.contains(c));
    search_config.min_row_id = if is_load { 1 } else { 0 };
    search_config.max_row_id = if is_load { 1 } else { 0 };
    if search_config.minimum_num_taregt_cols == 0 {
        search_config.minimum_num_taregt_cols = constraint_info.refinable_cols.len();
    }

    let mut rng = StdRng::seed_from_u64(search_config.seed);
    let mut ds = vec![];
    for i in 0..args.num_trial {
        search_config.seed += i as u64;

        let r1: u8 = rng.random_range(0..32);
        let r2: u8 = rng.random_range(0..32);
        let x: u32 = rng.random();
        let y: u32 = rng.random_range(0..32513);
        let z: u32 = rng.random_range(0..32513);

        // ######################## Program Initialization ###########################
        let program = if is_load {
            target_program_load(opcode, 4, 4, r1, r2, x, y, z)
        } else {
            target_program_store(opcode, 4, 4, r1, r2, x, y, z)
        };
        let num_extracted_rows = if is_load { 2 } else { 1 };
        let base_abs_main_trace_data =
            generate_abstract_trace(&program, air_name.to_string(), num_extracted_rows);

        if base_abs_main_trace_data.is_empty() {
            continue;
        }

        // ######################## Set Info ##########################################
        let program_info = ProgramInfo {
            program_str: get_program_str(&program),
            program_len: program.instructions.len(),
        };

        // ######################## Solve ############################################
        let mut known_solution = HashSet::new();
        let result = experiment_harness(
            &program_info,
            &mut constraint_info,
            &search_config,
            &base_abs_main_trace_data,
            vec![],
            &if args.blocking_closure && args.range_interval == 0 { vec![0usize] } else { vec![] },
            nop_post_process,
            &final_check,
            &args.method,
            &mut known_solution,
            args.turn_off_ui,
        );
        println!("({} {} {} {} {}), {:?}", r1, r2, x, y, z, result);
        ds.push(result.unwrap());
    }
    let report = generate_report(&ds);
    println!("{:?}", report);
    let _ = write_output(args, search_config, report);

    Ok(())
}
