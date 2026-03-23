use std::collections::HashSet;

use crate::interval::{AbstractInterval, MayBeFlag};
use crate::symbolic::{
    gather_vars_simple, get_curr_add_vars_or_zero, get_curr_add_vars_sub_const, get_curr_i,
    get_curr_i_or_zero, get_curr_i_sub_const, get_curr_i_sub_cur_j, get_curr_i_with_polarity,
    ZEBRASymbolicExpr,
};
use crate::trace::AbstractTrace;
use crate::wordop::word_addu;

/// Detects selector-gated constant assignment constraints.
///
/// Searches for constraints of the form:
///
/// ```text
/// selector * (curr[v] - target) = 0
/// ```
///
/// where `selector` and `v` are variables from the current row and `target` is a
/// constant value.
///
/// Such constraints express a conditional assignment:
///
/// > If `selector = 1`, then `curr[v] = target`.
///
/// # Parameters
///
/// * `constraints` — Symbolic constraints to analyze
/// * `prime` — Field modulus used for canonical constant interpretation
///
/// # Returns
///
/// A list of tuples:
///
/// ```text
/// (selector_index, value_index, target_constant)
/// ```
///
/// representing each detected conditional constraint.
///
/// # Detection Strategy
///
/// Matches multiplication nodes where one operand is a selector variable and
/// the other is a subtraction of a variable and a constant, regardless of order.
///
/// # Use Cases
///
/// * Interval refinement under conditional execution
/// * Symbolic simplification
/// * Constraint-driven search pruning
///
/// # See Also
///
/// * [`refine_conditional_constraints_var_sub_const`]
pub fn detect_conditional_var_sub_const_constraints(
    constraints: &[ZEBRASymbolicExpr],
    prime: u32,
) -> Vec<(usize, usize, i128)> {
    let mut const_constraints = Vec::new();

    for c in constraints {
        // Mul(selector, Sub(lhs, rhs))
        if let ZEBRASymbolicExpr::Mul(lhs_expr, rhs_expr) = c {
            // 1. the left is selector, and the right is Sub expr
            if let Some(s_idx) = get_curr_i(lhs_expr) {
                if let Some((v_idx, target)) = get_curr_i_sub_const(rhs_expr, prime) {
                    if s_idx != v_idx {
                        const_constraints.push((s_idx, v_idx, target));
                        continue;
                    }
                }
            }

            // 2. the left is Sub expr, and the right is the selector
            if let Some(s_idx) = get_curr_i(rhs_expr) {
                if let Some((v_idx, target)) = get_curr_i_sub_const(lhs_expr, prime) {
                    if s_idx != v_idx {
                        const_constraints.push((s_idx, v_idx, target));
                        continue;
                    }
                }
            }
        }
    }

    const_constraints
}

pub fn detect_conditional_addvars_sub_const_constraints(
    constraints: &[ZEBRASymbolicExpr],
    prime: u32,
) -> Vec<(usize, Vec<usize>, i128)> {
    let mut const_constraints = Vec::new();

    for c in constraints {
        // Mul(selector, Sub(lhs, rhs))
        if let ZEBRASymbolicExpr::Mul(lhs_expr, rhs_expr) = c {
            // 1. the left is selector, and the right is Sub expr
            if let Some(s_idx) = get_curr_i(lhs_expr) {
                if let Some((vs, target)) = get_curr_add_vars_sub_const(rhs_expr, prime) {
                    if !vs.contains(&s_idx) && !vs.is_empty() {
                        const_constraints.push((s_idx, vs, target));
                        continue;
                    }
                }
            }

            // 2. the left is Sub expr, and the right is the selector
            if let Some(s_idx) = get_curr_i(rhs_expr) {
                if let Some((vs, target)) = get_curr_add_vars_sub_const(lhs_expr, prime) {
                    if !vs.contains(&s_idx) && !vs.is_empty() {
                        const_constraints.push((s_idx, vs, target));
                        continue;
                    }
                }
            }
        }
    }

    const_constraints
}

/// Applies interval refinement for selector-controlled constant assignments.
///
/// Implements refinement for constraints of the form:
///
/// ```text
/// selector * (curr[v] - target) = 0
/// ```
///
/// If the selector is known to be `1`, the value variable is intersected with
/// the singleton interval `{target}`.
///
/// # Parameters
///
/// * `trace` — Abstract execution trace to refine
/// * `const_constraints` — Triples `(selector_idx, value_idx, target_constant)`
///
/// # Returns
///
/// * `MayBeFlag::False` — Constraint violation detected
/// * `MayBeFlag::MayBe` — Refinement applied successfully or inconclusive
///
/// # Behavior
///
/// For each row:
///
/// * If selector interval is exactly `{1}`:
///   * Restrict the value interval to `target`
///   * Fail if intersection is empty
///
/// Future improvements may infer `selector = 0` when the value cannot equal
/// the target.
///
/// # Use Cases
///
/// * Constraint propagation
/// * Search-space pruning
/// * Abstract interpretation
pub fn refine_conditional_constraints_var_sub_const(
    trace: &mut AbstractTrace,
    const_constraints: &[(usize, usize, i128)], // (selector_idx, value_idx, target_constant)
) -> MayBeFlag {
    let num_rows = trace.data.len();

    for r in 0..num_rows {
        for &(sel_idx, val_idx, target) in const_constraints {
            let selector = &trace.data[r][sel_idx];
            let target_interval = AbstractInterval::from_i128(target);

            // selector is one
            if selector.is_singleton() && selector.lo == 1 {
                let current_val = &trace.data[r][val_idx];
                if let Some(refined) = current_val.intersect(&target_interval) {
                    trace.data[r][val_idx] = refined;
                } else {
                    return MayBeFlag::False;
                }
            }

            // TODO: if the interval does not contain the target, the selector must be zero.
        }
    }

    MayBeFlag::MayBe
}

// detect_conditional_addvars_sub_const_constraints
pub fn refine_conditional_constraints_addvars_sub_const(
    trace: &mut AbstractTrace,
    const_constraints: &[(usize, Vec<usize>, i128)], // (selector_idx, value_idx, target_constant)
) -> MayBeFlag {
    let num_rows = trace.data.len();

    for r in 0..num_rows {
        for &(sel_idx, ref vs, target) in const_constraints {
            let selector = &trace.data[r][sel_idx];
            let target_interval = AbstractInterval::from_i128(target);

            // selector is one
            if selector.is_singleton() && selector.lo == 1 {
                let mut singletons = vec![];
                let mut non_singletons = vec![];
                for v_idx in vs {
                    if trace.data[r][*v_idx].is_singleton() {
                        singletons.push(v_idx.clone());
                    } else {
                        non_singletons.push(v_idx.clone());
                    }
                }
                if non_singletons.len() == 1 && singletons.len() >= 1 {
                    let mut ai = AbstractInterval::zero();
                    for v_idx in singletons {
                        ai = ai + trace.data[r][v_idx].clone();
                    }

                    let current_val = &trace.data[r][non_singletons[0]];
                    if let Some(refined) = current_val.intersect(&(target_interval - ai)) {
                        trace.data[r][non_singletons[0]] = refined;
                    } else {
                        return MayBeFlag::False;
                    }
                }
            }

            // TODO: if the interval does not contain the target, the selector must be zero.
        }
    }

    MayBeFlag::MayBe
}

/// Detects selector-gated equality constraints between variables.
///
/// Searches for constraints of the form:
///
/// ```text
/// selector * (curr[a] - curr[b]) = 0
/// ```
///
/// expressing:
///
/// > If `selector = 1`, then `curr[a] = curr[b]`.
///
/// # Parameters
///
/// * `constraints` — Symbolic constraints to analyze
///
/// # Returns
///
/// A list of triples:
///
/// ```text
/// (selector_index, a_index, b_index)
/// ```
///
/// representing detected conditional equalities.
///
/// # Use Cases
///
/// * Equality propagation
/// * Alias detection
/// * Interval tightening
///
/// # See Also
///
/// * [`refine_conditional_constraints_var_sub_var`]
pub fn detect_conditional_var_sub_var_constraints(
    constraints: &[ZEBRASymbolicExpr],
) -> Vec<(usize, usize, usize)> {
    let mut eq_constraints = Vec::new();

    for c in constraints {
        // Mul(selector, Sub(lhs, rhs))
        if let ZEBRASymbolicExpr::Mul(lhs_expr, rhs_expr) = c {
            // 1. the left is selector, and the right is Sub expr
            if let Some(s_idx) = get_curr_i(lhs_expr) {
                if let Some((a_idx, b_idx)) = get_curr_i_sub_cur_j(rhs_expr) {
                    eq_constraints.push((s_idx, a_idx, b_idx));
                    continue;
                }
            }

            // 2. the left is Sub expr, and the right is the selector
            if let Some(s_idx) = get_curr_i(rhs_expr) {
                if let Some((a_idx, b_idx)) = get_curr_i_sub_cur_j(lhs_expr) {
                    eq_constraints.push((s_idx, a_idx, b_idx));
                    continue;
                }
            }
        }
    }

    eq_constraints
}

/// Refines intervals using selector-controlled variable equality constraints.
///
/// Implements propagation for:
///
/// ```text
/// selector * (curr[a] - curr[b]) = 0
/// ```
///
/// If the selector is known to be `1`, both variables are intersected with each
/// other, enforcing equality.
///
/// # Parameters
///
/// * `trace` — Abstract trace to refine
/// * `eq_constraints` — Triples `(selector_idx, a_idx, b_idx)`
///
/// # Returns
///
/// * `MayBeFlag::False` — Inconsistent intervals detected
/// * `MayBeFlag::MayBe` — Refinement applied or inconclusive
///
/// # Behavior
///
/// For each row where selector equals `{1}`:
///
/// * Replace intervals of `a` and `b` with their intersection
/// * Fail if intersection is empty
///
/// # Use Cases
///
/// * Constraint propagation
/// * Equality reasoning
/// * Solver pruning
pub fn refine_conditional_constraints_var_sub_var(
    trace: &mut AbstractTrace,
    eq_constraints: &[(usize, usize, usize)], // (selector_idx, a_idx, b_idx)
) -> MayBeFlag {
    let num_rows = trace.data.len();

    for r in 0..num_rows {
        for &(sel_idx, a_idx, b_idx) in eq_constraints {
            let selector = &trace.data[r][sel_idx];

            if selector.is_singleton() && selector.lo == 1 {
                let a_val = trace.data[r][a_idx].clone();
                let b_val = trace.data[r][b_idx].clone();

                if let Some(intersected) = a_val.intersect(&b_val) {
                    trace.data[r][a_idx] = intersected.clone();
                    trace.data[r][b_idx] = intersected;
                } else {
                    return MayBeFlag::False;
                }
            }
            // TODO: if the intervals of a and b are disjoint, the selector must be zero.
        }
    }

    MayBeFlag::MayBe
}

/// Normalizes additive expressions by moving a subtraction term to the right.
///
/// Performs the rewrite:
///
/// ```text
/// (a - c) + b  →  (a + b) - c
/// ```
///
/// This canonicalization simplifies pattern matching for affine forms and is
/// particularly useful for detecting constraints of the form `L - d·e`.
///
/// # Parameters
///
/// * `expr` — Expression to normalize
///
/// # Returns
///
/// A transformed expression if the pattern matches, otherwise a clone of the
/// original expression.
///
/// # Use Cases
///
/// * Affine constraint detection
/// * Symbolic normalization
/// * Pattern matching for interval refinement
pub fn move_sub_expr_to_right(expr: &ZEBRASymbolicExpr) -> ZEBRASymbolicExpr {
    // (a - c) + b ==> (a + b) - c
    if let ZEBRASymbolicExpr::Add(lhs_1, rhs_1) = expr {
        if let ZEBRASymbolicExpr::Sub(lhs_2, rhs_2) = *lhs_1.clone() {
            return ZEBRASymbolicExpr::Sub(
                Box::new(ZEBRASymbolicExpr::Add(lhs_2, rhs_1.clone())),
                rhs_2,
            );
        }
    }

    expr.clone()
}

//   a = L - de
//   d \in [d, d]
//   a \in [0, d - 1]
//   L \in [l, h]
//   a, b \notin FV(L)
// ----------------------------------------
//   e \in [[(l - (d - 1)) / d], [h / d]]
//
// *Proof*
//   0 <= L - de <= d - 1
//   L - (d - 1) <= de <= L
//   (L - (d - 1) / d) <= e <= L / d
//   (l - (d - 1)) / d <= e <= h / d

//   **Generalized Version**
// Affine Backward Interval Refinement (ABIR)
//
// a = L - d e
// d ∈ ℕ, d > 0
// a ∈ [amin, amax]
// L ∈ [ℓ, h]
// a, e ∉ FV(L)
// ----------------------------------------
// e :=
// e ∧ [
//   ⌊(ℓ - amax) / d ,
//   ⌊(h - amin) / d
// ]

/// Represents an affine backward interval refinement (ABIR) constraint.
///
/// Models relations of the form:
///
/// ```text
/// a = L - d * e
/// ```
///
/// where:
///
/// * `a` — Left-hand side variable
/// * `L` — Affine symbolic expression independent of `a` and `e`
/// * `e` — Quotient variable
/// * `d` — Positive stride (integer coefficient)
///
/// Such constraints allow backward propagation of interval information from
/// `a` and `L` to refine the possible values of `e`.
///
/// # Fields
///
/// * `lhs_var` — Index of variable `a`
/// * `affine_rhs` — Expression representing `L`
/// * `quotient_var` — Index of variable `e`
/// * `stride` — Positive coefficient `d`
///
/// # Use Cases
///
/// * Division-like constraints
/// * Modular decompositions
/// * Bit/limb extraction reasoning
///
/// # See Also
///
/// * [`detect_abir_constraints`]
/// * [`apply_abir_refinement`]
#[derive(Debug, Clone)]
pub struct AbirConstraint {
    pub lhs_var: usize,                     // a
    pub affine_rhs: Box<ZEBRASymbolicExpr>, // L
    pub quotient_var: usize,                // e
    pub stride: u32,                        // d > 0
}

/// Detects affine backward interval refinement (ABIR) constraints.
///
/// Identifies constraints equivalent to:
///
/// ```text
/// a - (L - d * e) = 0
/// ```
///
/// which implies:
///
/// ```text
/// a = L - d * e
/// ```
///
/// Subject to the side condition that variables `a` and `e` do not occur in `L`.
///
/// # Parameters
///
/// * `constraints` — Symbolic constraints to analyze
/// * `prime` — Field modulus for constant interpretation
///
/// # Returns
///
/// A list of [`AbirConstraint`] objects describing detected affine relations.
///
/// # Detection Steps
///
/// 1. Match subtraction structure
/// 2. Normalize RHS to `L - d·e`
/// 3. Extract stride and quotient variable
/// 4. Verify independence conditions
///
/// # Use Cases
///
/// * Interval refinement for quotient variables
/// * Detecting arithmetic decompositions
/// * Constraint simplification
pub fn detect_abir_constraints(
    constraints: &[ZEBRASymbolicExpr],
    prime: u32,
) -> Vec<AbirConstraint> {
    let mut result = Vec::new();

    for constraint in constraints {
        // Expect: a - RHS
        let (lhs, rhs) = match constraint {
            ZEBRASymbolicExpr::Sub(lhs, rhs) => (lhs, rhs),
            _ => continue,
        };

        let lhs_var = match &**lhs {
            ZEBRASymbolicExpr::Variable(v) => v.index,
            _ => continue,
        };

        // Normalize RHS: (L - d*e)
        let normalized_rhs = move_sub_expr_to_right(rhs);

        let (affine_rhs, de_term) = match normalized_rhs {
            ZEBRASymbolicExpr::Sub(lhs, rhs) => (lhs, rhs),
            _ => continue,
        };

        // Match d * e
        let (quotient_var, stride) = match &*de_term {
            ZEBRASymbolicExpr::Mul(x, y) => match (&**x, &**y) {
                (ZEBRASymbolicExpr::Variable(v), ZEBRASymbolicExpr::Constant(k))
                | (ZEBRASymbolicExpr::Constant(k), ZEBRASymbolicExpr::Variable(v)) => {
                    (v.index, k.as_canonical_u32(prime))
                }
                _ => continue,
            },
            ZEBRASymbolicExpr::Variable(v) => (v.index, 1),
            _ => continue,
        };

        if stride == 0 {
            continue;
        }

        // Side condition: a, e ∉ FV(L)
        let mut free_vars = HashSet::new();
        gather_vars_simple(&affine_rhs, &mut free_vars);

        if free_vars.contains(&lhs_var) || free_vars.contains(&quotient_var) {
            continue;
        }

        result.push(AbirConstraint {
            lhs_var,
            affine_rhs: affine_rhs.clone(),
            quotient_var,
            stride,
        });
    }

    result
}

/// Applies affine backward interval refinement (ABIR) to an abstract trace.
///
/// For constraints of the form:
///
/// ```text
/// a = L - d * e
/// ```
///
/// this function refines the interval of `e` using bounds inferred from `a`
/// and the evaluated range of `L`.
///
/// # Parameters
///
/// * `trace` — Abstract trace to refine
/// * `constraints` — ABIR constraints to apply
/// * `prime` — Field modulus for evaluation
///
/// # Returns
///
/// A pair:
///
/// ```text
/// (MayBeFlag, refinement_log)
/// ```
///
/// where the log records interval updates applied to quotient variables.
///
/// # Behavior
///
/// For each step:
///
/// 1. Evaluate `L` under current intervals
/// 2. Compute inferred bounds for `e`
/// 3. Intersect with existing interval
/// 4. Fail if intersection is empty
///
/// # Use Cases
///
/// * Division reasoning
/// * Constraint propagation
/// * Solver pruning
pub fn apply_abir_refinement(
    trace: &mut AbstractTrace,
    constraints: &[AbirConstraint],
    prime: u32,
) -> (MayBeFlag, Vec<(AbstractInterval, AbstractInterval)>) {
    let mut logs = Vec::new();

    for constraint in constraints {
        let AbirConstraint {
            lhs_var,
            affine_rhs,
            quotient_var,
            stride,
        } = constraint;

        let num_steps = trace.data.len();

        for step in 0..num_steps {
            let (past, future) = trace.data.split_at_mut(step + 1);
            let row = &mut past[past.len() - 1];
            let next_row = future.first().map(|r| &r[..]);

            // Evaluate L
            let rhs_interval = affine_rhs.eval(
                row,
                next_row,
                None,
                step == 0,
                step + 1 < num_steps,
                step + 1 == num_steps,
                prime,
            );

            // Weak ABIR:
            //   e ∈ floor((L - a) / d)
            let inferred_e = (rhs_interval - row[*lhs_var].clone()).div_floor(*stride as i128);

            if let Some(refined) = row[*quotient_var].intersect(&inferred_e) {
                logs.push((row[*quotient_var].clone(), refined.clone()));
                row[*quotient_var] = refined;
            } else {
                return (MayBeFlag::False, logs);
            }
        }
    }

    (MayBeFlag::MayBe, logs)
}

// ─── Double-selector constraint detection and refinement ──────────────────────
//
// Handles constraints of the form:
//   (sel1_expr * sel2_expr) * body = 0
// where each selector expression is either a plain variable (fires when == 1)
// or its complement `1 - Var(s)` (fires when == 0).
//
// Tuple layout for single-value body:   (s1_idx, s1_inv, s2_idx, s2_inv, v_idx, target)
// Tuple layout for multi-value body:    (s1_idx, s1_inv, s2_idx, s2_inv, v_idxs, target)

fn double_sel_fires(row: &[AbstractInterval], s_idx: usize, s_inv: bool) -> bool {
    let sel = &row[s_idx];
    if sel.is_singleton() {
        if s_inv {
            sel.lo == 0
        } else {
            sel.lo == 1
        }
    } else {
        false
    }
}

/// Detects `(sel1 * sel2) * (curr[v] - target) = 0` patterns.
///
/// Returns tuples `(s1_idx, s1_inv, s2_idx, s2_inv, v_idx, target)`.
/// When `s_inv` is `true` the corresponding selector must equal `0`.
pub fn detect_double_sel_var_sub_const(
    constraints: &[ZEBRASymbolicExpr],
    prime: u32,
) -> Vec<(usize, bool, usize, bool, usize, i128)> {
    let mut result = Vec::new();
    for c in constraints {
        if let ZEBRASymbolicExpr::Mul(outer_lhs, outer_rhs) = c {
            // Try lhs = Mul(sel1, sel2), rhs = body
            for (sel_expr, body_expr) in [
                (outer_lhs.as_ref(), outer_rhs.as_ref()),
                (outer_rhs.as_ref(), outer_lhs.as_ref()),
            ] {
                if let ZEBRASymbolicExpr::Mul(s1_expr, s2_expr) = sel_expr {
                    if let (Some((s1_idx, s1_inv)), Some((s2_idx, s2_inv))) = (
                        get_curr_i_with_polarity(s1_expr),
                        get_curr_i_with_polarity(s2_expr),
                    ) {
                        if let Some((v_idx, target)) = get_curr_i_or_zero(body_expr, prime) {
                            if v_idx != s1_idx && v_idx != s2_idx {
                                result.push((s1_idx, s1_inv, s2_idx, s2_inv, v_idx, target));
                            }
                        }
                    }
                }
            }
        }
    }
    result
}

/// Detects `(sel1 * sel2) * (sum(curr[vs]) - target) = 0` patterns.
///
/// Returns tuples `(s1_idx, s1_inv, s2_idx, s2_inv, v_idxs, target)`.
pub fn detect_double_sel_addvars_sub_const(
    constraints: &[ZEBRASymbolicExpr],
    prime: u32,
) -> Vec<(usize, bool, usize, bool, Vec<usize>, i128)> {
    let mut result = Vec::new();
    for c in constraints {
        if let ZEBRASymbolicExpr::Mul(outer_lhs, outer_rhs) = c {
            for (sel_expr, body_expr) in [
                (outer_lhs.as_ref(), outer_rhs.as_ref()),
                (outer_rhs.as_ref(), outer_lhs.as_ref()),
            ] {
                if let ZEBRASymbolicExpr::Mul(s1_expr, s2_expr) = sel_expr {
                    if let (Some((s1_idx, s1_inv)), Some((s2_idx, s2_inv))) = (
                        get_curr_i_with_polarity(s1_expr),
                        get_curr_i_with_polarity(s2_expr),
                    ) {
                        if let Some((vs, target)) = get_curr_add_vars_or_zero(body_expr, prime) {
                            if vs.len() >= 2 && !vs.contains(&s1_idx) && !vs.contains(&s2_idx) {
                                result.push((s1_idx, s1_inv, s2_idx, s2_inv, vs, target));
                            }
                        }
                    }
                }
            }
        }
    }
    result
}

/// Applies interval refinement for double-selector single-variable constraints.
///
/// For each `(s1, s1_inv, s2, s2_inv, v, target)`: when both selectors satisfy
/// their polarity conditions (singleton 0 or 1), tightens `curr[v]` to `target`.
pub fn refine_double_sel_var_sub_const(
    trace: &mut AbstractTrace,
    constraints: &[(usize, bool, usize, bool, usize, i128)],
) -> MayBeFlag {
    for r in 0..trace.data.len() {
        for &(s1_idx, s1_inv, s2_idx, s2_inv, v_idx, target) in constraints {
            if double_sel_fires(&trace.data[r], s1_idx, s1_inv)
                && double_sel_fires(&trace.data[r], s2_idx, s2_inv)
            {
                let target_iv = AbstractInterval::from_i128(target);
                if let Some(refined) = trace.data[r][v_idx].intersect(&target_iv) {
                    trace.data[r][v_idx] = refined;
                } else {
                    return MayBeFlag::False;
                }
            }
        }
    }
    MayBeFlag::MayBe
}

// ─── Selector-gated WordAddU refinement ───────────────────────────────────────
//
// Handles constraints of the form:
//   selector * (a_word - WordAddU(b_limbs, c_limbs)) = 0
//
// When the selector fires (singleton = 1) and all limbs of b and c are
// singletons, the result of WordAddU(b, c) is a concrete u32.  The four byte
// limbs of that result are then intersected with the corresponding a limb
// variables, provided each a limb's current interval fits within u8 range.

/// Holds a detected `selector * (a_word - WordAddU(b, c)) = 0` constraint.
#[derive(Debug, Clone)]
pub struct SelectorAddUConstraint {
    /// Selector expression (fires when it evaluates to singleton `{1}`).
    pub selector_expr: Box<ZEBRASymbolicExpr>,
    /// Column indices for the four byte-limbs of a (LSB first).
    pub a_var_indices: [usize; 4],
    /// Symbolic expressions for the four byte-limbs of b.
    pub b_limb_exprs: [Box<ZEBRASymbolicExpr>; 4],
    /// Symbolic expressions for the four byte-limbs of c.
    pub c_limb_exprs: [Box<ZEBRASymbolicExpr>; 4],
}

/// Attempts to reduce an expression to a bare variable index when the
/// expression is semantically equivalent to `curr[i]` (coefficient 1).
///
/// Handles the two extra layers of wrapping that appear in practice:
///
/// * `Variable(i)`
/// * `Mul(Variable(i), Const(1))` / `Mul(Const(1), Variable(i))`
/// * `Add(Const(0), inner)` / `Add(inner, Const(0))` — recursively unwrapped
fn try_unwrap_unit_var(expr: &ZEBRASymbolicExpr) -> Option<usize> {
    match expr {
        ZEBRASymbolicExpr::Variable(v) => Some(v.index),
        ZEBRASymbolicExpr::Mul(lhs, rhs) => match (lhs.as_ref(), rhs.as_ref()) {
            (ZEBRASymbolicExpr::Variable(v), ZEBRASymbolicExpr::Constant(k))
            | (ZEBRASymbolicExpr::Constant(k), ZEBRASymbolicExpr::Variable(v))
                if k.is_singleton() && k.lo == 1 =>
            {
                Some(v.index)
            }
            _ => None,
        },
        ZEBRASymbolicExpr::Add(lhs, rhs) => {
            // Add(Const(0), inner) or Add(inner, Const(0))
            if let ZEBRASymbolicExpr::Constant(k) = lhs.as_ref() {
                if k.is_singleton() && k.lo == 0 {
                    return try_unwrap_unit_var(rhs);
                }
            }
            if let ZEBRASymbolicExpr::Constant(k) = rhs.as_ref() {
                if k.is_singleton() && k.lo == 0 {
                    return try_unwrap_unit_var(lhs);
                }
            }
            None
        }
        _ => None,
    }
}

/// Collects `(var_index, scale)` pairs from a word-reconstruction expression.
///
/// Handles the actual format produced in practice:
///
/// ```text
/// Mul(Add(Const(0), Mul(Variable(i), Const(1))), Const(scale))
/// ```
///
/// i.e. each limb term is `Mul(inner_expr, Const(scale))` where `inner_expr`
/// is any expression that [`try_unwrap_unit_var`] can reduce to a variable.
/// The terms are accumulated inside a left-leaning `Add` tree.
///
/// `clean` is set to `false` whenever an unexpected node is encountered
/// (non-zero constant, bare variable, or unrecognized Mul).  Callers that
/// require a pure word reconstruction should check this flag.
fn collect_scaled_vars_from_word(
    expr: &ZEBRASymbolicExpr,
    result: &mut Vec<(usize, i128)>,
    clean: &mut bool,
) {
    match expr {
        ZEBRASymbolicExpr::Add(lhs, rhs) => {
            collect_scaled_vars_from_word(lhs, result, clean);
            collect_scaled_vars_from_word(rhs, result, clean);
        }
        ZEBRASymbolicExpr::Mul(lhs, rhs) => {
            // Try Mul(inner, Const(scale)) and Mul(Const(scale), inner)
            for (inner, scale_expr) in [(lhs.as_ref(), rhs.as_ref()), (rhs.as_ref(), lhs.as_ref())]
            {
                if let ZEBRASymbolicExpr::Constant(k) = scale_expr {
                    if let Some(var_idx) = try_unwrap_unit_var(inner) {
                        result.push((var_idx, k.lo));
                        return;
                    }
                }
            }
            *clean = false; // unrecognised Mul pattern
        }
        ZEBRASymbolicExpr::Constant(k) => {
            // Only the implicit zero padding (Const(0)) is allowed; anything
            // else (e.g. the "+4" offset in `next_pc + 4`) taints the result.
            if !(k.is_singleton() && k.lo == 0) {
                *clean = false;
            }
        }
        _ => {
            *clean = false; // bare variable or other unexpected node
        }
    }
}

/// Tries to parse a word-reconstruction expression into four variable indices.
///
/// Handles the nested format:
/// `(((0 + ((0 + (curr[i] * 1)) * 1)) + ((0 + (curr[j] * 1)) * 256)) + ...)`
///
/// Returns `[a0_idx, a1_idx, a2_idx, a3_idx]` on success, or `None` if the
/// expression contains unexpected content (e.g. a non-zero constant addend).
fn extract_word_var_indices(expr: &ZEBRASymbolicExpr) -> Option<[usize; 4]> {
    let mut scaled: Vec<(usize, i128)> = Vec::new();
    let mut clean = true;
    collect_scaled_vars_from_word(expr, &mut scaled, &mut clean);

    if !clean || scaled.len() != 4 {
        return None;
    }

    scaled.sort_by_key(|&(_, s)| s);

    let expected = [1i128, 256, 65536, 16_777_216];
    let mut indices = [0usize; 4];
    for (i, &(var_idx, scale)) in scaled.iter().enumerate() {
        if scale != expected[i] {
            return None;
        }
        indices[i] = var_idx;
    }
    Some(indices)
}

/// Detects `selector_expr * (a_word - WordAddU(b_limbs, c_limbs)) = 0` constraints.
///
/// The selector may be any symbolic expression (e.g. as produced by
/// `make_impl_constraint`).  The side condition — that a's limb variables do
/// not appear free in the selector — is checked via [`gather_vars_simple`].
///
/// Returns a list of [`SelectorAddUConstraint`] values ready for refinement.
pub fn detect_selector_addu_constraints(
    constraints: &[ZEBRASymbolicExpr],
) -> Vec<SelectorAddUConstraint> {
    let mut result = Vec::new();

    for c in constraints {
        if let ZEBRASymbolicExpr::Mul(outer_lhs, outer_rhs) = c {
            for (sel_expr, body_expr) in [
                (outer_lhs.as_ref(), outer_rhs.as_ref()),
                (outer_rhs.as_ref(), outer_lhs.as_ref()),
            ] {
                let ZEBRASymbolicExpr::Sub(a_word_expr, word_op_expr) = body_expr else {
                    continue;
                };
                let ZEBRASymbolicExpr::WordAddU(b_arr, c_arr) = word_op_expr.as_ref() else {
                    continue;
                };
                let Some(a_var_indices) = extract_word_var_indices(a_word_expr) else {
                    continue;
                };

                // Side condition: a's limb variables must not appear free in the selector.
                let mut sel_vars = HashSet::new();
                gather_vars_simple(sel_expr, &mut sel_vars);
                if a_var_indices.iter().any(|idx| sel_vars.contains(idx)) {
                    continue;
                }

                result.push(SelectorAddUConstraint {
                    selector_expr: Box::new(sel_expr.clone()),
                    a_var_indices,
                    b_limb_exprs: b_arr.clone(),
                    c_limb_exprs: c_arr.clone(),
                });
                break; // matched one ordering; no need to try the other
            }
        }
    }

    result
}

/// Applies interval refinement for selector-gated `WordAddU` constraints.
///
/// For each constraint and each row, when:
///
/// * the selector expression evaluates to the singleton `{1}`,
/// * all four limbs of both b and c evaluate to singletons, and
/// * each limb of a satisfies `0 ≤ lo` and `hi ≤ 255` (proper u8 domain),
///
/// the function computes the exact 32-bit result of `WordAddU(b, c)`, decodes
/// it into four bytes, and intersects each byte with the corresponding a limb.
///
/// Returns `MayBeFlag::False` on contradiction, `MayBeFlag::MayBe` otherwise.
pub fn refine_selector_addu_constraints(
    trace: &mut AbstractTrace,
    constraints: &[SelectorAddUConstraint],
    prime: u32,
) -> MayBeFlag {
    let num_rows = trace.data.len();

    for r in 0..num_rows {
        for constraint in constraints {
            let next_row = if r + 1 < num_rows {
                Some(&trace.data[r + 1][..])
            } else {
                None
            };
            let row = &trace.data[r];

            // Evaluate the selector expression; proceed only when it fires (= {1}).
            let sel_val = constraint.selector_expr.eval(
                row,
                next_row,
                None,
                r == 0,
                r + 1 < num_rows,
                r + 1 == num_rows,
                prime,
            );
            if !sel_val.is_singleton() || sel_val.lo != 1 {
                continue;
            }

            // Evaluate b and c limbs.
            let b_word: [AbstractInterval; 4] = std::array::from_fn(|i| {
                constraint.b_limb_exprs[i].eval(
                    row,
                    next_row,
                    None,
                    r == 0,
                    r + 1 < num_rows,
                    r + 1 == num_rows,
                    prime,
                )
            });
            let c_word: [AbstractInterval; 4] = std::array::from_fn(|i| {
                constraint.c_limb_exprs[i].eval(
                    row,
                    next_row,
                    None,
                    r == 0,
                    r + 1 < num_rows,
                    r + 1 == num_rows,
                    prime,
                )
            });

            // b and c must all be singletons.
            if !b_word.iter().all(|x| x.is_singleton()) || !c_word.iter().all(|x| x.is_singleton())
            {
                continue;
            }

            // Each a limb must lie within the proper u8 domain [0, 255].
            let a_ok = constraint
                .a_var_indices
                .iter()
                .all(|&idx| trace.data[r][idx].lo >= 0 && trace.data[r][idx].hi <= 255);
            if !a_ok {
                continue;
            }

            // Compute the exact 32-bit result.
            let result = word_addu(&b_word, &c_word);
            if !result.is_singleton() {
                continue;
            }
            let target_u32 = result.lo as u32;

            // Refine each a limb to its exact byte.
            for i in 0..4 {
                let byte_val = ((target_u32 >> (8 * i)) & 0xFF) as i128;
                let target_iv = AbstractInterval::from_i128(byte_val);
                let a_idx = constraint.a_var_indices[i];
                if let Some(refined) = trace.data[r][a_idx].intersect(&target_iv) {
                    trace.data[r][a_idx] = refined;
                } else {
                    return MayBeFlag::False;
                }
            }
        }
    }

    MayBeFlag::MayBe
}

// ─── Selector-gated word-assignment refinement ────────────────────────────────
//
// Handles the general pattern:
//   sel_expr * (word_a - rhs_expr) = 0   (or Sub in the other direction)
//
// i.e. "when the selector fires, word_a equals rhs_expr".
//
// Unlike SelectorAddUConstraint, the RHS here can be any symbolic expression
// (e.g. next_pc_word + 4).  The refinement fires when:
//   1. sel_expr evaluates to singleton {1}
//   2. rhs_expr evaluates to a singleton (concrete u32)
//   3. every limb of word_a lies in [0, 255]
//
// Side conditions (checked at detection time):
//   • word_a's limb variables must not appear free in sel_expr
//   • word_a's limb variables must not appear free in rhs_expr
//     (avoids a circular evaluation where rhs depends on a)

/// Holds a detected `sel_expr * (word_a ± rhs_expr) = 0` constraint where
/// `word_a` is a 4-limb word reconstruction and `rhs_expr` is an arbitrary
/// expression that is expected to become a singleton at refinement time.
#[derive(Debug, Clone)]
pub struct SelectorWordAssignConstraint {
    /// Combined selector expression (fires when it evaluates to any non-zero singleton).
    pub selector_expr: Box<ZEBRASymbolicExpr>,
    /// Column indices for the four byte-limbs of `word_a` (LSB first).
    pub a_var_indices: [usize; 4],
    /// Expression that `word_a` must equal when the selector fires.
    pub rhs_expr: Box<ZEBRASymbolicExpr>,
}

/// Flattens a top-level `Mul` chain into its leaf factors.
///
/// Only descends into `Mul` nodes; stops at any other variant.
/// For example:
/// ```text
/// Mul(A, Mul(B, C))  →  [A, B, C]
/// ```
fn collect_mul_factors(expr: &ZEBRASymbolicExpr, factors: &mut Vec<ZEBRASymbolicExpr>) {
    if let ZEBRASymbolicExpr::Mul(lhs, rhs) = expr {
        collect_mul_factors(lhs, factors);
        collect_mul_factors(rhs, factors);
    } else {
        factors.push(expr.clone());
    }
}

/// Detects `(sel_factors…) * (word_a - rhs_expr) = 0` constraints.
///
/// Handles arbitrarily deep `Mul` nesting by first flattening the constraint
/// into its multiplicative factors, then searching every `Sub` factor for one
/// whose side can be parsed as a **pure** 4-limb word reconstruction (no
/// extra constant offsets).  All remaining factors — including scalar `Sub`s
/// like `curr[59]-1` — are recombined into the `selector_expr`.
///
/// Example matched:
/// ```text
/// Mul(sum_vars, Mul(curr[59]-1, Sub(Add(next_pc_word,4), next_next_pc_word)))
/// ```
/// Flattened factors: `[sum_vars, Sub(curr[59],1), Sub(Add(next_pc+4), next_next_pc)]`
///   → `Sub(curr[59],1)`: neither side is a pure word → skip
///   → `Sub(Add(next_pc+4), next_next_pc)`:
///       LHS has a +4 constant → not a pure word → skip
///       RHS = `next_next_pc` → pure 4-limb word → matched!
///   → selector = `Mul(sum_vars, Sub(curr[59],1))`
///
/// Side conditions:
/// * `word_a`'s limb variables must not appear free in `selector_expr`.
/// * `word_a`'s limb variables must not appear free in `rhs_expr`
///   (prevents circular evaluation during refinement).
pub fn detect_selector_word_assign_constraints(
    constraints: &[ZEBRASymbolicExpr],
) -> Vec<SelectorWordAssignConstraint> {
    let mut result = Vec::new();

    'outer: for c in constraints {
        // Flatten the top-level Mul chain into individual factors.
        let mut factors: Vec<ZEBRASymbolicExpr> = Vec::new();
        collect_mul_factors(c, &mut factors);
        if factors.len() < 2 {
            continue;
        }

        // Search every Sub factor for one whose side is a pure word reconstruction.
        for sub_idx in 0..factors.len() {
            let ZEBRASymbolicExpr::Sub(sub_lhs, sub_rhs) = &factors[sub_idx] else {
                continue;
            };

            for (a_side, rhs_side) in [
                (sub_lhs.as_ref(), sub_rhs.as_ref()),
                (sub_rhs.as_ref(), sub_lhs.as_ref()),
            ] {
                // Reject expressions that are not a pure word reconstruction
                // (e.g. Add(next_pc_word, Const(4)) fails because of the +4).
                let Some(a_var_indices) = extract_word_var_indices(a_side) else {
                    continue;
                };

                // Combine every other factor into the selector.
                let sel_expr = factors
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != sub_idx)
                    .map(|(_, f)| f.clone())
                    .reduce(|a, b| ZEBRASymbolicExpr::Mul(Box::new(a), Box::new(b)))
                    .unwrap(); // safe: at least one non-Sub factor remains

                let mut sel_vars = HashSet::new();
                gather_vars_simple(&sel_expr, &mut sel_vars);
                let mut rhs_vars = HashSet::new();
                gather_vars_simple(rhs_side, &mut rhs_vars);

                if a_var_indices
                    .iter()
                    .any(|idx| sel_vars.contains(idx) || rhs_vars.contains(idx))
                {
                    continue;
                }

                result.push(SelectorWordAssignConstraint {
                    selector_expr: Box::new(sel_expr),
                    a_var_indices,
                    rhs_expr: Box::new(rhs_side.clone()),
                });
                continue 'outer; // next top-level constraint
            }
        }
    }

    result
}

/// Applies interval refinement for selector-gated word-assignment constraints.
///
/// For each constraint and each row, when:
///
/// * the selector expression evaluates to a **non-zero singleton**
///   (i.e. the constraint definitely forces `word_a = rhs_expr`),
/// * `rhs_expr` evaluates to a concrete singleton, and
/// * every limb of `word_a` satisfies `0 ≤ lo` and `hi ≤ 255`,
///
/// the function decodes the singleton into four bytes and intersects each byte
/// with the corresponding limb of `word_a`.
///
/// The selector condition is **non-zero** (not specifically `1`) to handle
/// compound selectors like `Mul(sum_vars, curr[59]-1)` that evaluate to `-1`
/// when the constraint is active.
///
/// Returns `MayBeFlag::False` on contradiction, `MayBeFlag::MayBe` otherwise.
pub fn refine_selector_word_assign_constraints(
    trace: &mut AbstractTrace,
    constraints: &[SelectorWordAssignConstraint],
    prime: u32,
) -> MayBeFlag {
    let num_rows = trace.data.len();

    for r in 0..num_rows {
        for constraint in constraints {
            let next_row = if r + 1 < num_rows {
                Some(&trace.data[r + 1][..])
            } else {
                None
            };
            let row = &trace.data[r];

            // Selector must be a concrete non-zero value (constraint is enforced).
            let sel_val = constraint.selector_expr.eval(
                row,
                next_row,
                None,
                r == 0,
                r + 1 < num_rows,
                r + 1 == num_rows,
                prime,
            );
            if !sel_val.is_singleton() || sel_val.lo == 0 {
                continue;
            }

            // RHS must be a concrete singleton.
            let rhs_val = constraint.rhs_expr.eval(
                row,
                next_row,
                None,
                r == 0,
                r + 1 < num_rows,
                r + 1 == num_rows,
                prime,
            );
            if !rhs_val.is_singleton() {
                continue;
            }

            // Each a limb must lie within [0, 255].
            if !constraint
                .a_var_indices
                .iter()
                .all(|&idx| trace.data[r][idx].lo >= 0 && trace.data[r][idx].hi <= 255)
            {
                continue;
            }

            let target_u32 = rhs_val.lo as u32;

            // Refine each limb to its exact byte.
            for i in 0..4 {
                let byte_val = ((target_u32 >> (8 * i)) & 0xFF) as i128;
                let target_iv = AbstractInterval::from_i128(byte_val);
                let a_idx = constraint.a_var_indices[i];
                if let Some(refined) = trace.data[r][a_idx].intersect(&target_iv) {
                    trace.data[r][a_idx] = refined;
                } else {
                    return MayBeFlag::False;
                }
            }
        }
    }

    MayBeFlag::MayBe
}

/// Applies interval refinement for double-selector sum-of-variables constraints.
///
/// For each `(s1, s1_inv, s2, s2_inv, vs, target)`: when both selectors fire,
/// and all but one of the `vs` are singletons, tightens the remaining variable.
pub fn refine_double_sel_addvars_sub_const(
    trace: &mut AbstractTrace,
    constraints: &[(usize, bool, usize, bool, Vec<usize>, i128)],
) -> MayBeFlag {
    for r in 0..trace.data.len() {
        for (s1_idx, s1_inv, s2_idx, s2_inv, vs, target) in constraints {
            if double_sel_fires(&trace.data[r], *s1_idx, *s1_inv)
                && double_sel_fires(&trace.data[r], *s2_idx, *s2_inv)
            {
                let target_iv = AbstractInterval::from_i128(*target);
                let mut singletons = vec![];
                let mut non_singletons = vec![];
                for &v_idx in vs {
                    if trace.data[r][v_idx].is_singleton() {
                        singletons.push(v_idx);
                    } else {
                        non_singletons.push(v_idx);
                    }
                }
                if non_singletons.len() == 1 && !singletons.is_empty() {
                    let mut sum = AbstractInterval::zero();
                    for v_idx in &singletons {
                        sum = sum + trace.data[r][*v_idx].clone();
                    }
                    let remainder = target_iv - sum;
                    if let Some(refined) = trace.data[r][non_singletons[0]].intersect(&remainder) {
                        trace.data[r][non_singletons[0]] = refined;
                    } else {
                        return MayBeFlag::False;
                    }
                } else if non_singletons.is_empty() {
                    // All singletons — verify the sum
                    let mut sum = AbstractInterval::zero();
                    for &v_idx in vs {
                        sum = sum + trace.data[r][v_idx].clone();
                    }
                    if sum.intersect(&target_iv).is_none() {
                        return MayBeFlag::False;
                    }
                }
            }
        }
    }
    MayBeFlag::MayBe
}
