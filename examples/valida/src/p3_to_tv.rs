use std::collections::HashSet;

use p3_air::PairCol;
use p3_air::VirtualPairCol;
use p3_field::PrimeField32;

use valida_machine::symbolic::symbolic_builder::get_symbolic_constraints;
use valida_machine::symbolic::symbolic_expression::SymbolicExpression;
use valida_machine::symbolic::symbolic_variable::{SymbolicVariable, Trace};
use valida_machine::ChipWithPersistence;
use valida_machine::Machine;
use valida_machine::StarkConfig;

use latticevm::impl_p3_to_tv_conversion;
use latticevm::symbolic::gather_boolean_variables;
use latticevm::symbolic::LatticeVMConstraints;
use latticevm::{
    interval::AbstractInterval,
    symbolic::{LatticeVMSymbolicEntry, LatticeVMSymbolicExpr, LatticeVMSymbolicVal},
};

pub fn convert_valida_variable<F: PrimeField32>(var: &SymbolicVariable<F>) -> LatticeVMSymbolicVal {
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

impl_p3_to_tv_conversion!(
    valida_machine::symbolic::symbolic_expression::SymbolicExpression, // Expr型
    valida_machine::symbolic::symbolic_variable::SymbolicVariable,     // Var型
    p3_air,
    p3_field,
    convert_valida_variable
);

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
