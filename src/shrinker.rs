use std::collections::HashSet;

use crate::interval::{AbstractInterval, MayBeFlag};
use crate::symbolic::{
    gather_vars_simple, get_curr_i, get_curr_i_sub_const, get_curr_i_sub_cur_j,
    LatticeVMSymbolicExpr,
};
use crate::trace::AbstractTrace;

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
    constraints: &[LatticeVMSymbolicExpr],
    prime: u32,
) -> Vec<(usize, usize, i64)> {
    let mut const_constraints = Vec::new();

    for c in constraints {
        // Mul(selector, Sub(lhs, rhs))
        if let LatticeVMSymbolicExpr::Mul(lhs_expr, rhs_expr) = c {
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
    const_constraints: &[(usize, usize, i64)], // (selector_idx, value_idx, target_constant)
) -> MayBeFlag {
    let num_rows = trace.data.len();

    for r in 0..num_rows {
        for &(sel_idx, val_idx, target) in const_constraints {
            let selector = &trace.data[r][sel_idx];
            let target_interval = AbstractInterval::from_i64(target);

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
    constraints: &[LatticeVMSymbolicExpr],
) -> Vec<(usize, usize, usize)> {
    let mut eq_constraints = Vec::new();

    for c in constraints {
        // Mul(selector, Sub(lhs, rhs))
        if let LatticeVMSymbolicExpr::Mul(lhs_expr, rhs_expr) = c {
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
pub fn move_sub_expr_to_right(expr: &LatticeVMSymbolicExpr) -> LatticeVMSymbolicExpr {
    // (a - c) + b ==> (a + b) - c
    if let LatticeVMSymbolicExpr::Add(lhs_1, rhs_1) = expr {
        if let LatticeVMSymbolicExpr::Sub(lhs_2, rhs_2) = *lhs_1.clone() {
            return LatticeVMSymbolicExpr::Sub(
                Box::new(LatticeVMSymbolicExpr::Add(lhs_2, rhs_1.clone())),
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
    pub lhs_var: usize,                         // a
    pub affine_rhs: Box<LatticeVMSymbolicExpr>, // L
    pub quotient_var: usize,                    // e
    pub stride: u32,                            // d > 0
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
    constraints: &[LatticeVMSymbolicExpr],
    prime: u32,
) -> Vec<AbirConstraint> {
    let mut result = Vec::new();

    for constraint in constraints {
        // Expect: a - RHS
        let (lhs, rhs) = match constraint {
            LatticeVMSymbolicExpr::Sub(lhs, rhs) => (lhs, rhs),
            _ => continue,
        };

        let lhs_var = match &**lhs {
            LatticeVMSymbolicExpr::Variable(v) => v.index,
            _ => continue,
        };

        // Normalize RHS: (L - d*e)
        let normalized_rhs = move_sub_expr_to_right(rhs);

        let (affine_rhs, de_term) = match normalized_rhs {
            LatticeVMSymbolicExpr::Sub(lhs, rhs) => (lhs, rhs),
            _ => continue,
        };

        // Match d * e
        let (quotient_var, stride) = match &*de_term {
            LatticeVMSymbolicExpr::Mul(x, y) => match (&**x, &**y) {
                (LatticeVMSymbolicExpr::Variable(v), LatticeVMSymbolicExpr::Constant(k))
                | (LatticeVMSymbolicExpr::Constant(k), LatticeVMSymbolicExpr::Variable(v)) => {
                    (v.index, k.as_canonical_u32(prime))
                }
                _ => continue,
            },
            LatticeVMSymbolicExpr::Variable(v) => (v.index, 1),
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
            let inferred_e = (rhs_interval - row[*lhs_var].clone()).div_floor(*stride as i64);

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
