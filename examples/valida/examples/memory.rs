use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::rc::Rc;
use std::time;
use std::{io, thread, time::Duration};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::rngs::StdRng;
use rand::thread_rng;
use rand::SeedableRng;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

use p3_baby_bear::BabyBear;
use p3_challenger::DuplexChallenger;
use p3_commit::ExtensionMmcs;
use p3_dft::Radix2Bowers;
use p3_field::extension::BinomialExtensionField;
use p3_field::{AbstractField, Field, PrimeField32, TwoAdicField};
use p3_fri::FriConfig;
use p3_fri::{TwoAdicFriPcs, TwoAdicFriPcsConfig};
use p3_keccak::Keccak256Hash;
use p3_matrix::dense::RowMajorMatrix;
use p3_matrix::Matrix;
use p3_mds::coset_mds::CosetMds;
use p3_merkle_tree::FieldMerkleTreeMmcs;
use p3_poseidon::Poseidon;
use p3_symmetric::{CompressionFunctionFromHasher, SerializingHasher32};
//use p3_uni_stark::symbolic_builder::get_symbolic_constraints;

use valida_alu_u32::add::columns::ADD_COL_MAP;
use valida_alu_u32::add::Add32Chip;
use valida_alu_u32::add::{columns::NUM_ADD_COLS, Add32Instruction, MachineWithAdd32Chip};
use valida_alu_u32::bitwise::columns::COL_MAP;
use valida_alu_u32::bitwise::Bitwise32Chip;
use valida_alu_u32::com::columns::COM_COL_MAP;
use valida_alu_u32::com::Com32Chip;
use valida_alu_u32::com::Eq32Instruction;
use valida_alu_u32::com::Ne32Instruction;
use valida_alu_u32::mul::Mul32Chip;
use valida_alu_u32::sub::columns::NUM_SUB_COLS;
use valida_alu_u32::sub::columns::SUB_COL_MAP;
use valida_alu_u32::sub::Sub32Chip;
use valida_alu_u32::sub::Sub32Instruction;
use valida_basic_api::BasicMachine;
use valida_basic_api::BasicMachineMetrics;
use valida_basic_api::ValidaRuntime;
use valida_cpu::BneInstruction;
use valida_cpu::Imm32Instruction;
use valida_cpu::MachineWithRegisters;
use valida_cpu::StopInstruction;
use valida_cpu::{
    columns::{CPU_COL_MAP, NUM_CPU_COLS},
    CpuChip,
};
use valida_machine::symbolic::symbolic_builder::{
    get_symbolic_constraints, get_symbolic_lookups, SymbolicAirBuilder,
};
use valida_machine::symbolic::symbolic_expression::SymbolicExpression;
use valida_machine::Chip;
use valida_machine::ChipWithPersistence;
use valida_machine::StarkConfig;
use valida_machine::StarkConfigImpl;
use valida_machine::{
    check_constraints::display_interaction, Instruction, InstructionWord, Machine, MachineProof,
    MachineRuntime, MemoryBackendTrait, MultiSegmentMachineProof, Operands, ProgramROM,
    ProverOptions, SegmentMachine, StarkField, ValidaMemoryBackend, Word,
};
use valida_memory::columns::{MemoryCols, MEM_COL_MAP, NUM_MEM_COLS};
use valida_memory::MemoryChip;
use valida_opcodes::BYTES_PER_INSTR;
use valida_program::MachineWithProgramROM;
use valida_program::ProgramTableType;

use latticevm::interval::{AbstractInterval, MayBeFlag};
use latticevm::quick::quick_api;
use latticevm::solver::{
    dummy_adjust_pc_program, dummy_program_counter_refine_fn, dummy_table_deriver,
};
use latticevm::state::MemoryOp;
use latticevm::state::MemoryOpKind;
use latticevm::symbolic::{AbstractTrace, LatticeVMConstraints};
use latticevm::ui::save_repr_if_unique;
use latticevm::ui::UiState;
use latticevm::utils::create_or_clear_dir;

use latticevm_valida::config::MyConfig;
use latticevm_valida::utils::{
    extract_constraints_and_range, generate_bootstrap_trace_from_program, make_pc_adjuster,
    refine_pc_interval,
};

fn program_counter_refine_fn(
    abs_main_trace_data: &mut Vec<Vec<AbstractInterval>>,
    program_len: usize,
    i: usize,
    j: usize,
) {
}

fn adjust_pc_program(main_trace: &mut AbstractTrace, prime: u32) {}

fn reconstruct_word(row: &[AbstractInterval], base: usize) -> AbstractInterval {
    let mut val = AbstractInterval::from_i64(0);
    let mut mul = 1_i64;
    for i in 0..4 {
        val = val + row[base + i].clone() * AbstractInterval::from_i64(mul);
        mul *= 256;
    }
    val
}

// ############## Final Check Function ##############################
fn final_check(
    trace: &AbstractTrace,
    num_trial: usize,
    prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    let mut string_representation = String::new();

    let num_row = trace.data.len();
    let def_interval = AbstractInterval::zero();
    let mut memory = HashMap::<i64, AbstractInterval>::new();
    let mut is_consistent_flag = true;
    let mut break_point = 0;

    for i in 0..num_row {
        let addr = trace.data[i][12].clone();
        let value = reconstruct_word(&trace.data[i], 4);
        let is_read = trace.data[i][14].clone() + trace.data[i][15].clone();
        let is_write = &trace.data[i][16];
        string_representation.push_str(&format!(
            "addr: {}, value: {}, is_read: {}, is_write: {}\n",
            addr, value, is_read, is_write
        ));

        if is_read.is_zero(prime) != MayBeFlag::True {
            for a in addr.lo..(addr.hi + 1) {
                let prev_value = memory.get(&a).unwrap_or(&def_interval);
                if (value.clone() - prev_value.clone()).is_zero(prime) != MayBeFlag::True {
                    is_consistent_flag = false;
                    string_representation.push_str("crash\n");
                    break_point = i;
                }
            }
        }

        if is_write.is_zero(prime) != MayBeFlag::True {
            for a in addr.lo..(addr.hi + 1) {
                memory.insert(a, value.clone());
            }
        }
    }

    if !is_consistent_flag {
        save_repr_if_unique(&string_representation, known_reprt, ui);
    }
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
            opcode: <Sub32Instruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
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
    let max_iteration = 10000;
    let min_row_id = 1;
    let max_row_id = 1;
    let num_extracted_rows = 1;
    let seed = 41;

    // ######################## Extract Add Constraints ##########################
    println!("MEM AIR MAP");
    println!("  {:?}", MEM_COL_MAP);

    let air = MemoryChip::default();
    let num_col = NUM_MEM_COLS;
    let chip_idx = 2;

    let machine = BasicMachine::<BabyBear>::default();
    let (air_constraints, lookup_constraints, mut refinable_cols, range_types, general_lookup_info) =
        extract_constraints_and_range::<BasicMachine<BabyBear>, MyConfig, _>(
            &machine, &air, num_col, prime,
        );

    let constraints = LatticeVMConstraints {
        air_constraints,
        lookup_constraints,
        pv_pos_constraints: vec![],
        pv_neg_constraints: vec![],
    };
    let minimum_num_taregt_cols = 1; //refinable_cols.len();

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
