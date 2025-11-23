use std::{collections::HashMap, fmt};

use crate::interval::{AbstractInterval, MayBeFlag};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AbstractState {
    pub clk: AbstractInterval,
    pub pc: AbstractInterval,
    pub memory: HashMap<u32, AbstractInterval>,
    pub is_done: MayBeFlag,
}

impl fmt::Display for AbstractState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut vec: Vec<(u32, AbstractInterval)> = self.memory.clone().into_iter().collect();
        vec.sort_by_key(|(k, _)| *k);
        let memory_str = vec
            .iter()
            .map(|(k, v)| format!("{:?}: {}", k, v))
            .collect::<Vec<_>>()
            .join(", ");

        write!(
            f,
            "(clk: {}, pc: {}, is_done: {:?}, memory: [{:?}])",
            self.clk, self.pc, self.is_done, memory_str
        )
    }
}
