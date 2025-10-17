use std::rc::Rc;

use p3_field::PrimeField32;
use p3_uni_stark::{Entry, SymbolicExpression, SymbolicVariable};

use crate::{
    interval::AbstractInterval,
    symbolic::{TwinVMSymbolicEntry, TwinVMSymbolicExpr, TwinVMSymbolicVal},
};

pub fn convert_p3_entry(entry: &Entry) -> TwinVMSymbolicEntry {
    match entry {
        Entry::Preprocessed { offset: _offset } => todo!(),
        Entry::Main { offset } => TwinVMSymbolicEntry::Main {
            is_curr: *offset == 0,
        },
        Entry::Permutation { offset: _offset } => todo!(),
        Entry::Public => TwinVMSymbolicEntry::Public,
        Entry::Challenge => todo!(),
    }
}

pub fn convert_p3_variable<F>(var: &SymbolicVariable<F>) -> TwinVMSymbolicVal {
    TwinVMSymbolicVal {
        entry: convert_p3_entry(&var.entry),
        index: var.index,
    }
}

pub fn convert_p3_expr<F: PrimeField32>(expr: &SymbolicExpression<F>) -> TwinVMSymbolicExpr {
    match expr {
        SymbolicExpression::Variable(symbolic_variable) => {
            TwinVMSymbolicExpr::Variable(convert_p3_variable(symbolic_variable))
        }
        SymbolicExpression::IsFirstRow => TwinVMSymbolicExpr::IsFirstRow,
        SymbolicExpression::IsLastRow => TwinVMSymbolicExpr::IsLastRow,
        SymbolicExpression::IsTransition => TwinVMSymbolicExpr::IsTransition,
        SymbolicExpression::Constant(v) => {
            TwinVMSymbolicExpr::Constant(AbstractInterval::from_f(v))
        }
        SymbolicExpression::Add {
            x,
            y,
            degree_multiple: _degree_multiple,
        } => TwinVMSymbolicExpr::Add(Rc::new(convert_p3_expr(x)), Rc::new(convert_p3_expr(y))),
        SymbolicExpression::Sub {
            x,
            y,
            degree_multiple: _degree_multiple,
        } => TwinVMSymbolicExpr::Sub(Rc::new(convert_p3_expr(x)), Rc::new(convert_p3_expr(y))),
        SymbolicExpression::Neg {
            x,
            degree_multiple: _degree_multiple,
        } => TwinVMSymbolicExpr::Neg(Rc::new(convert_p3_expr(x))),
        SymbolicExpression::Mul {
            x,
            y,
            degree_multiple: _degree_multiple,
        } => TwinVMSymbolicExpr::Mul(Rc::new(convert_p3_expr(x)), Rc::new(convert_p3_expr(y))),
    }
}
