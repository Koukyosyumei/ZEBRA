use clap::Parser;
use std::collections::HashSet;
use std::fs;
use std::io;

use p3_baby_bear::BabyBear;

use valida_alu_u32::com::columns::COM_COL_MAP;
use valida_alu_u32::com::columns::NUM_COM_COLS;
use valida_alu_u32::com::Com32Chip;
use valida_alu_u32::com::Eq32Instruction;
use valida_alu_u32::com::Ne32Instruction;
use valida_basic_api::BasicMachine;
use valida_cpu::Imm32Instruction;
use valida_cpu::StopInstruction;
use valida_machine::{Instruction, InstructionWord, Operands, StarkField};
use valida_opcodes::BYTES_PER_INSTR;

use latticevm::quick::{experiment_harness, load_config, Args, ProgramInfo};
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn, RangeType};
use latticevm::symbolic::AbstractTrace;
use latticevm::symbolic::LatticeVMConstraints;
use latticevm::ui::generate_alu_final_checker;
use latticevm::ui::save_repr_if_unique;
use latticevm::ui::UiState;
use latticevm::utils::create_or_clear_dir;
use latticevm::utils::trace_fmt_with_idxs;

use latticevm_valida::config::MyConfig;
use latticevm_valida::utils::{
    extract_constraints_and_range, generate_bootstrap_trace_from_program,
};

// ############## Final Check Function ##############################
fn final_check(
    trace: &AbstractTrace,
    _num_trial: usize,
    _prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    let string_representation = format!(
        "input0: [{}], input1: [{}], output: [{}, 0, 0, 0]",
        trace_fmt_with_idxs(trace, 0, &[0, 1, 2, 3]),
        trace_fmt_with_idxs(trace, 0, &[4, 5, 6, 7]),
        trace.data[0][11],
    );

    save_repr_if_unique(&string_representation, known_reprt, ui);
}

fn get_target_program<Val: StarkField>(opcode: u32, a: i32, b: i32) -> Vec<InstructionWord<i32>> {
    let _bytes_per_instr = BYTES_PER_INSTR as i32;

    let mut program = vec![];
    program.extend([
        InstructionWord {
            opcode: <Imm32Instruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands([-4, a, 0, 0, 0]),
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
        "EQ" => <Eq32Instruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
        "NE" => <Ne32Instruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
        _ => panic!("unsupported instruction"),
    }
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    let args = Args::parse();
    let opcode_str = args.opcode_str;
    let mut search_config = load_config(&args.config).unwrap();

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(27) + 1;

    // ######################## Extract Add Constraints ##########################
    println!("COM AIR MAP");
    println!("  {:?}", COM_COL_MAP);

    let air = Com32Chip::default();
    let num_col = NUM_COM_COLS;
    let chip_idx = 9;

    let machine = BasicMachine::<BabyBear>::default();
    let (mut constraint_info, _general_lookup_info) =
        extract_constraints_and_range::<BasicMachine<BabyBear>, MyConfig, _>(
            &machine, &air, num_col, prime,
        );
    constraint_info.refinable_cols.extend(&[11]);
    constraint_info.output_columns.push(11);

    // ######################## Program Initialization ###########################
    let program = get_target_program::<BabyBear>(get_opcode_addsub::<BabyBear>(&opcode_str), 3, 4);
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
    if search_config.minimum_num_taregt_cols == 0 {
        search_config.minimum_num_taregt_cols = constraint_info.refinable_cols.len();
    }

    // ######################## Solve ############################################
    let result = experiment_harness(
        &program_info,
        &mut constraint_info,
        &search_config,
        &base_abs_main_trace_data,
        vec![],
        &vec![], // vec![0],
        dummy_program_counter_refine_fn,
        dummy_adjust_pc_program,
        final_check,
        &args.method,
    );
    println!("{:?}", result);

    Ok(())
}
