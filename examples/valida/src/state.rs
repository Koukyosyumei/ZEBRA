use std::collections::HashMap;

use valida_basic_api::machine::basic::ValidaSimpleState;

use latticevm::interval::AbstractInterval;
use latticevm::interval::MayBeFlag;
use latticevm::state::AbstractState;

pub fn check_eq_valida(state_x: &AbstractState, state_y: &AbstractState, prime: u32) -> MayBeFlag {
    if (state_x.clk.clone() - state_y.clk.clone()).is_zero(prime) == MayBeFlag::False {
        return MayBeFlag::False;
    }
    if (state_x.pc.clone() - state_y.pc.clone()).is_zero(prime) == MayBeFlag::False {
        return MayBeFlag::False;
    }
    if state_x.is_done != state_y.is_done {
        return MayBeFlag::False;
    }

    if state_x.memory.len() != state_y.memory.len() {
        return MayBeFlag::False;
    }

    for (k, vx) in &state_x.memory {
        if let Some(vy) = state_y.memory.get(&k) {
            if (vx.clone() - vy.clone()).is_zero(prime) != MayBeFlag::True {
                return MayBeFlag::False;
            }
        }
    }

    MayBeFlag::True
}

pub fn check_eq_states(
    states_x: &[AbstractState],
    states_y: &[AbstractState],
    prime: u32,
) -> (MayBeFlag, usize) {
    for i in 0..(states_x.len() - 1) {
        if check_eq_valida(&states_x[i], &states_y[i], prime) == MayBeFlag::False {
            return (MayBeFlag::False, i);
        }
        if states_x[i].is_done != MayBeFlag::False {
            break;
        }
    }

    (MayBeFlag::True, 0)
}

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
    prev_memory: &HashMap<u32, AbstractInterval>,
    prime: u32,
) -> AbstractState {
    let mut memory = prev_memory.clone();
    if abstract_row[42].is_non_zero(prime) != MayBeFlag::False {
        let val = abstract_row[44].clone()
            + abstract_row[45].clone()
            + abstract_row[46].clone()
            + abstract_row[47].clone();
        memory.insert(abstract_row[43].as_canonical_u32(prime), val);
    }

    AbstractState {
        clk: abstract_row[0].clone(),
        pc: abstract_row[1].clone(),
        next_pc: abstract_row[1].clone(),
        is_done: match abstract_row[24].clone().is_zero(prime) {
            MayBeFlag::True => MayBeFlag::False,
            MayBeFlag::False => MayBeFlag::True,
            MayBeFlag::MayBe => MayBeFlag::MayBe,
        },
        memory,
    }
}
