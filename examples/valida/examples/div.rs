use clap::Parser;
use core::mem::transmute;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashSet;
use std::io;

use p3_baby_bear::BabyBear;

use valida_alu_u32::div::columns::DIV_COL_MAP;
use valida_alu_u32::div::Div32Chip;
use valida_alu_u32::div::{columns::NUM_DIV_COLS, Div32Instruction};
use valida_basic_api::BasicMachine;
use valida_cpu::Imm32Instruction;
use valida_cpu::StopInstruction;
use valida_machine::{Instruction, InstructionWord, Operands, StarkField};
use valida_opcodes::BYTES_PER_INSTR;

use latticevm::interval::AbstractInterval;
use latticevm::quick::{
    experiment_harness, generate_report, load_config, write_output, Args, ProgramInfo,
};
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

fn final_check(
    trace: &AbstractTrace,
    num_trial: usize,
    prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    let string_representation = format!(
        "input0: [{}], input1: [{}], output: [{}]",
        trace_fmt_with_idxs(trace, 0, &[0, 1, 2, 3]),
        trace_fmt_with_idxs(trace, 0, &[4, 5, 6, 7]),
        trace_fmt_with_idxs(trace, 0, &[8, 9, 10, 11]),
    );

    save_repr_if_unique(&string_representation, known_reprt, ui);
}

fn get_target_program<Val: StarkField>(a: i32, b: i32) -> Vec<InstructionWord<i32>> {
    let bytes_per_instr = BYTES_PER_INSTR as i32;

    let mut program = vec![];
    program.extend([
        InstructionWord {
            opcode: <Imm32Instruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands([-4, a, 0, 0, 0]),
        },
        InstructionWord {
            opcode: <Div32Instruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
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

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(27) + 1;

    // ######################## Solver Parameters ###############################
    let max_iteration = 100000;
    let min_row_id = 0;
    let max_row_id = 0;
    let num_extracted_rows = 1;
    let seed = 41;

    // ######################## Extract Add Constraints ##########################
    println!("ADD AIR MAP");
    println!("  {:?}", DIV_COL_MAP);

    let air = Div32Chip::default();
    let num_col = NUM_DIV_COLS;
    let chip_idx = 6;

    let machine = BasicMachine::<BabyBear>::default();
    let (air_constraints, lookup_constraints, mut refinable_cols, range_types, general_lookup_info) =
        extract_constraints_and_range::<BasicMachine<BabyBear>, MyConfig, _>(
            &machine, &air, num_col, prime,
        );
    refinable_cols.extend(&[8, 9, 10, 11]);

    let constraints = LatticeVMConstraints {
        air_constraints,
        lookup_constraints,
        pv_pos_constraints: vec![],
        pv_neg_constraints: vec![],
    };
    let minimum_num_taregt_cols = 3; //refinable_cols.len();

    // ######################## Program Initialization ###########################
    let program = get_target_program::<BabyBear>(12, 4);
    let program_str = program
        .iter()
        .map(|inst| format!("{}\n", inst))
        .collect::<String>();

    let mut base_abs_main_trace_data =
        generate_bootstrap_trace_from_program(&program, chip_idx, 0, 0x1000);

    /*
    let aux = vec![
        12, 0, 0, 0, 4, 0, 0, 0, 3, 0, 0, 60, 10, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 1, 0, 1, 0,
    ];
    for i in 0..aux.len() {
        base_abs_main_trace_data[0][i] = AbstractInterval::from_i64(aux[i]);
    }
    let at = AbstractTrace::new(base_abs_main_trace_data.clone());
    let con = vec![constraints.air_constraints[45].clone()];
    println!("-------- {}", con[0]);
    let result = eval_air_constraints(&at, None, &con, prime);
    println!("$$$$$$$$$$$$$$$ {:?}", result.0);*/

    /*
        pub fn eval_air_constraints(
        trace: &AbstractTrace,
        public_vals: Option<&[AbstractInterval]>,
        constraints: &[LatticeVMSymbolicExpr],
        prime: u32,
    ) -> (MayBeFlag, i32, HashSet<(usize, usize)>) {
         */

    // ######################## Solve ############################################
    quick_api(
        program_str,
        &constraints,
        &refinable_cols,
        &range_types,
        &vec![],
        &base_abs_main_trace_data,
        vec![],
        max_iteration,
        minimum_num_taregt_cols,
        min_row_id,
        max_row_id,
        program.len(),
        dummy_program_counter_refine_fn,
        dummy_adjust_pc_program,
        final_check,
        prime,
        seed,
    )
}
