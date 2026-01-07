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
    p3_air::PairCol,
    p3_air::VirtualPairCol,
    p3_field::PrimeField32,
    p3_air,
    p3_field,
    convert_valida_variable
);
