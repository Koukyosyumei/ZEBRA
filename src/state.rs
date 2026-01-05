use std::{collections::HashSet, fmt};

use crate::interval::{AbstractInterval, MayBeFlag};

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum MemoryOpKind {
    Read,
    Write,
}

impl fmt::Display for MemoryOpKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryOpKind::Read => {
                write!(f, "Read")
            }
            MemoryOpKind::Write => {
                write!(f, "Write")
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct MemoryOp {
    pub kind: MemoryOpKind,
    pub addr: AbstractInterval,
    pub value: AbstractInterval,
}

impl fmt::Display for MemoryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({}: addr: {}, value: {})",
            self.kind, self.addr, self.value
        )
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AbstractState {
    pub clk: AbstractInterval,
    pub pc: AbstractInterval,
    pub memory_ops: Vec<MemoryOp>,
    pub is_done: MayBeFlag,
}

impl fmt::Display for AbstractState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut memory_str = self
            .memory_ops
            .iter()
            .map(|m| format!("{}", m).to_string())
            .collect::<Vec<String>>();
        memory_str.sort();
        write!(
            f,
            "(clk: {}, pc: {}, is_done: {:?}, memory: [{}])",
            self.clk,
            self.pc,
            self.is_done,
            memory_str.join(",")
        )
    }
}
