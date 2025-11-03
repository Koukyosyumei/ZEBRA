use std::collections::HashSet;

use crate::interval::AbstractInterval;
use crate::symbolic::LatticeVMConstraints;
use crate::symbolic::{LatticeVMSymbolicEntry, LatticeVMSymbolicExpr};

/// Converts the given expression into SMT-LIB constraints over all rows.
pub fn expr_to_smt(
    constraints: &LatticeVMConstraints,
    n_rows: usize,
    n_cols: usize,
    prime: u32,
) -> String {
    fn helper(
        expr: &LatticeVMSymbolicExpr,
        row_id: usize,
        n_rows: usize,
        vars: &mut HashSet<String>,
    ) -> String {
        match expr {
            LatticeVMSymbolicExpr::IsFirstRow => {
                if row_id == 0 {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }
            LatticeVMSymbolicExpr::IsTransition => {
                if row_id < n_rows - 1 {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }
            LatticeVMSymbolicExpr::IsLastRow => {
                if row_id == n_rows - 1 {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }
            LatticeVMSymbolicExpr::Constant(AbstractInterval { lo, hi: _hi }) => {
                format!("{}", lo)
            }
            LatticeVMSymbolicExpr::Variable(v) => {
                // "curr" -> a_row_index, "next" -> a_row+1_index
                let base_row = match v.entry {
                    LatticeVMSymbolicEntry::Main { is_curr } => {
                        if is_curr {
                            row_id
                        } else {
                            row_id + 1
                        }
                    }
                    LatticeVMSymbolicEntry::Public => 0,
                };
                let name = format!("trace_{}_{}", base_row, v.index);
                vars.insert(name.clone());
                name
            }
            LatticeVMSymbolicExpr::Add(a, b) => {
                format!(
                    "(+ {} {})",
                    helper(a, row_id, n_rows, vars),
                    helper(b, row_id, n_rows, vars)
                )
            }
            LatticeVMSymbolicExpr::Sub(a, b) => {
                format!(
                    "(- {} {})",
                    helper(a, row_id, n_rows, vars),
                    helper(b, row_id, n_rows, vars)
                )
            }
            LatticeVMSymbolicExpr::Mul(a, b) => {
                format!(
                    "(* {} {})",
                    helper(a, row_id, n_rows, vars),
                    helper(b, row_id, n_rows, vars)
                )
            }
            LatticeVMSymbolicExpr::Neg(a) => format!("(- {})", helper(a, row_id, n_rows, vars)),
        }
    }

    let mut vars = HashSet::new();
    let mut smt = String::new();
    smt.push_str("(set-logic QF_NIA)\n");
    // smt.push_str("(set-option :incremental true)\n");
    // smt.push_str(&format!("(define-sort F () (_ FiniteField {}))\n", prime));

    // Declare all trace variables
    for i in 0..(n_rows + 1) {
        for j in 0..n_cols {
            let name = format!("trace_{}_{}", i, j);
            vars.insert(name.clone());
        }
    }
    for v in &vars {
        smt.push_str(&format!("(declare-fun {} () Int)\n", v));
    }

    // Add modular constraints for each row
    for expr in &constraints.air_constraints {
        for i in 0..n_rows {
            let body = helper(expr, i, n_rows, &mut vars);
            smt.push_str(&format!("(assert (= (mod {} {}) 0))\n", body, prime));
        }
    }

    for expr in &constraints.pv_pos_constraints {
        for i in 0..n_rows {
            let body = helper(expr, i, n_rows, &mut vars);
            smt.push_str(&format!("(assert (= (mod {} {}) 0))\n", body, prime));
        }
    }

    for expr in &constraints.pv_neg_constraints {
        for i in 0..n_rows {
            let body = helper(expr, i, n_rows, &mut vars);
            smt.push_str(&format!("(assert (not (= (mod {} {}) 0)))\n", body, prime));
        }
    }

    smt.push_str("(check-sat)\n(get-model)\n");
    smt
}
