use openvm_stark_backend::air_builders::symbolic::{
    get_symbolic_builder, symbolic_expression::SymbolicExpression, symbolic_variable::Entry,
    symbolic_variable::SymbolicVariable, SymbolicConstraints, SymbolicRapBuilder,
};
use openvm_stark_backend::interaction::RapPhaseSeqKind;
use openvm_stark_backend::keygen::types::TraceWidth;
use openvm_stark_backend::p3_field::PrimeField32;
use openvm_stark_backend::rap::{BaseAirWithPublicValues, PartitionedBaseAir, Rap};

use zebra::interval::AbstractInterval;
use zebra::symbolic::{ZEBRASymbolicEntry, ZEBRASymbolicExpr, ZEBRASymbolicVal};

/// Returns `true` if `expr` involves any permutation, challenge, or exposed
/// column.  Such expressions arise from the FRI logup auxiliary columns and
/// should be dropped when extracting polynomial constraints for ZEBRA.
pub fn contains_permutation<F>(expr: &SymbolicExpression<F>) -> bool {
    match expr {
        SymbolicExpression::Variable(v) => matches!(
            v.entry,
            Entry::Permutation { .. } | Entry::Challenge | Entry::Exposed
        ),
        SymbolicExpression::IsFirstRow
        | SymbolicExpression::IsLastRow
        | SymbolicExpression::IsTransition
        | SymbolicExpression::Constant(_) => false,
        SymbolicExpression::Add { x, y, .. } | SymbolicExpression::Sub { x, y, .. } | SymbolicExpression::Mul { x, y, .. } => {
            contains_permutation(x) || contains_permutation(y)
        }
        SymbolicExpression::Neg { x, .. } => contains_permutation(x),
    }
}

/// If `expr` is a simple current-row main-trace variable, return its column
/// index.  Returns `None` for anything more complex.
pub fn as_main_col<F>(expr: &SymbolicExpression<F>) -> Option<usize> {
    if let SymbolicExpression::Variable(v) = expr {
        if let Entry::Main { offset: 0, .. } = v.entry {
            return Some(v.index);
        }
    }
    None
}

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

/// Evaluate the chip's AIR symbolically and return the raw `SymbolicConstraints`.
///
/// The caller can then filter out FRI-logup permutation constraints with
/// [`contains_permutation`] and convert the remaining polynomial constraints
/// with [`convert_openvm_expr`].
///
/// The `after_challenge` width and challenge randomness are auto-populated by
/// [`SymbolicRapBuilder::finalize_interactions`] when there are interactions, so
/// passing empty slices here is correct.
pub fn get_symbolic_constraints_openvm<F, R>(rap: &R) -> SymbolicConstraints<F>
where
    F: PrimeField32,
    R: Rap<SymbolicRapBuilder<F>> + BaseAirWithPublicValues<F> + PartitionedBaseAir<F>,
{
    let width = TraceWidth {
        preprocessed: None,
        cached_mains: rap.cached_main_widths(),
        common_main: rap.common_main_width(),
        after_challenge: vec![],
    };
    get_symbolic_builder(rap, &width, &[], &[], RapPhaseSeqKind::FriLogUp, 0)
        .constraints()
}
