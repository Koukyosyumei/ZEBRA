use p3_air::PairCol;
use p3_uni_stark::{Entry, SymbolicVariable};

use latticevm::impl_p3_to_tv_conversion;
use latticevm::{
    interval::AbstractInterval,
    symbolic::{LatticeVMSymbolicEntry, LatticeVMSymbolicExpr, LatticeVMSymbolicVal},
};

pub fn convert_p3_entry(entry: &Entry) -> LatticeVMSymbolicEntry {
    match entry {
        Entry::Preprocessed { offset: _offset } => todo!(),
        Entry::Main { offset } => LatticeVMSymbolicEntry::Main {
            is_curr: *offset == 0,
        },
        Entry::Permutation { offset: _offset } => todo!(),
        Entry::Public => LatticeVMSymbolicEntry::Public,
        Entry::Challenge => todo!(),
    }
}

pub fn convert_ziren_variable<F>(var: &SymbolicVariable<F>) -> LatticeVMSymbolicVal {
    LatticeVMSymbolicVal {
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
    convert_ziren_variable,
    F::one()
);
