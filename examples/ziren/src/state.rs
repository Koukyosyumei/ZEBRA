use std::collections::HashMap;
use std::io;
use std::rc::Rc;

use itertools::Itertools;
use rand::{rngs::StdRng, SeedableRng};

use p3_air::BaseAir;
use p3_field::PrimeField32;
use p3_mersenne_31::Mersenne31;
use p3_uni_stark::{get_symbolic_constraints, SymbolicExpression};

use zkm_core_executor::{
    syscalls::SyscallCode, ExecutionState, Executor, Instruction, Opcode, Program,
};
use zkm_core_machine::mips::MipsAir;
use zkm_core_machine::utils::trace_checkpoint;
use zkm_core_machine::utils::ZKMCoreProverError;
use zkm_core_machine::CpuChip;
use zkm_stark::{koala_bear_poseidon2::KoalaBearPoseidon2, StarkGenericConfig};
use zkm_stark::{CpuProver, MachineProver};
use zkm_stark::{ZKMCoreOpts, ZKM_PROOF_NUM_PV_ELTS};

use latticevm::interval::MayBeFlag;
use latticevm::smt::expr_to_smt;
use latticevm::state::AbstractState;
use latticevm::symbolic::preprocess_row;
use latticevm::symbolic::LatticeVMSymbolicEntry;
use latticevm::symbolic::LatticeVMSymbolicExpr;
use latticevm::symbolic::LatticeVMSymbolicVal;
use latticevm::{
    interval::AbstractInterval, solver::solve, symbolic::gather_boolean_variables,
    symbolic::AbstractTrace, symbolic::LatticeVMConstraints, utils::BitCombinationsDictOrder,
};

use crate::p3_to_tv::convert_p3_expr;

pub fn ziren_state_to_abstract_state(ziren_state: &ExecutionState) -> AbstractState {
    let memory = ziren_state
        .memory
        .clone()
        .into_iter()
        .map(|(addr, record)| (addr, AbstractInterval::from_i64(record.value as i64)))
        .collect();

    AbstractState {
        clk: AbstractInterval::from_i64(ziren_state.clk as i64),
        pc: AbstractInterval::from_i64(ziren_state.pc as i64),
        next_pc: AbstractInterval::from_i64(ziren_state.next_pc as i64),
        is_done: if ziren_state.pc == 0 {
            MayBeFlag::True
        } else {
            MayBeFlag::False
        }, // TODO || ziren_state.exited,
        memory: memory,
    }
}

pub fn ziren_abstract_trace_to_abstract_state(
    abstract_row: &Vec<AbstractInterval>,
    prime: u32,
) -> AbstractState {
    AbstractState {
        clk: abstract_row[1].clone()
            + abstract_row[2].clone() * AbstractInterval::from_i64(2_usize.pow(16) as i64),
        pc: abstract_row[5].clone(),
        next_pc: abstract_row[6].clone(),
        is_done: abstract_row[5].clone().is_zero(prime),
        memory: HashMap::new(),
    }
}
