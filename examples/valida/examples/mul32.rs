use clap::Parser;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashSet;
use std::io;

use p3_baby_bear::BabyBear;

use valida_alu_u32::mul::columns::MUL_COL_MAP;
use valida_alu_u32::mul::Mul32Chip;
use valida_alu_u32::mul::{columns::NUM_MUL_COLS, Mul32Instruction};
use valida_basic_api::BasicMachine;
use valida_cpu::Imm32Instruction;
use valida_cpu::StopInstruction;
use valida_machine::{Instruction, InstructionWord, Operands, StarkField};
use valida_opcodes::BYTES_PER_INSTR;

use zebra::canonicalizer::generate_alu_final_checker;
use zebra::quick::{
    experiment_harness, generate_report, load_config, write_output, Args, ProgramInfo,
};
use zebra::solver::nop_post_process;
use zebra::utils::create_or_clear_dir;

use zebra_valida::config::MyConfig;
use zebra_valida::utils::{
    extract_constraints_and_range, generate_bootstrap_trace_from_program,
};

fn get_target_program<Val: StarkField>(a: i32, b: i32) -> Vec<InstructionWord<i32>> {
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
            opcode: <Mul32Instruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands([-8, -4, b, 0, 1]),
        },
        InstructionWord {
            opcode: <StopInstruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands::default(),
        },
    ]);

    program
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    let args = Args::parse();
    let _opcode_str = args.opcode_str.clone();
    let mut search_config = load_config(&args.config).unwrap();

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(27) + 1;

    // ######################## Extract Add Constraints ##########################
    println!("MUL AIR MAP");
    println!("  {:?}", MUL_COL_MAP);

    let air = Mul32Chip::default();
    let num_col = NUM_MUL_COLS;
    let chip_idx = 5;

    let machine = BasicMachine::<BabyBear>::default();
    let (mut constraint_info, mut general_lookup_info) =
        extract_constraints_and_range::<BasicMachine<BabyBear>, MyConfig, _>(
            &machine, &air, num_col, prime,
        );
    general_lookup_info.op_a.extend(&[8, 9, 10, 11]);
    general_lookup_info.op_b.extend(&[0, 1, 2, 3]);
    general_lookup_info.op_c.extend(&[4, 5, 6, 7]);
    println!("{:?}", general_lookup_info);
    println!("{:?}", constraint_info.range_types);

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
        let x: i32 = rng.r#gen_range(-0x3C000000..0x3C000000);
        let y: i32 = rng.r#gen_range(-0x3C000000..0x3C000000);

        // ######################## Program Initialization ###########################
        let program = get_target_program::<BabyBear>(x, y);
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
        let mut known_solution = HashSet::new();
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
