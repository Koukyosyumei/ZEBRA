use std::collections::HashSet;

use latticevm::interval::AbstractInterval;
use latticevm::interval::MayBeFlag;
use latticevm::state::AbstractState;
use latticevm::state::MemoryOp;
use latticevm::state::MemoryOpKind;

fn reconstruct_word(row: &[AbstractInterval], base: usize) -> AbstractInterval {
    let mut val = AbstractInterval::from_i64(0);
    let mut mul = 1_i64;
    for i in 0..4 {
        val = val + row[base + i].clone() * AbstractInterval::from_i64(mul);
        mul *= 256;
    }
    val
}

pub fn valida_abstract_trace_to_abstract_state(
    abstract_row: &Vec<AbstractInterval>,
    prime: u32,
) -> AbstractState {
    let mut memory_ops = HashSet::new();

    if abstract_row[30].is_non_zero(prime) != MayBeFlag::False {
        memory_ops.insert(MemoryOp {
            kind: MemoryOpKind::Read,
            addr: abstract_row[31].clone(),
            value: reconstruct_word(abstract_row, 32),
        });
    }

    if abstract_row[36].is_non_zero(prime) != MayBeFlag::False {
        memory_ops.insert(MemoryOp {
            kind: MemoryOpKind::Read,
            addr: abstract_row[37].clone(),
            value: reconstruct_word(abstract_row, 38),
        });
    }

    if abstract_row[42].is_non_zero(prime) != MayBeFlag::False {
        memory_ops.insert(MemoryOp {
            kind: MemoryOpKind::Write,
            addr: abstract_row[43].clone(),
            value: reconstruct_word(abstract_row, 44),
        });
    }

    AbstractState {
        clk: abstract_row[0].clone(),
        pc: abstract_row[1].clone(),
        is_done: match abstract_row[24].clone().is_zero(prime) {
            MayBeFlag::True => MayBeFlag::False,
            MayBeFlag::False => MayBeFlag::True,
            MayBeFlag::MayBe => MayBeFlag::MayBe,
        },
        memory_ops,
    }
}
