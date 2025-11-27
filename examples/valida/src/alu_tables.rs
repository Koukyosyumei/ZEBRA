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

use crate::p3_to_tv::convert_p3_expr;
use crate::p3_to_tv::get_converted_symbolicconstraints;
use crate::state::valida_abstract_trace_to_abstract_state;

pub fn derive_add_table(
    cpu_main_trace: &Vec<Vec<AbstractInterval>>,
    potential_boolean_vars: &Vec<usize>,
    prime: u32,
) -> Vec<Vec<AbstractInterval>> {
    let mut out = vec![];
    for row in cpu_main_trace {
        if row[58].as_canonical_u32(prime) != 0 && row[3].as_canonical_u32(prime) == 100 {
            let mut r: Vec<_> = (0..16).map(|_| AbstractInterval::top(prime)).collect();
            for i in potential_boolean_vars {
                r[*i] = AbstractInterval::bool();
            }

            let cpu_columns = vec![32, 33, 34, 35, 38, 39, 40, 41, 44, 45, 46, 47];
            let add_columns = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
            for i in 0..(cpu_columns.len()) {
                r[add_columns[i]] = row[cpu_columns[i]].clone();
            }
            r[15] = AbstractInterval::one();
            out.push(r);
        }
    }

    out
}

pub fn derive_sub_table(
    cpu_main_trace: &Vec<Vec<AbstractInterval>>,
    potential_boolean_vars: &Vec<usize>,
    prime: u32,
) -> Vec<Vec<AbstractInterval>> {
    let mut out = vec![];
    for row in cpu_main_trace {
        if row[58].as_canonical_u32(prime) != 0 && row[3].as_canonical_u32(prime) == 101 {
            let mut r: Vec<_> = (0..17).map(|_| AbstractInterval::top(prime)).collect();
            for i in potential_boolean_vars {
                r[*i] = AbstractInterval::bool();
            }

            let cpu_columns = vec![32, 33, 34, 35, 38, 39, 40, 41, 44, 45, 46, 47];
            let sub_columns = vec![0, 1, 2, 3, 4, 5, 6, 7, 12, 13, 14, 15];
            for i in 0..(cpu_columns.len()) {
                r[sub_columns[i]] = row[cpu_columns[i]].clone();
            }
            r[16] = AbstractInterval::one();
            out.push(r);
        }
    }

    out
}

pub fn derive_com_table(
    cpu_main_trace: &Vec<Vec<AbstractInterval>>,
    potential_boolean_vars: &Vec<usize>,
    prime: u32,
) -> Vec<Vec<AbstractInterval>> {
    let mut out = vec![];
    for row in cpu_main_trace {
        if row[58].as_canonical_u32(prime) != 0
            && (row[3].as_canonical_u32(prime) == 116 || row[3].as_canonical_u32(prime) == 111)
        {
            let mut r: Vec<_> = (0..14).map(|_| AbstractInterval::i4()).collect();
            for i in potential_boolean_vars {
                r[*i] = AbstractInterval::bool();
            }

            let cpu_columns = vec![32, 33, 34, 35, 38, 39, 40, 41, 44];
            let com_columns = vec![0, 1, 2, 3, 4, 5, 6, 7, 11];
            for i in 0..(cpu_columns.len()) {
                r[com_columns[i]] = row[cpu_columns[i]].clone();
            }
            if row[3].as_canonical_u32(prime) == 116 {
                r[13] = AbstractInterval::one();
                r[12] = AbstractInterval::zero();
            } else {
                r[13] = AbstractInterval::zero();
                r[12] = AbstractInterval::one();
            }
            out.push(r);
        }
    }

    out
}
