use clap::Parser;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashSet;
use std::io;

use openvm_rv32im_transpiler::BaseAluOpcode;

use zebra::canonicalizer::generate_alu_final_checker;
use zebra::quick::{
    experiment_harness, generate_report, load_config, load_config_for_args, print_all_sparsity_reports, write_output, Args, ProgramInfo,
};
use zebra::solver::nop_post_process;
use zebra::utils::create_or_clear_dir;

use zebra_openvm::utils::{
    extract_base_alu_constraints, make_base_alu_row, to_limbs, BABY_BEAR_PRIME,
};

pub fn get_opcode(opcode_str: &str) -> BaseAluOpcode {
    match opcode_str {
        "AND" => BaseAluOpcode::AND,
        "OR"  => BaseAluOpcode::OR,
        "XOR" => BaseAluOpcode::XOR,
        _ => panic!("unsupported opcode: {opcode_str}; use AND, OR, or XOR"),
    }
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    let args = Args::parse();
    let opcode_str = args.opcode_str.clone();
    let mut search_config = load_config_for_args(&args);
    search_config.enable_heuristic = !args.no_heuristic;
    search_config.enable_interval_refinement = !args.no_refinement;

    let op = get_opcode(&opcode_str);

    let (mut constraint_info, general_lookup_info) =
        extract_base_alu_constraints(BABY_BEAR_PRIME);
    let final_check = generate_alu_final_checker(general_lookup_info.clone());

    constraint_info
        .refinable_cols
        .extend(&general_lookup_info.op_a.clone());
    constraint_info.output_columns = general_lookup_info.op_a.clone();
    if search_config.minimum_num_taregt_cols == 0 {
        search_config.minimum_num_taregt_cols = constraint_info.refinable_cols.len();
    }

    if args.sparsity {
        print_all_sparsity_reports(&[("BitwiseAlu", &constraint_info)]);
        return Ok(());
    }
    let mut rng = StdRng::seed_from_u64(search_config.seed);
    let mut ds = vec![];
    for _ in 0..args.num_trial {
        let mut new_constraint_info = constraint_info.clone();
        if args.range_interval != 0 {
            use rand::prelude::IndexedRandom;
            use rand::seq::SliceRandom;
            use zebra::solver::RangeType;
            let mut rng2 = StdRng::seed_from_u64(search_config.seed);
            let op_b = general_lookup_info.op_b.clone();
            let op_c = general_lookup_info.op_c.clone();
            let b = op_b.choose(&mut rng2).unwrap();
            let c = op_c.choose(&mut rng2).unwrap();
            let b_lo = rng2.random_range(0..(255 - args.range_interval));
            let c_lo = rng2.random_range(0..(255 - args.range_interval));
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

        let x: u32 = rng.random();
        let y: u32 = rng.random();
        let b_limbs = to_limbs(x);
        let c_limbs = to_limbs(y);

        let base_abs_main_trace_data = vec![make_base_alu_row(b_limbs, c_limbs, op)];

        let program_info = ProgramInfo {
            program_str: format!("{op:?}({x:#010x}, {y:#010x})"),
            program_len: 1,
        };

        let mut known_solution = HashSet::new();
        let result = experiment_harness(
            &program_info,
            &mut new_constraint_info,
            &search_config,
            &base_abs_main_trace_data,
            vec![],
            &if args.blocking_closure && args.range_interval == 0 {
                vec![0usize]
            } else {
                vec![]
            },
            nop_post_process,
            &final_check,
            &args.method,
            &mut known_solution,
            args.turn_off_ui,
        );
        println!("({x:#010x}, {y:#010x}): {:?}", result);
        let r = result.unwrap();
        let verified = r.is_verified();
        ds.push(r);
        if args.fail_fast && !verified {
            break;
        }
    }
    let report = generate_report(&ds);
    println!("{report:?}");
    let _ = write_output(args, search_config, report);

    Ok(())
}
