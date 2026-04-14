use clap::Parser;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashSet;
use std::io;

use openvm_rv32im_transpiler::{BranchEqualOpcode, BranchLessThanOpcode};

use zebra::canonicalizer::generate_alu_final_checker;
use zebra::quick::{
    experiment_harness, generate_report, load_config, load_config_for_args, print_sparsity_report, write_output, Args, ProgramInfo,
};
use zebra::solver::nop_post_process;
use zebra::utils::create_or_clear_dir;

use zebra_openvm::utils::{
    extract_branch_eq_constraints, extract_branch_lt_constraints, make_branch_eq_row,
    make_branch_lt_row, to_limbs, BABY_BEAR_PRIME,
};

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    let args = Args::parse();
    let opcode_str = args.opcode_str.clone();
    let mut search_config = load_config_for_args(&args);
    search_config.enable_heuristic = !args.no_heuristic;
    search_config.enable_interval_refinement = !args.no_refinement;

    let mut rng = StdRng::seed_from_u64(search_config.seed);
    let mut ds = vec![];

    match opcode_str.as_str() {
        "BEQ" | "BNE" => {
            let (mut constraint_info, general_lookup_info) =
                extract_branch_eq_constraints(
                    BABY_BEAR_PRIME,
                );
            let final_check = generate_alu_final_checker(general_lookup_info.clone());
            constraint_info
                .refinable_cols
                .extend(general_lookup_info.op_a.clone());
            constraint_info.output_columns = general_lookup_info.op_a.clone();
            if args.sparsity {
                print_sparsity_report(&constraint_info, "BranchEqual");
                return Ok(());
            }
            if search_config.minimum_num_taregt_cols == 0 {
                search_config.minimum_num_taregt_cols = constraint_info.refinable_cols.len();
            }

            let op = match opcode_str.as_str() {
                "BEQ" => BranchEqualOpcode::BEQ,
                _ => BranchEqualOpcode::BNE,
            };

            for _ in 0..args.num_trial {
                let x: u32 = rng.random();
                let y: u32 = rng.random();
                let a_limbs = to_limbs(x);
                let b_limbs = to_limbs(y);
                let base = vec![make_branch_eq_row(a_limbs, b_limbs, op)];
                let program_info = ProgramInfo {
                    program_str: format!("{op:?}({x:#010x}, {y:#010x})"),
                    program_len: 1,
                };
                let mut known_solution = HashSet::new();
                let result = experiment_harness(
                    &program_info,
                    &mut constraint_info.clone(),
                    &search_config,
                    &base,
                    vec![],
                    &vec![],
                    nop_post_process,
                    &final_check,
                    &args.method,
                    &mut known_solution,
                    args.turn_off_ui,
                );
                println!("{op:?}({x:#010x}, {y:#010x}): {:?}", result);
                let r = result.unwrap();
                let verified = r.is_verified();
                ds.push(r);
                if args.fail_fast && !verified {
                    break;
                }
            }
        }
        "BLT" | "BLTU" | "BGE" | "BGEU" => {
            let (mut constraint_info, general_lookup_info) =
                extract_branch_lt_constraints(
                    BABY_BEAR_PRIME,
                );
            let final_check = generate_alu_final_checker(general_lookup_info.clone());
            constraint_info
                .refinable_cols
                .extend(general_lookup_info.op_a.clone());
            constraint_info.output_columns = general_lookup_info.op_a.clone();
            if args.sparsity {
                print_sparsity_report(&constraint_info, "BranchLessThan");
                return Ok(());
            }
            if search_config.minimum_num_taregt_cols == 0 {
                search_config.minimum_num_taregt_cols = constraint_info.refinable_cols.len();
            }

            let op = match opcode_str.as_str() {
                "BLT"  => BranchLessThanOpcode::BLT,
                "BLTU" => BranchLessThanOpcode::BLTU,
                "BGE"  => BranchLessThanOpcode::BGE,
                _      => BranchLessThanOpcode::BGEU,
            };

            for _ in 0..args.num_trial {
                let x: u32 = rng.random();
                let y: u32 = rng.random();
                let a_limbs = to_limbs(x);
                let b_limbs = to_limbs(y);
                let base = vec![make_branch_lt_row(a_limbs, b_limbs, op)];
                let program_info = ProgramInfo {
                    program_str: format!("{op:?}({x:#010x}, {y:#010x})"),
                    program_len: 1,
                };
                let mut known_solution = HashSet::new();
                let result = experiment_harness(
                    &program_info,
                    &mut constraint_info.clone(),
                    &search_config,
                    &base,
                    vec![],
                    &vec![],
                    nop_post_process,
                    &final_check,
                    &args.method,
                    &mut known_solution,
                    args.turn_off_ui,
                );
                println!("{op:?}({x:#010x}, {y:#010x}): {:?}", result);
                let r = result.unwrap();
                let verified = r.is_verified();
                ds.push(r);
                if args.fail_fast && !verified {
                    break;
                }
            }
        }
        other => panic!("unsupported opcode: {other}; use BEQ, BNE, BLT, BLTU, BGE, or BGEU"),
    }

    let report = generate_report(&ds);
    println!("{report:?}");
    let _ = write_output(args, search_config, report);
    Ok(())
}
