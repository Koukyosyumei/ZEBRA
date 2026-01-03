use std::collections::HashSet;

use valida_machine::symbolic::symbolic_builder::get_symbolic_constraints;
use valida_machine::symbolic::symbolic_expression::SymbolicExpression;
use valida_machine::symbolic::symbolic_variable::{SymbolicVariable, Trace};
use valida_machine::ChipWithPersistence;
use valida_machine::Machine;
use valida_machine::StarkConfig;

use p3_field::PrimeField32;

use latticevm::symbolic::gather_boolean_variables;
use latticevm::symbolic::LatticeVMConstraints;
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

pub fn get_converted_symbolicconstraints<M, SC, C>(
    machine: &M,
    chip: &C,
) -> (LatticeVMConstraints, Vec<usize>)
where
    M: Machine<SC::Val>,
    SC: StarkConfig,
    C: ChipWithPersistence<M, SC>,
{
    let symbolic_constraints = get_symbolic_constraints::<M, SC, C>(&machine, &chip);
    let tv_constraints = symbolic_constraints
        .iter()
        .map(|sc| convert_p3_expr::<SC::Val>(&sc))
        .collect::<Vec<_>>();
    let multiplicities = HashSet::new();
    let potential_boolean_vars = gather_boolean_variables(&tv_constraints, &multiplicities);
    let constraints = LatticeVMConstraints {
        air_constraints: tv_constraints.clone(),
        pv_pos_constraints: vec![],
        pv_neg_constraints: vec![],
    };

    (constraints, potential_boolean_vars)
}
