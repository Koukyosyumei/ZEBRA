use clap::Parser;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashSet;
use std::io;

use zebra::canonicalizer::generate_alu_final_checker;
use zebra::quick::{
    experiment_harness, generate_report, load_config, load_config_for_args, print_sparsity_report, write_output, Args, ProgramInfo,
};
use zebra::solver::nop_post_process;
use zebra::utils::create_or_clear_dir;

use zebra_openvm::utils::{
    extract_jal_constraints, extract_jalr_constraints, make_jal_row, make_jalr_row, make_lui_row,
    to_limbs, BABY_BEAR_PRIME,
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
        "JAL" => {
            let (mut constraint_info, general_lookup_info) =
                extract_jal_constraints(
                    BABY_BEAR_PRIME,
                );
            let final_check = generate_alu_final_checker(general_lookup_info.clone());
            constraint_info
                .refinable_cols
                .extend(&general_lookup_info.op_a.clone());
            constraint_info.output_columns = general_lookup_info.op_a.clone();
            if args.sparsity {
                print_sparsity_report(&constraint_info, "Jal");
                return Ok(());
            }
            if search_config.minimum_num_taregt_cols == 0 {
                search_config.minimum_num_taregt_cols = constraint_info.refinable_cols.len();
            }

            for _ in 0..args.num_trial {
                let imm: i32 = rng.random_range(-1048576..=1048575); // 21-bit signed
                let from_pc: u32 = 0;
                let base = vec![make_jal_row(from_pc, imm)];
                let program_info = ProgramInfo {
                    program_str: format!("JAL(pc={from_pc}, imm={imm})"),
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
                println!("JAL(pc={from_pc}, imm={imm}): {:?}", result);
                let r = result.unwrap();
                let verified = r.is_verified();
                ds.push(r);
                if args.fail_fast && !verified {
                    break;
                }
            }
        }
        "LUI" => {
            let (mut constraint_info, general_lookup_info) =
                extract_jal_constraints(
                    BABY_BEAR_PRIME,
                );
            let final_check = generate_alu_final_checker(general_lookup_info.clone());
            constraint_info
                .refinable_cols
                .extend(&general_lookup_info.op_a.clone());
            constraint_info.output_columns = general_lookup_info.op_a.clone();
            if args.sparsity {
                print_sparsity_report(&constraint_info, "Lui");
                return Ok(());
            }
            if search_config.minimum_num_taregt_cols == 0 {
                search_config.minimum_num_taregt_cols = constraint_info.refinable_cols.len();
            }

            for _ in 0..args.num_trial {
                let imm: u32 = rng.random_range(0..0x100000u32); // 20-bit
                let base = vec![make_lui_row(imm)];
                let program_info = ProgramInfo {
                    program_str: format!("LUI(imm={imm:#07x})"),
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
                println!("LUI(imm={imm:#07x}): {:?}", result);
                let r = result.unwrap();
                let verified = r.is_verified();
                ds.push(r);
                if args.fail_fast && !verified {
                    break;
                }
            }
        }
        "JALR" => {
            let (mut constraint_info, general_lookup_info) =
                extract_jalr_constraints(BABY_BEAR_PRIME);
            let final_check = generate_alu_final_checker(general_lookup_info.clone());
            constraint_info
                .refinable_cols
                .extend(&general_lookup_info.op_b.clone());
            constraint_info.output_columns = general_lookup_info.op_b.clone();
            if args.sparsity {
                print_sparsity_report(&constraint_info, "Jalr");
                return Ok(());
            }
            if search_config.minimum_num_taregt_cols == 0 {
                search_config.minimum_num_taregt_cols = constraint_info.refinable_cols.len();
            }

            for _ in 0..args.num_trial {
                let rs1_val: u32 = rng.random_range(0..(1 << 29)); // keep to_pc in PC_BITS
                let rs1_limbs = to_limbs(rs1_val);
                let imm: u32 = rng.random_range(0..256u32); // small positive imm
                let base = vec![make_jalr_row(rs1_limbs, imm, 0)];
                let program_info = ProgramInfo {
                    program_str: format!("JALR(rs1={rs1_val:#010x}, imm={imm})"),
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
                println!("JALR(rs1={rs1_val:#010x}, imm={imm}): {:?}", result);
                let r = result.unwrap();
                let verified = r.is_verified();
                ds.push(r);
                if args.fail_fast && !verified {
                    break;
                }
            }
        }
        other => panic!("unsupported opcode: {other}; use JAL, LUI, or JALR"),
    }

    let report = generate_report(&ds);
    println!("{report:?}");
    let _ = write_output(args, search_config, report);
    Ok(())
}
