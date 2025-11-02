use std::collections::HashMap;

use crate::interval::{AbstractInterval, MayBeFlag};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AbstractState {
    pub clk: AbstractInterval,
    pub pc: AbstractInterval,
    pub next_pc: AbstractInterval,
    pub memory: HashMap<u32, AbstractInterval>,
    pub is_done: MayBeFlag,
}
