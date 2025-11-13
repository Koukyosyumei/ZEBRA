use std::collections::HashMap;

use valida_basic_api::machine::basic::ValidaSimpleState;

use latticevm::interval::AbstractInterval;
use latticevm::interval::MayBeFlag;
use latticevm::state::AbstractState;

pub fn valida_state_to_abstract_state(
    valida_state: &ValidaSimpleState,
    valida_next_state: &Option<ValidaSimpleState>,
) -> AbstractState {
    let memory = valida_state
        .memory
        .clone()
        .into_iter()
        .map(|(addr, record)| {
            (
                addr,
                AbstractInterval::from_i64(
                    record.0.into_iter().enumerate().fold(0, |acc, (n, item)| {
                        acc + (item as i64) * (1 << (8 * n as i64)) as i64
                    }),
                ),
            )
        })
        .collect();

    let is_done = if let Some(state) = valida_next_state {
        state.is_done
    } else {
        false
    };

    AbstractState {
        clk: AbstractInterval::from_i64(valida_state.clk as i64),
        pc: AbstractInterval::from_i64(valida_state.pc as i64),
        next_pc: AbstractInterval::from_i64(valida_state.pc as i64),
        is_done: if is_done {
            MayBeFlag::True
        } else {
            MayBeFlag::False
        }, // TODO || ziren_state.exited,
        memory: memory,
    }
}

pub fn valida_abstract_trace_to_abstract_state(
    abstract_row: &Vec<AbstractInterval>,
    prime: u32,
) -> AbstractState {
    AbstractState {
        clk: abstract_row[0].clone(),
        pc: abstract_row[1].clone(),
        next_pc: abstract_row[1].clone(),
        is_done: match abstract_row[24].clone().is_zero(prime) {
            MayBeFlag::True => MayBeFlag::False,
            MayBeFlag::False => MayBeFlag::True,
            MayBeFlag::MayBe => MayBeFlag::MayBe,
        },
        memory: HashMap::new(),
    }
}
