use std::{collections::HashMap, fmt};

use crate::interval::{AbstractInterval, MayBeFlag};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AbstractState {
    pub clk: AbstractInterval,
    pub pc: AbstractInterval,
    pub next_pc: AbstractInterval,
    pub memory: HashMap<u32, AbstractInterval>,
    pub is_done: MayBeFlag,
}

impl fmt::Display for AbstractState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(clk: {}, pc: {}, next_pc: {}, is_done: {:?})",
            self.clk, self.pc, self.next_pc, self.is_done
        )
    }
}
