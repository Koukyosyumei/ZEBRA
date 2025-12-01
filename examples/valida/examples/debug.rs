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
use valida_alu_u32::mul::columns::MUL_COL_MAP;
use valida_alu_u32::mul::Mul32Chip;
use valida_alu_u32::shift::columns::COL_MAP as OtherCOL_MAP;
use valida_alu_u32::shift::Shift32Chip;
use valida_alu_u32::sub::columns::SUB_COL_MAP;
use valida_alu_u32::sub::Sub32Chip;
use valida_alu_u32::sub::Sub32Instruction;
use valida_basic_api::BasicMachine;
use valida_basic_api::BasicMachineMetrics;
use valida_basic_api::ValidaRuntime;
use valida_bytes::build_column_to_op_map;
use valida_cpu::BneInstruction;
use valida_cpu::Imm32Instruction;
use valida_cpu::MachineWithRegisters;
use valida_cpu::StopInstruction;
use valida_cpu::{
    columns::{CPU_COL_MAP, NUM_CPU_COLS},
    CpuChip,
};
use valida_machine::symbolic::symbolic_builder::{
    get_lookup_interactions, get_symbolic_constraints, get_symbolic_lookups, SymbolicAirBuilder,
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
use valida_opcodes::BYTES_PER_INSTR;
use valida_program::MachineWithProgramROM;
use valida_program::ProgramTableType;

use latticevm::interval::AbstractInterval;
use latticevm::solver::run_solver;
use latticevm::solver::RangeType;
use latticevm::symbolic::eval_air_constraints;
use latticevm::symbolic::eval_constraints;
use latticevm::symbolic::gather_boolean_variables;
use latticevm::symbolic::AbstractTrace;
use latticevm::symbolic::LatticeVMConstraints;
use latticevm::ui::UiState;

use latticevm::interval::MayBeFlag;
use latticevm_valida::alu_constraints::get_alu_constraints;
use latticevm_valida::alu_tables::{derive_add_table, derive_com_table, derive_sub_table};
use latticevm_valida::config::{get_machine_config, prover_options, MyConfig};
use latticevm_valida::p3_to_tv::get_converted_symbolicconstraints;
use latticevm_valida::state::valida_abstract_trace_to_abstract_state;
use latticevm_valida::utils::{
    generate_bootstrap_trace_from_program, make_pc_adjuster, refine_pc_interval,
};

fn main() -> Result<(), io::Error> {
    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(27) + 1;

    // ######################## Extract Add Constraints ##########################
    println!("ADD AIR MAP");
    println!("  {:?}", ADD_COL_MAP);
    let chip = Add32Chip::default();
    let machine = BasicMachine::<BabyBear>::default();
    let mut cols_constrained_by_u8_chip = Vec::new();
    let mut cols_constrained_by_cpu_chip = Vec::new();
    let mut counter_col = Vec::new();
    get_lookup_interactions::<BasicMachine<BabyBear>, MyConfig, _>(
        &machine,
        &chip,
        &mut cols_constrained_by_u8_chip,
        &mut cols_constrained_by_cpu_chip,
        &mut counter_col,
    );
    println!("u8: {:?}", cols_constrained_by_u8_chip);
    println!("bus: {:?}", cols_constrained_by_cpu_chip);
    println!("counter: {:?}", counter_col);

    println!("MUL AIR MAP");
    println!("  {:?}", MUL_COL_MAP);
    let chip = Mul32Chip::default();
    let machine = BasicMachine::<BabyBear>::default();
    let mut cols_constrained_by_u8_chip = Vec::new();
    let mut cols_constrained_by_cpu_chip = Vec::new();
    let mut counter_col = Vec::new();
    get_lookup_interactions::<BasicMachine<BabyBear>, MyConfig, _>(
        &machine,
        &chip,
        &mut cols_constrained_by_u8_chip,
        &mut cols_constrained_by_cpu_chip,
        &mut counter_col,
    );
    println!("u8: {:?}", cols_constrained_by_u8_chip);
    println!("bus: {:?}", cols_constrained_by_cpu_chip);
    println!("counter: {:?}", counter_col);

    println!("Shift32 AIR MAP");
    println!("  {:?}", OtherCOL_MAP);
    let chip = Shift32Chip::default();
    let machine = BasicMachine::<BabyBear>::default();
    let mut cols_constrained_by_u8_chip = Vec::new();
    let mut cols_constrained_by_cpu_chip = Vec::new();
    let mut counter_col = Vec::new();
    get_lookup_interactions::<BasicMachine<BabyBear>, MyConfig, _>(
        &machine,
        &chip,
        &mut cols_constrained_by_u8_chip,
        &mut cols_constrained_by_cpu_chip,
        &mut counter_col,
    );
    println!("u8: {:?}", cols_constrained_by_u8_chip);
    println!("bus: {:?}", cols_constrained_by_cpu_chip);
    println!("counter: {:?}", counter_col);

    let map = build_column_to_op_map();

    for (col, info) in map.iter().enumerate() {
        if let Some(info) = info {
            println!(
                "col {} => op {:?}, output_index {}",
                col, info.op, info.output_index
            );
        }
    }

    Ok(())
}
