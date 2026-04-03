use openvm_stark_backend::air_builders::symbolic::{
    symbolic_expression::SymbolicExpression, symbolic_variable::Entry,
    symbolic_variable::SymbolicVariable,
};
use openvm_stark_backend::p3_field::PrimeField32;

use zebra::interval::AbstractInterval;
use zebra::symbolic::{ZEBRASymbolicEntry, ZEBRASymbolicExpr, ZEBRASymbolicVal};

/// Convert an OpenVM `Entry` to a ZEBRA `ZEBRASymbolicEntry`.
///
/// OpenVM's `Entry::Main` carries a `part_index` (which partition of the main
/// trace) and an `offset` (0 = current row, 1 = next row). For a single-part
/// trace we treat `part_index = 0` as the only partition.
pub fn convert_openvm_entry(entry: &Entry) -> ZEBRASymbolicEntry {
    match entry {
        Entry::Main { offset, .. } => ZEBRASymbolicEntry::Main {
            is_curr: *offset == 0,
        },
        Entry::Public => ZEBRASymbolicEntry::Public,
        Entry::Preprocessed { .. } => todo!("Preprocessed columns not yet supported"),
        Entry::Permutation { offset } => ZEBRASymbolicEntry::Permutation {
            is_curr: *offset == 0,
        },
        // Challenge and Exposed values arise from the FRI logup permutation
        // phase.  We map them to a sentinel permutation entry so they are
        // filtered out later by `contains_permutation`.
        Entry::Challenge | Entry::Exposed => ZEBRASymbolicEntry::Permutation { is_curr: true },
    }
}

/// Convert an OpenVM `SymbolicVariable` to a ZEBRA `ZEBRASymbolicVal`.
pub fn convert_openvm_variable<F>(var: &SymbolicVariable<F>) -> ZEBRASymbolicVal {
    ZEBRASymbolicVal {
        entry: convert_openvm_entry(&var.entry),
        index: var.index,
    }
}

/// Recursively convert an OpenVM `SymbolicExpression<F>` to a
/// `ZEBRASymbolicExpr`.  The two types share the same tree structure; only
/// the leaf `Variable` and `Constant` nodes need field-specific handling.
pub fn convert_openvm_expr<F: PrimeField32>(expr: &SymbolicExpression<F>) -> ZEBRASymbolicExpr {
    match expr {
        SymbolicExpression::Variable(v) => {
            ZEBRASymbolicExpr::Variable(convert_openvm_variable(v))
        }
        SymbolicExpression::IsFirstRow => ZEBRASymbolicExpr::IsFirstRow,
        SymbolicExpression::IsLastRow => ZEBRASymbolicExpr::IsLastRow,
        SymbolicExpression::IsTransition => ZEBRASymbolicExpr::IsTransition,
        SymbolicExpression::Constant(v) => ZEBRASymbolicExpr::Constant(AbstractInterval {
            lo: v.as_canonical_u32() as i128,
            hi: v.as_canonical_u32() as i128,
        }),
        SymbolicExpression::Add { x, y, .. } => ZEBRASymbolicExpr::Add(
            Box::new(convert_openvm_expr(x)),
            Box::new(convert_openvm_expr(y)),
        ),
        SymbolicExpression::Sub { x, y, .. } => ZEBRASymbolicExpr::Sub(
            Box::new(convert_openvm_expr(x)),
            Box::new(convert_openvm_expr(y)),
        ),
        SymbolicExpression::Neg { x, .. } => {
            ZEBRASymbolicExpr::Neg(Box::new(convert_openvm_expr(x)))
        }
        SymbolicExpression::Mul { x, y, .. } => ZEBRASymbolicExpr::Mul(
            Box::new(convert_openvm_expr(x)),
            Box::new(convert_openvm_expr(y)),
        ),
    }
}
