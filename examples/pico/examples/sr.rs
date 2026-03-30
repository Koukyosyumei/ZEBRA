use clap::Parser;
use core::mem::transmute;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashSet;
use std::io;

use p3_koala_bear::KoalaBear;

use pico_vm::chips::chips::alu::sr::columns::ShiftRightCols;
use pico_vm::chips::chips::alu::sr::traces::ShiftRightChip;
use pico_vm::compiler::riscv::program::Program;
use pico_vm::compiler::riscv::{instruction::Instruction, opcode::Opcode};

use zebra::canonicalizer::generate_alu_final_checker;
use zebra::quick::{
    experiment_harness, generate_report, load_config, write_output, Args, ProgramInfo,
};
use zebra::solver::nop_post_process;
use zebra::utils::{create_or_clear_dir, indices_arr};

const NUM_SLR_COLS: usize = size_of::<ShiftRightCols<u8>>();

use zebra_pico::utils::{extract_constraints_and_range, generate_abstract_trace, get_program_str};

const fn make_col_map() -> ShiftRightCols<usize> {
    let indices_arr = indices_arr::<{ NUM_SLR_COLS }>();
    unsafe { transmute::<[usize; NUM_SLR_COLS], ShiftRightCols<usize>>(indices_arr) }
}

pub fn target_program(pc_start: u32, pc_base: u32, x: u32, y: u32) -> Program {
    let instructions = vec![Instruction::new(Opcode::SRL, 1, x, y, true, true)];
    Program::new(instructions, pc_start, pc_base)
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    let args = Args::parse();
    let _opcode_str = args.opcode_str.clone();
    let mut search_config = load_config(&args.config).unwrap();

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;

    // ######################## Extract CPU Constraints ##########################
    let air: ShiftRightChip<KoalaBear> = ShiftRightChip::default();
    let air_name = "ShiftRight";
    let _colmap = make_col_map();

    let (mut constraint_info, general_lookup_info) = extract_constraints_and_range::<
        KoalaBear,
        ShiftRightChip<KoalaBear>,
    >(&air, NUM_SLR_COLS, prime, args.method == "bb");
    let final_check = generate_alu_final_checker(general_lookup_info.clone());
    constraint_info
        .refinable_cols
        .extend(&general_lookup_info.op_a);
    constraint_info.output_columns = general_lookup_info.op_a;
    if search_config.minimum_num_taregt_cols == 0 {
        search_config.minimum_num_taregt_cols = constraint_info.refinable_cols.len();
    }

    let mut rng = StdRng::seed_from_u64(search_config.seed);
    let mut ds = vec![];
    for _ in 0..args.num_trial {
        let mut new_constraint_info = constraint_info.clone();
        if args.range_interval != 0 {
            use rand::prelude::IndexedRandom;
            use zebra::solver::RangeType;
            let mut rng = StdRng::seed_from_u64(search_config.seed);
            let op_b = general_lookup_info.op_b.clone();
            let op_c = general_lookup_info.op_c.clone();
            let b = op_b.choose(&mut rng).unwrap();
            let c = op_c.choose(&mut rng).unwrap();
            let b_lo = rng.random_range(0..(255 - args.range_interval));
            let c_lo = rng.random_range(0..(255 - args.range_interval));

            new_constraint_info.refinable_cols.push(*b);
            new_constraint_info.refinable_cols.push(*c);
            new_constraint_info.range_types.insert(
                *b,
                RangeType::Any(b_lo as i128, (b_lo + args.range_interval) as i128),
            );
            new_constraint_info.range_types.insert(
                *c,
                RangeType::Any(c_lo as i128, (c_lo + args.range_interval) as i128),
            );
            search_config.minimum_num_taregt_cols = new_constraint_info.refinable_cols.len();
        }
        let x: u32 = rng.random_range(0..4);
        let y: u32 = rng.random_range(0..4);

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4, x, y);
        let base_abs_main_trace_data = generate_abstract_trace(&program, air_name.to_string(), 1);

        // ######################## Set Info ##########################################
        let program_info = ProgramInfo {
            program_str: get_program_str(&program),
            program_len: program.instructions.len(),
        };

        // ######################## Solve ############################################
        let mut known_solution = HashSet::new();
        let result = experiment_harness(
            &program_info,
            &mut new_constraint_info,
            &search_config,
            &base_abs_main_trace_data,
            vec![],
            &if args.blocking_closure && args.range_interval == 0 { vec![0usize] } else { vec![] },
            nop_post_process,
            &final_check,
            &args.method,
            &mut known_solution,
        );
        println!("({} {}), {:?}", x, y, result);
        ds.push(result.unwrap());
    }
    let report = generate_report(&ds);
    println!("{:?}", report);
    let _ = write_output(args, search_config, report);

    Ok(())
}
