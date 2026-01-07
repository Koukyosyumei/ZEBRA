use std::io;

use p3_baby_bear::BabyBear;

use valida_alu_u32::add::columns::ADD_COL_MAP;
use valida_alu_u32::add::Add32Chip;
use valida_alu_u32::add::{columns::NUM_ADD_COLS, Add32Instruction};
use valida_basic_api::BasicMachine;
use valida_cpu::Imm32Instruction;
use valida_cpu::StopInstruction;
use valida_machine::{Instruction, InstructionWord, Operands, StarkField};
use valida_opcodes::BYTES_PER_INSTR;

use latticevm::interval::{AbstractInterval, MayBeFlag};
use latticevm::quick::quick_api;
use latticevm::solver::RangeType;
use latticevm::solver::{
    dummy_adjust_pc_program, dummy_program_counter_refine_fn, dummy_table_deriver,
};
use latticevm::symbolic::{AbstractTrace, LatticeVMConstraints};
use latticevm::ui::generate_alu_final_checker;
use latticevm::ui::UiState;
use latticevm::utils::create_or_clear_dir;

use latticevm_valida::config::MyConfig;
use latticevm_valida::state::valida_abstract_trace_to_abstract_state;
use latticevm_valida::utils::{
    extract_constraints_and_range, generate_bootstrap_trace_from_program, make_pc_adjuster,
};

fn get_target_program<Val: StarkField>(a: i32, b: i32) -> Vec<InstructionWord<i32>> {
    let bytes_per_instr = BYTES_PER_INSTR as i32;

    let mut program = vec![];
    program.extend([
        InstructionWord {
            opcode: <Imm32Instruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands([-4, a, 0, 0, 0]),
        },
        InstructionWord {
            opcode: <Add32Instruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
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
    let max_iteration = 100000000;
    let min_row_id = 0;
    let max_row_id = 0;
    let num_extracted_rows = 1;
    let seed = 41;

    // ######################## Extract Add Constraints ##########################
    println!("ADD AIR MAP");
    println!("  {:?}", ADD_COL_MAP);

    let air = Add32Chip::default();
    let num_col = NUM_ADD_COLS;
    let chip_idx = 3;

    let machine = BasicMachine::<BabyBear>::default();
    let (tv_constraints, mut refinable_cols, mut range_types, general_lookup_info) =
        extract_constraints_and_range::<BasicMachine<BabyBear>, MyConfig, _>(
            &machine, &air, num_col, prime,
        );
    let final_check = generate_alu_final_checker(general_lookup_info.clone());
    refinable_cols.extend(&general_lookup_info.alu_output);
    refinable_cols.extend(&[0, 4]);
    range_types.insert(0, RangeType::U4);
    range_types.insert(4, RangeType::U4);

    for t in &tv_constraints {
        println!("#### {}", t);
    }
    println!("{:?}", refinable_cols);
    println!("{:?}", range_types);

    let constraints = LatticeVMConstraints {
        air_constraints: tv_constraints.clone(),
        pv_pos_constraints: vec![],
        pv_neg_constraints: vec![],
    };
    let minimum_num_taregt_cols = refinable_cols.len();

    // ######################## Program Initialization ###########################
    let program = get_target_program::<BabyBear>(3, 4);
    let program_str = program
        .iter()
        .map(|inst| format!("{}\n", inst))
        .collect::<String>();

    let base_abs_main_trace_data =
        generate_bootstrap_trace_from_program(&program, chip_idx, 0, 0x1000);

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
