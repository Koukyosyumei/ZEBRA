use zkm_stark::MachineProver;

use latticevm::interval::AbstractInterval;
use latticevm::state::AbstractState;

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
