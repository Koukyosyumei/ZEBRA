use clap::Parser;
use core::mem::transmute;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashSet;
use std::io;

use p3_koala_bear::KoalaBear;

use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::memory::{
    columns::MemoryInstructionsColumns, columns::NUM_MEMORY_INSTRUCTIONS_COLUMNS,
};
use zkm_core_machine::MemoryInstructionsChip;

use zebra::canonicalizer::generate_memory_op_final_checker;
use zebra::quick::{
    experiment_harness, generate_report, load_config, load_config_for_args, print_all_sparsity_reports, write_output, Args, ProgramInfo,
};
use zebra::solver::nop_post_process;
use zebra::utils::{create_or_clear_dir, indices_arr};

use zebra_ziren::utils::{extract_constraints_and_range, generate_abstract_trace, get_program_str};

const fn make_col_map() -> MemoryInstructionsColumns<usize> {
    let indices_arr = indices_arr::<{ NUM_MEMORY_INSTRUCTIONS_COLUMNS }>();
    unsafe {
        transmute::<[usize; NUM_MEMORY_INSTRUCTIONS_COLUMNS], MemoryInstructionsColumns<usize>>(
            indices_arr,
        )
    }
}

pub fn target_program_load(opcode: Opcode, pc_start: u32, pc_base: u32) -> Program {
    let instructions = vec![
        Instruction::new(Opcode::ADD, 29, 0, 0x12348765, false, true),
        Instruction::new(Opcode::SW, 29, 0, 0x27654320, false, true),
        Instruction::new(opcode, 29, 0, 0x27654320, false, true),
    ];
    Program::new(instructions, pc_start, pc_base)
}

pub fn target_program_store(opcode: Opcode, pc_start: u32, pc_base: u32) -> Program {
    let instructions = vec![
        Instruction::new(Opcode::ADD, 29, 0, 0x12348765, false, true),
        Instruction::new(opcode, 29, 0, 0x27654320, false, true),
    ];
    Program::new(instructions, pc_start, pc_base)
}

pub fn get_opcode(opcode_str: &str) -> (Opcode, bool) {
    match opcode_str {
        "LB" => (Opcode::LB, true),
        "LBU" => (Opcode::LBU, true),
        "LH" => (Opcode::LH, true),
        "LHU" => (Opcode::LHU, true),
        "LW" => (Opcode::LW, true),
        "SB" => (Opcode::SB, false),
        "SH" => (Opcode::SH, false),
        "SW" => (Opcode::SW, false),
        "SC" => (Opcode::SC, false),
        "SWL" => (Opcode::SWL, false),
        "SWR" => (Opcode::SWR, false),
        _ => panic!("unsupported instruction"),
    }
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    let args = Args::parse();
    let opcode_str = args.opcode_str.clone();
    let mut search_config = load_config_for_args(&args);
    search_config.enable_heuristic = !args.no_heuristic;
    search_config.enable_interval_refinement = !args.no_refinement;
    let (opcode, is_load) = get_opcode(&opcode_str);

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;

    // ######################## Extract CPU Constraints ##########################
    let air = MemoryInstructionsChip::default();
    let air_name = "MemoryInstrs";
    let _colmap = make_col_map();
    println!("{:?}", _colmap);

    let (mut constraint_info, general_lookup_info) = extract_constraints_and_range::<
        KoalaBear,
        MemoryInstructionsChip,
    >(
        &air, NUM_MEMORY_INSTRUCTIONS_COLUMNS, prime, args.method == "bb" && !args.no_simplify
    );

    let final_check = generate_memory_op_final_checker(
        3,                    // clk
        vec![4, 5, 6, 7],     // op_a
        vec![8, 9, 10, 11],   // op_b
        vec![12, 13, 14, 15], // op_c
        vec![57, 58, 59, 60], // mem
        general_lookup_info,
    );
    let mut semantic_inputs = vec![
        0, 1, 2, 3, 8, 9, 10, 11, 12, 13, 14, 15, 53, 54, 55, 56, 66, 67, 68, 69,
    ];
    if is_load {
        semantic_inputs.extend(&[57, 58, 59, 60]);
        constraint_info.output_columns = vec![4, 5, 6, 7];
    } else {
        semantic_inputs.extend(&[4, 5, 6, 7]);
        constraint_info.output_columns = vec![57, 58, 59, 60];
    }
    constraint_info
        .refinable_cols
        .retain(|c| !semantic_inputs.contains(c));
    search_config.min_row_id = if is_load { 1 } else { 0 };
    search_config.max_row_id = if is_load { 1 } else { 0 };
    if search_config.minimum_num_taregt_cols == 0 {
        search_config.minimum_num_taregt_cols = constraint_info.refinable_cols.len();
    }

    if args.sparsity {
        print_all_sparsity_reports(&[("MemoryReadWrite", &constraint_info)]);
        return Ok(());
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
            target_program_load(opcode, 4, 4)
        } else {
            target_program_store(opcode, 4, 4)
        };
        let num_extracted_rows = if is_load { 2 } else { 1 };
        let base_abs_main_trace_data =
            generate_abstract_trace(&program, air_name.to_string(), num_extracted_rows);

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
        let r = result.unwrap();
        let verified = r.is_verified();
        ds.push(r);
        if args.fail_fast && !verified {
            break;
        }
    }
    let report = generate_report(&ds);
    println!("{:?}", report);
    let _ = write_output(args, search_config, report);

    Ok(())
}
