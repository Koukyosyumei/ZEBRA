use std::collections::HashMap;

use zkm_core_executor::ExecutionState;
use zkm_stark::MachineProver;

use latticevm::interval::AbstractInterval;
use latticevm::interval::MayBeFlag;
use latticevm::state::AbstractState;

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
