use std::collections::HashMap;

use latticevm::interval::AbstractInterval;
use latticevm::interval::MayBeFlag;
use latticevm::state::AbstractState;

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
