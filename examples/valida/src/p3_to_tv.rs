use p3_air::PairCol;
use p3_field::PrimeField32;

use valida_machine::symbolic::symbolic_variable::{SymbolicVariable, Trace};

use zebra::impl_p3_to_tv_conversion;
use zebra::{
    interval::AbstractInterval,
    symbolic::{ZEBRASymbolicEntry, ZEBRASymbolicExpr, ZEBRASymbolicVal},
};

pub fn convert_valida_variable<F: PrimeField32>(var: &SymbolicVariable<F>) -> ZEBRASymbolicVal {
    match var.trace {
        Trace::Preprocessed => ZEBRASymbolicVal {
            entry: ZEBRASymbolicEntry::Main {
                is_curr: !var.is_next,
            },
            index: var.column,
        },
        Trace::Permutation => ZEBRASymbolicVal {
            entry: ZEBRASymbolicEntry::Main {
                is_curr: !var.is_next,
            },
            index: var.column,
        },
        Trace::Main => ZEBRASymbolicVal {
            entry: ZEBRASymbolicEntry::Main {
                is_curr: !var.is_next,
            },
            index: var.column,
        },
        Trace::Public => ZEBRASymbolicVal {
            entry: ZEBRASymbolicEntry::Public,
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
    convert_valida_variable,
    F::one()
);
