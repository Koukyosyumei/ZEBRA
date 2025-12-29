use num_traits::{One, Signed, Zero};

use p3_air::{PairCol, VirtualPairCol};
use p3_field::Field;
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

pub fn convert_p3_paircol(pair_col: &PairCol) -> LatticeVMSymbolicVal {
    match pair_col {
        PairCol::Preprocessed(_idx) => todo!(),
        PairCol::Main(index) => LatticeVMSymbolicVal {
            entry: LatticeVMSymbolicEntry::Main { is_curr: true },
            index: *index,
        },
    }
}

fn get_weighted_var<F: PrimeField32>(paircol: &PairCol, weight: &F) -> LatticeVMSymbolicExpr {
    //if Field::is_one(weight) {
    //LatticeVMSymbolicExpr::Variable(convert_p3_paircol(paircol))
    //} else {
    LatticeVMSymbolicExpr::Mul(
        Box::new(LatticeVMSymbolicExpr::Variable(convert_p3_paircol(paircol))),
        Box::new(LatticeVMSymbolicExpr::Constant(AbstractInterval::from_i64(
            weight.as_canonical_u32() as i64,
        ))),
    )
    //}
}

pub fn convert_p3_virtual_pair_col<F: PrimeField32>(
    vpair: &VirtualPairCol<F>,
) -> LatticeVMSymbolicExpr {
    if vpair.column_weights.is_empty() {
        LatticeVMSymbolicExpr::Constant(AbstractInterval {
            lo: vpair.constant.as_canonical_u32() as i64,
            hi: vpair.constant.as_canonical_u32() as i64,
        })
    } else {
        let mut expr = LatticeVMSymbolicExpr::Constant(AbstractInterval {
            lo: vpair.constant.as_canonical_u32() as i64,
            hi: vpair.constant.as_canonical_u32() as i64,
        }); // get_weighted_var(&vpair.column_weights[0].0, &vpair.column_weights[0].1);
        for (paircol, w) in vpair.column_weights.iter() {
            if (w.clone() + F::ONE).as_canonical_u32() == 0 {
                expr = LatticeVMSymbolicExpr::Sub(
                    Box::new(expr.clone()),
                    Box::new(get_weighted_var(paircol, &F::ONE)),
                );
            } else {
                expr = LatticeVMSymbolicExpr::Add(
                    Box::new(expr.clone()),
                    Box::new(get_weighted_var(paircol, w)),
                );
            }
        }
        expr
    }
}
