use p3_air::PairCol;
use p3_uni_stark::{Entry, SymbolicVariable};

use zebra::impl_p3_to_tv_conversion;
use zebra::{
    interval::AbstractInterval,
    symbolic::{ZEBRASymbolicEntry, ZEBRASymbolicExpr, ZEBRASymbolicVal},
};

pub fn convert_p3_entry(entry: &Entry) -> ZEBRASymbolicEntry {
    match entry {
        Entry::Preprocessed { offset: _offset } => todo!(),
        Entry::Main { offset } => ZEBRASymbolicEntry::Main {
            is_curr: *offset == 0,
        },
        Entry::Permutation { offset: _offset } => todo!(),
        Entry::Public => ZEBRASymbolicEntry::Public,
        Entry::Challenge => todo!(),
    }
}

pub fn convert_pico_variable<F>(var: &SymbolicVariable<F>) -> ZEBRASymbolicVal {
    ZEBRASymbolicVal {
        entry: convert_p3_entry(&var.entry),
        index: var.index,
    }
}

impl_p3_to_tv_conversion!(
    p3_uni_stark::SymbolicExpression, // Expr型
    p3_uni_stark::SymbolicVariable,   // Var型
    p3_air::PairCol,
    p3_air::VirtualPairCol,
    p3_field::PrimeField32,
    convert_pico_variable,
    F::ONE
);
