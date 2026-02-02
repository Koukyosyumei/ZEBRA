use std::collections::{HashMap, HashSet};

use crate::interval::AbstractInterval;
use crate::solver::RangeType;
use crate::symbolic::LatticeVMConstraints;
use crate::symbolic::{LatticeVMSymbolicEntry, LatticeVMSymbolicExpr};

/// Converts the given expression into SMT-LIB constraints over all rows.
pub fn expr_to_smt(
    constraints: &LatticeVMConstraints,
    constants: &Vec<(usize, usize, AbstractInterval)>,
    neg_constants: &Vec<(usize, usize, AbstractInterval)>,
    range_types: &HashMap<usize, RangeType>,
    n_rows: usize,
    n_cols: usize,
    n_pvs: usize,
    prime: u32,
) -> String {
    fn helper(
        expr: &LatticeVMSymbolicExpr,
        row_id: usize,
        n_rows: usize,
        n_pvs: usize,
        vars: &mut HashSet<String>,
    ) -> String {
        fn reduce_word(
            vec: &[Box<LatticeVMSymbolicExpr>],
            row_id: usize,
            n_rows: usize,
            n_pvs: usize,
            vars: &mut HashSet<String>,
        ) -> String {
            let mut acc = helper(&vec[0], row_id, n_rows, n_pvs, vars);
            let mut factor: u32 = 1;
            for w in vec.iter().skip(1) {
                factor = factor.wrapping_mul(256);
                acc = format!(
                    "(+ {} (* {} {}))",
                    acc,
                    helper(w, row_id, n_rows, n_pvs, vars),
                    factor
                );
            }
            acc
        }

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
                let (ty, base_row) = match v.entry {
                    LatticeVMSymbolicEntry::Main { is_curr } => {
                        if is_curr {
                            ("trace", row_id)
                        } else {
                            ("trace", row_id + 1)
                        }
                    }
                    LatticeVMSymbolicEntry::Public => ("public", 0),
                    _ => todo!(),
                };
                let name = format!("{}_{}_{}", ty, base_row, v.index);
                vars.insert(name.clone());
                name
            }
            LatticeVMSymbolicExpr::Add(a, b) => {
                format!(
                    "(+ {} {})",
                    helper(a, row_id, n_rows, n_pvs, vars),
                    helper(b, row_id, n_rows, n_pvs, vars)
                )
            }
            LatticeVMSymbolicExpr::Sub(a, b) => {
                format!(
                    "(- {} {})",
                    helper(a, row_id, n_rows, n_pvs, vars),
                    helper(b, row_id, n_rows, n_pvs, vars)
                )
            }
            LatticeVMSymbolicExpr::Mul(a, b) => {
                format!(
                    "(* {} {})",
                    helper(a, row_id, n_rows, n_pvs, vars),
                    helper(b, row_id, n_rows, n_pvs, vars)
                )
            }
            LatticeVMSymbolicExpr::Neg(a) => {
                format!("(- {})", helper(a, row_id, n_rows, n_pvs, vars))
            }
            LatticeVMSymbolicExpr::WordAnd(a_vec, b_vec) => {
                let a_val = reduce_word(a_vec, row_id, n_rows, n_pvs, vars);
                let b_val = reduce_word(b_vec, row_id, n_rows, n_pvs, vars);
                format!("bitwise_and({}, {})", a_val, b_val)
            }
            LatticeVMSymbolicExpr::WordOr(a_vec, b_vec) => {
                let a_val = reduce_word(a_vec, row_id, n_rows, n_pvs, vars);
                let b_val = reduce_word(b_vec, row_id, n_rows, n_pvs, vars);
                format!("bitwise_or({}, {})", a_val, b_val)
            }
            LatticeVMSymbolicExpr::WordXOr(a_vec, b_vec) => {
                let a_val = reduce_word(a_vec, row_id, n_rows, n_pvs, vars);
                let b_val = reduce_word(b_vec, row_id, n_rows, n_pvs, vars);
                format!("bitwise_xor({}, {})", a_val, b_val)
            }
            _ => {
                todo!()
            }
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
    for i in 0..n_pvs {
        let name = format!("public_0_{}", i);
        vars.insert(name.clone());
    }
    for v in &vars {
        smt.push_str(&format!("(declare-fun {} () Int)\n", v));
    }

    // Add modular constraints for each row
    for expr in &constraints.air_constraints {
        for i in 0..n_rows {
            let body = helper(expr, i, n_rows, n_pvs, &mut vars);
            smt.push_str(&format!("(assert (= (mod {} {}) 0))\n", body, prime));
        }
    }

    for expr in &constraints.pv_pos_constraints {
        for i in 0..n_rows {
            let body = helper(expr, i, n_rows, n_pvs, &mut vars);
            smt.push_str(&format!("(assert (= (mod {} {}) 0))\n", body, prime));
        }
    }

    for expr in &constraints.pv_neg_constraints {
        for i in 0..n_rows {
            let body = helper(expr, i, n_rows, n_pvs, &mut vars);
            smt.push_str(&format!("(assert (not (= (mod {} {}) 0)))\n", body, prime));
        }
    }

    for (i, j, v) in constants {
        smt.push_str(&format!(
            "(assert (= trace_{}_{} {}))\n",
            i,
            j,
            v.as_canonical_u32(prime)
        ));
    }

    if !neg_constants.is_empty() {
        smt.push_str("(assert (not (and\n");
        for (i, j, v) in neg_constants {
            smt.push_str(&format!(
                "  (= (mod (- trace_{}_{} {}) {prime}) 0)\n",
                i,
                j,
                v.as_canonical_u32(prime)
            ));
        }
        smt.push_str(")))\n");
    }

    // ranges
    for (j, k) in range_types {
        if let RangeType::U8 = k {
            for i in 0..n_rows {
                smt.push_str(&format!("(assert (<= trace_{}_{} 255))\n", i, j,));
                smt.push_str(&format!("(assert (<= 0 trace_{}_{}))\n", i, j,));
            }
        }
    }

    smt.push_str("(check-sat)\n(get-model)\n");
    smt
}
