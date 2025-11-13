use valida_machine::symbolic::symbolic_expression::SymbolicExpression;
use valida_machine::symbolic::symbolic_variable::{SymbolicVariable, Trace};

use p3_field::PrimeField32;

use latticevm::{
    interval::AbstractInterval,
    symbolic::{LatticeVMSymbolicEntry, LatticeVMSymbolicExpr, LatticeVMSymbolicVal},
};

pub fn convert_p3_variable<F: PrimeField32>(var: &SymbolicVariable<F>) -> LatticeVMSymbolicVal {
    match var.trace {
        Trace::Preprocessed => LatticeVMSymbolicVal {
            entry: LatticeVMSymbolicEntry::Main {
                is_curr: !var.is_next,
            },
            index: var.column,
        },
        Trace::Permutation => LatticeVMSymbolicVal {
            entry: LatticeVMSymbolicEntry::Main {
                is_curr: !var.is_next,
            },
            index: var.column,
        },
        Trace::Main => LatticeVMSymbolicVal {
            entry: LatticeVMSymbolicEntry::Main {
                is_curr: !var.is_next,
            },
            index: var.column,
        },
        Trace::Public => LatticeVMSymbolicVal {
            entry: LatticeVMSymbolicEntry::Public,
            index: var.column,
        },
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
