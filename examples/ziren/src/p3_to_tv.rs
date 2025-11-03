use p3_field::PrimeField32;
use p3_uni_stark::{Entry, SymbolicExpression, SymbolicVariable};

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

pub fn convert_p3_variable<F>(var: &SymbolicVariable<F>) -> LatticeVMSymbolicVal {
    LatticeVMSymbolicVal {
        entry: convert_p3_entry(&var.entry),
        index: var.index,
    }
}

pub fn convert_p3_expr<F: PrimeField32>(expr: &SymbolicExpression<F>) -> LatticeVMSymbolicExpr {
    match expr {
        SymbolicExpression::Variable(symbolic_variable) => {
            LatticeVMSymbolicExpr::Variable(convert_p3_variable(symbolic_variable))
        }
        SymbolicExpression::IsFirstRow => LatticeVMSymbolicExpr::IsFirstRow,
        SymbolicExpression::IsLastRow => LatticeVMSymbolicExpr::IsLastRow,
        SymbolicExpression::IsTransition => LatticeVMSymbolicExpr::IsTransition,
        SymbolicExpression::Constant(v) => LatticeVMSymbolicExpr::Constant(AbstractInterval {
            lo: v.as_canonical_u32() as i64,
            hi: v.as_canonical_u32() as i64,
        }),
        SymbolicExpression::Add {
            x,
            y,
            degree_multiple: _degree_multiple,
        } => LatticeVMSymbolicExpr::Add(Box::new(convert_p3_expr(x)), Box::new(convert_p3_expr(y))),
        SymbolicExpression::Sub {
            x,
            y,
            degree_multiple: _degree_multiple,
        } => LatticeVMSymbolicExpr::Sub(Box::new(convert_p3_expr(x)), Box::new(convert_p3_expr(y))),
        SymbolicExpression::Neg {
            x,
            degree_multiple: _degree_multiple,
        } => LatticeVMSymbolicExpr::Neg(Box::new(convert_p3_expr(x))),
        SymbolicExpression::Mul {
            x,
            y,
            degree_multiple: _degree_multiple,
        } => LatticeVMSymbolicExpr::Mul(Box::new(convert_p3_expr(x)), Box::new(convert_p3_expr(y))),
    }
}
