use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::rc::Rc;
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
use valida_opcodes::BYTES_PER_INSTR;
use valida_program::MachineWithProgramROM;
use valida_program::ProgramTableType;

use latticevm::interval::AbstractInterval;
use latticevm::interval::MayBeFlag;
use latticevm::solver::run_solver;
use latticevm::solver::AbsConstraintObj;
use latticevm::symbolic::eval_air_constraints;
use latticevm::symbolic::eval_constraints;
use latticevm::symbolic::gather_boolean_variables;
use latticevm::symbolic::AbstractTrace;
use latticevm::symbolic::LatticeVMConstraints;
use latticevm::ui::UiState;

use crate::config::MyConfig;
use crate::p3_to_tv::convert_p3_expr;
use crate::p3_to_tv::get_converted_symbolicconstraints;
use crate::state::valida_abstract_trace_to_abstract_state;

pub fn get_alu_constraints() -> HashMap<String, AbsConstraintObj> {
    let machine = BasicMachine::<BabyBear>::default();
    let mut result = HashMap::new();

    let add_air = Add32Chip::default();
    let (add_constraints, add_potential_boolean_vars) =
        get_converted_symbolicconstraints::<BasicMachine<BabyBear>, MyConfig, _>(
            &machine, &add_air,
        );
    let add_target_cols = vec![8, 9, 10, 11, 12, 13, 14];
    let aux_add_obj = AbsConstraintObj {
        name: "Add".to_string(),
        aux_constraints: add_constraints,
        aux_target_cols: add_target_cols,
        aux_potential_boolean_vars: add_potential_boolean_vars,
    };
    result.insert("Add".to_string(), aux_add_obj);

    let sub_air = Sub32Chip::default();
    let (sub_constraints, sub_potential_boolean_vars) =
        get_converted_symbolicconstraints::<BasicMachine<BabyBear>, MyConfig, _>(
            &machine, &sub_air,
        );
    let sub_target_cols = vec![8, 9, 10, 11];
    let aux_sub_obj = AbsConstraintObj {
        name: "Sub".to_string(),
        aux_constraints: sub_constraints,
        aux_target_cols: sub_target_cols,
        aux_potential_boolean_vars: sub_potential_boolean_vars,
    };
    result.insert("Sub".to_string(), aux_sub_obj);

    let bitwise_air = Bitwise32Chip::default();
    let (bitsise_constraints, bitsise_potential_boolean_vars) = get_converted_symbolicconstraints::<
        BasicMachine<BabyBear>,
        MyConfig,
        _,
    >(&machine, &bitwise_air);
    let bitsise_target_cols = (0..64).collect::<Vec<usize>>();
    //result.insert("Bitwise".to_string(), aux_sub_obj);

    let com_air = Com32Chip::default();
    let (com_constraints, com_potential_boolean_vars) =
        get_converted_symbolicconstraints::<BasicMachine<BabyBear>, MyConfig, _>(
            &machine, &com_air,
        );
    let com_target_cols = vec![8, 9, 10];
    let aux_com_obj = AbsConstraintObj {
        name: "Com".to_string(),
        aux_constraints: com_constraints,
        aux_target_cols: com_target_cols,
        aux_potential_boolean_vars: com_potential_boolean_vars,
    };
    result.insert("Com".to_string(), aux_com_obj);

    result
}
