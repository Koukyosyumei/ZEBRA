use std::rc::Rc;

use p3_field::PrimeCharacteristicRing;
use p3_uni_stark::{
    get_symbolic_constraints, prove, verify, Entry, StarkConfig, SymbolicAirBuilder,
    SymbolicExpression, SymbolicVariable,
};

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

pub fn convert_p3_expr<F: PrimeCharacteristicRing>(
    expr: &SymbolicExpression<F>,
) -> TwinVMSymbolicExpr<F> {
    match expr {
        SymbolicExpression::Variable(symbolic_variable) => {
            TwinVMSymbolicExpr::Variable(convert_p3_variable(symbolic_variable))
        }
        SymbolicExpression::IsFirstRow => TwinVMSymbolicExpr::<F>::IsFirstRow,
        SymbolicExpression::IsLastRow => TwinVMSymbolicExpr::<F>::IsLastRow,
        SymbolicExpression::IsTransition => TwinVMSymbolicExpr::<F>::IsTransition,
        SymbolicExpression::Constant(v) => {
            TwinVMSymbolicExpr::<F>::Constant(AbstractInterval::from_f(v))
        }
        SymbolicExpression::Add {
            x,
            y,
            degree_multiple: _degree_multiple,
        } => TwinVMSymbolicExpr::<F>::Add(Rc::new(convert_p3_expr(x)), Rc::new(convert_p3_expr(y))),
        SymbolicExpression::Sub {
            x,
            y,
            degree_multiple: _degree_multiple,
        } => TwinVMSymbolicExpr::<F>::Sub(Rc::new(convert_p3_expr(x)), Rc::new(convert_p3_expr(y))),
        SymbolicExpression::Neg {
            x,
            degree_multiple: _degree_multiple,
        } => TwinVMSymbolicExpr::<F>::Neg(Rc::new(convert_p3_expr(x))),
        SymbolicExpression::Mul {
            x,
            y,
            degree_multiple: _degree_multiple,
        } => TwinVMSymbolicExpr::<F>::Mul(Rc::new(convert_p3_expr(x)), Rc::new(convert_p3_expr(y))),
    }
}
