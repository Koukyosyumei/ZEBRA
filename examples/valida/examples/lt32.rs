use clap::Parser;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashSet;
use std::io;

use p3_baby_bear::BabyBear;

use valida_alu_u32::lt::columns::LT_COL_MAP;
use valida_alu_u32::lt::columns::NUM_LT_COLS;
use valida_alu_u32::lt::Lt32Chip;
use valida_alu_u32::lt::Lt32Instruction;
use valida_basic_api::BasicMachine;
use valida_cpu::Imm32Instruction;
use valida_cpu::StopInstruction;
use valida_machine::{Instruction, InstructionWord, Operands, StarkField};
use valida_opcodes::BYTES_PER_INSTR;

use latticevm::canonicalizer::generate_alu_final_checker;
use latticevm::canonicalizer::save_repr_if_unique;
use latticevm::quick::{
    experiment_harness, generate_report, load_config, write_output, Args, ProgramInfo,
};
use latticevm::solver::{nop_post_process, RangeType};
use latticevm::trace::trace_fmt_with_idxs;
use latticevm::trace::AbstractTrace;
use latticevm::ui::UiState;
use latticevm::utils::create_or_clear_dir;
use latticevm::utils::PrettySet;

use latticevm_valida::config::MyConfig;
use latticevm_valida::utils::{
    extract_constraints_and_range, generate_bootstrap_trace_from_program,
};

// ############## Final Check Function ##############################
fn final_check(
    trace: &AbstractTrace,
    num_trial: usize,
    prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    let mut record_reprs = HashSet::new();
    for i in 0..trace.data.len() {
        let string_representation = format!(
            "input0: [{}], input1: [{}], output: {}",
            trace_fmt_with_idxs(trace, i, &[0, 1, 2, 3]),
            trace_fmt_with_idxs(trace, i, &[4, 5, 6, 7]),
            trace.data[i][21],
        );
        record_reprs.insert(string_representation);
    }

    save_repr_if_unique(&PrettySet(record_reprs), known_reprt, ui);
}

fn get_target_program<Val: StarkField>(a: i32, b: i32) -> Vec<InstructionWord<i32>> {
    let bytes_per_instr = BYTES_PER_INSTR as i32;
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
            opcode: <Lt32Instruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
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
    println!("LT AIR MAP");
    println!("  {:?}", LT_COL_MAP);

    let air = Lt32Chip::default();
    let num_col = NUM_LT_COLS;
    let chip_idx = 8;

    let machine = BasicMachine::<BabyBear>::default();
    let (mut constraint_info, general_lookup_info) =
        extract_constraints_and_range::<BasicMachine<BabyBear>, MyConfig, _>(
            &machine, &air, num_col, prime,
        );
    let is_reals = [23, 24, 25, 26];
    constraint_info
        .refinable_cols
        .retain(|e| !is_reals.contains(e));
    constraint_info.refinable_cols.extend(&[21]);
    constraint_info.output_columns.extend(&[21]);
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
            generate_bootstrap_trace_from_program(&program, chip_idx, 0, 0x1000);

        // ######################## Set Info ##########################################
        let program_info = ProgramInfo {
            program_str: program_str,
            program_len: program.len(),
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
            final_check,
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
