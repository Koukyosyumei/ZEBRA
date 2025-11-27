use std::collections::HashMap;


use latticevm::interval::AbstractInterval;
use latticevm::interval::MayBeFlag;
use latticevm::state::AbstractState;

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
        is_done: match abstract_row[24].clone().is_zero(prime) {
            MayBeFlag::True => MayBeFlag::False,
            MayBeFlag::False => MayBeFlag::True,
            MayBeFlag::MayBe => MayBeFlag::MayBe,
        },
        memory,
    }
}
