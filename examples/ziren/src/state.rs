use std::collections::HashSet;

use zkm_core_executor::ExecutionState;
use zkm_stark::MachineProver;

use latticevm::interval::AbstractInterval;
use latticevm::interval::MayBeFlag;
use latticevm::state::AbstractState;

pub fn ziren_state_to_abstract_state(ziren_state: &ExecutionState) -> AbstractState {
    /*
    let memory = ziren_state
        .memory
        .clone()
        .into_iter()
        .map(|(addr, record)| (addr, AbstractInterval::from_i64(record.value as i64)))
        .collect();*/
    let memory_ops = Vec::new();

    AbstractState {
        clk: AbstractInterval::from_i64(ziren_state.clk as i64),
        pc: AbstractInterval::from_i64(ziren_state.pc as i64),
        is_done: if ziren_state.pc == 0 {
            MayBeFlag::True
        } else {
            MayBeFlag::False
        }, // TODO || ziren_state.exited,
        memory_ops: memory_ops,
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
        is_done: abstract_row[5].clone().is_zero(prime),
        memory_ops: Vec::new(),
    }
}
