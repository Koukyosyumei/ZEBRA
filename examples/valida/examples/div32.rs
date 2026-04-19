use clap::Parser;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashSet;
use std::io;

use p3_baby_bear::BabyBear;

use valida_alu_u32::div::columns::DIV_COL_MAP;
use valida_alu_u32::div::Div32Chip;
use valida_alu_u32::div::{columns::NUM_DIV_COLS, Div32Instruction, SDiv32Instruction};
use valida_basic_api::BasicMachine;
use valida_cpu::Imm32Instruction;
use valida_cpu::StopInstruction;
use valida_machine::{Instruction, InstructionWord, Operands, StarkField};
use valida_opcodes::BYTES_PER_INSTR;

use zebra::canonicalizer::generate_alu_final_checker;
use zebra::quick::{
    experiment_harness, generate_report, load_config, load_config_for_args, print_all_sparsity_reports, write_output, Args, ProgramInfo,
};
use zebra::solver::nop_post_process;
use zebra::utils::create_or_clear_dir;

use zebra_valida::config::MyConfig;
use zebra_valida::utils::{extract_constraints_and_range, generate_bootstrap_trace_from_program};

fn get_target_program<Val: StarkField>(opcode: u32, a: i32, b: i32) -> Vec<InstructionWord<i32>> {
    let _bytes_per_instr = BYTES_PER_INSTR as i32;
    let a_bytes = a.to_le_bytes();

    let mut program = vec![];
    program.extend([
        InstructionWord {
            opcode: <Imm32Instruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands([
                -4,
                a_bytes[0] as i32,
                a_bytes[1] as i32,
                a_bytes[2] as i32,
                a_bytes[3] as i32,
            ]),
        },
        InstructionWord {
            opcode: opcode,
            operands: Operands([-8, -4, b, 0, 1]),
        },
        InstructionWord {
            opcode: <StopInstruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands::default(),
        },
    ]);

    program
}

pub fn get_opcode_addsub<Val: StarkField>(target_opcode: &str) -> u32 {
    match target_opcode {
        "DIV" => <Div32Instruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
        "SDIV" => <SDiv32Instruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
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

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(27) + 1;

    // ######################## Extract Add Constraints ##########################
    //println!("DIV AIR MAP");
    //println!("  {:?}", DIV_COL_MAP);

    let air = Div32Chip::default();
    let num_col = NUM_DIV_COLS;
    let chip_idx = 6;

    let machine = BasicMachine::<BabyBear>::default();
    let (mut constraint_info, general_lookup_info) =
        extract_constraints_and_range::<BasicMachine<BabyBear>, MyConfig, _>(
            &machine,
            &air,
            num_col,
            prime,
            args.method == "bb" && !args.no_simplify,
        );
    let final_check = generate_alu_final_checker(general_lookup_info.clone());
    constraint_info
        .refinable_cols
        .extend(&general_lookup_info.op_a);
    constraint_info.output_columns = general_lookup_info.op_a;
    if search_config.minimum_num_taregt_cols == 0 {
        search_config.minimum_num_taregt_cols = constraint_info.refinable_cols.len();
    }

    if args.sparsity {
        print_all_sparsity_reports(&[("Div32", &constraint_info)]);
        return Ok(());
    }
    let mut rng = StdRng::seed_from_u64(search_config.seed);
    let mut ds = vec![];
    for _ in 0..args.num_trial {
        let x: i32 = rng.r#gen_range(-0x3C000000..0x3C000000);
        let y: i32 = rng.r#gen_range(-0x3C000000..0x3C000000);

        // ######################## Program Initialization ###########################
        let program = get_target_program::<BabyBear>(
            get_opcode_addsub::<BabyBear>(&opcode_str),
            x as i32,
            y as i32,
        );
        let program_str = program
            .iter()
            .map(|inst| format!("{}\n", inst))
            .collect::<String>();
        let base_abs_main_trace_data =
            generate_bootstrap_trace_from_program(&program, chip_idx, 0, 0x1000).1;

        // ######################## Set Info ##########################################
        let program_info = ProgramInfo {
            program_str: program_str,
            program_len: program.len(),
        };

        // ######################## Solve ############################################
        constraint_info.constraints.blocking_constraints.clear();
        let mut known_solution = HashSet::new();
        let result = experiment_harness(
            &program_info,
            &mut constraint_info,
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
        println!("({} {}), {:?}", x, y, result);
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
