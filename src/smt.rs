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
                "  (= trace_{}_{} {})\n",
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
            }
        }
    }

    smt.push_str("(check-sat)\n(get-model)\n");
    smt
}

/// Converts the given expression into SMT-LIB constraints over all rows using BitVectors.
pub fn expr_to_smt_bv(
    constraints: &LatticeVMConstraints,
    constants: &Vec<(usize, usize, AbstractInterval)>,
    neg_constants: &Vec<(usize, usize, AbstractInterval)>,
    range_types: &HashMap<usize, RangeType>,
    n_rows: usize,
    n_cols: usize,
    n_pvs: usize,
    prime: u32, // 32-bit prime
) -> String {
    fn helper(
        expr: &LatticeVMSymbolicExpr,
        row_id: usize,
        n_rows: usize,
        n_pvs: usize,
        vars: &mut HashSet<String>,
    ) -> String {
        fn word_to_bv32(
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
                    "(bvadd {} (bvmul {} #x{:08x}))",
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
                    "#x1".to_string()
                } else {
                    "#x0".to_string()
                }
            }
            LatticeVMSymbolicExpr::IsTransition => {
                if row_id < n_rows - 1 {
                    "#x1".to_string()
                } else {
                    "#x0".to_string()
                }
            }
            LatticeVMSymbolicExpr::IsLastRow => {
                if row_id == n_rows - 1 {
                    "#x1".to_string()
                } else {
                    "#x0".to_string()
                }
            }
            LatticeVMSymbolicExpr::Constant(AbstractInterval { lo, hi: _ }) => {
                format!("{}", lo)
            }
            LatticeVMSymbolicExpr::Variable(v) => {
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
                    "(bvadd {} {})",
                    helper(a, row_id, n_rows, n_pvs, vars),
                    helper(b, row_id, n_rows, n_pvs, vars)
                )
            }
            LatticeVMSymbolicExpr::Sub(a, b) => {
                format!(
                    "(bvsub {} {})",
                    helper(a, row_id, n_rows, n_pvs, vars),
                    helper(b, row_id, n_rows, n_pvs, vars)
                )
            }
            LatticeVMSymbolicExpr::Mul(a, b) => {
                format!(
                    "(bvmul {} {})",
                    helper(a, row_id, n_rows, n_pvs, vars),
                    helper(b, row_id, n_rows, n_pvs, vars)
                )
            }
            LatticeVMSymbolicExpr::Neg(a) => {
                // Two's complement negation
                format!("(bvneg {})", helper(a, row_id, n_rows, n_pvs, vars))
            }
            LatticeVMSymbolicExpr::Msb(a) => {
                format!(
                    "((_ zero_extend {}) ((_ extract {} {}) {}))",
                    32 - 1,
                    32 - 1,
                    32 - 1,
                    helper(a, row_id, n_rows, n_pvs, vars),
                )
            }
            LatticeVMSymbolicExpr::WhenNonZero(a, b) => {
                format!(
                    "(ite (= {} #x00000000) #x00000000 {})",
                    helper(a, row_id, n_rows, n_pvs, vars),
                    helper(b, row_id, n_rows, n_pvs, vars)
                )
            }
            LatticeVMSymbolicExpr::WhenZero(a, b) => {
                format!(
                    "(ite (= {} #x00000000) {} #x00000000)",
                    helper(a, row_id, n_rows, n_pvs, vars),
                    helper(b, row_id, n_rows, n_pvs, vars)
                )
            }
            LatticeVMSymbolicExpr::Lt(a, b) => {
                format!(
                    "(ite (bvult {} {}) #x00000000 #x00000001)",
                    helper(a, row_id, n_rows, n_pvs, vars),
                    helper(b, row_id, n_rows, n_pvs, vars),
                )
            }
            LatticeVMSymbolicExpr::Flip(a) => {
                format!(
                    "(ite (= {} #x00000000) #x00000001 #x00000000)",
                    helper(a, row_id, n_rows, n_pvs, vars),
                )
            }
            LatticeVMSymbolicExpr::WordAdd(a_vec, b_vec) => {
                let mut a_val = helper(&a_vec[0], row_id, n_rows, n_pvs, vars);
                let mut factor = 1;
                for w in a_vec.iter().skip(1) {
                    factor *= 256;
                    a_val = format!(
                        "(bvadd {} (bvmul {} #x{:08x}))",
                        a_val,
                        helper(w, row_id, n_rows, n_pvs, vars),
                        factor
                    );
                }

                let mut b_val = helper(&b_vec[0], row_id, n_rows, n_pvs, vars);
                let mut factor = 1;
                for w in b_vec.iter().skip(1) {
                    factor *= 256;
                    b_val = format!(
                        "(bvadd {} (bvmul {} #x{:08x}))",
                        b_val,
                        helper(w, row_id, n_rows, n_pvs, vars),
                        factor
                    );
                }

                format!("(bvadd {} {})", a_val, b_val)
            }
            LatticeVMSymbolicExpr::WordSubU(a_vec, b_vec) => {
                let a_val = word_to_bv32(a_vec, row_id, n_rows, n_pvs, vars);
                let b_val = word_to_bv32(b_vec, row_id, n_rows, n_pvs, vars);
                format!("(bvsub {} {})", a_val, b_val)
            }
            LatticeVMSymbolicExpr::WordMulhs(a_vec, b_vec) => {
                let a = word_to_bv32(a_vec, row_id, n_rows, n_pvs, vars);
                let b = word_to_bv32(b_vec, row_id, n_rows, n_pvs, vars);

                format!(
                    "((_ extract 63 32) \
            (bvmul ((_ sign_extend 32) {}) ((_ sign_extend 32) {})))",
                    a, b
                )
            }
            LatticeVMSymbolicExpr::WordMulhu(a_vec, b_vec) => {
                let a = word_to_bv32(a_vec, row_id, n_rows, n_pvs, vars);
                let b = word_to_bv32(b_vec, row_id, n_rows, n_pvs, vars);

                format!(
                    "((_ extract 63 32) \
            (bvmul ((_ zero_extend 32) {}) ((_ zero_extend 32) {})))",
                    a, b
                )
            }
            LatticeVMSymbolicExpr::WordMul(a_vec, b_vec) => {
                let mut a_val = helper(&a_vec[0], row_id, n_rows, n_pvs, vars);
                let mut factor = 1;
                for w in a_vec.iter().skip(1) {
                    factor *= 256;
                    a_val = format!(
                        "(bvadd {} (bvmul {} #x{:08x}))",
                        a_val,
                        helper(w, row_id, n_rows, n_pvs, vars),
                        factor
                    );
                }

                let mut b_val = helper(&b_vec[0], row_id, n_rows, n_pvs, vars);
                let mut factor = 1;
                for w in b_vec.iter().skip(1) {
                    factor *= 256;
                    b_val = format!(
                        "(bvadd {} (bvmul {} #x{:08x}))",
                        b_val,
                        helper(w, row_id, n_rows, n_pvs, vars),
                        factor
                    );
                }

                format!("(bvmul {} {})", a_val, b_val)
            }
            LatticeVMSymbolicExpr::WordSLt(a_vec, b_vec) => {
                let mut a_val = helper(&a_vec[0], row_id, n_rows, n_pvs, vars);
                let mut factor = 1;
                for w in a_vec.iter().skip(1) {
                    factor *= 256;
                    a_val = format!(
                        "(bvadd {} (bvmul {} #x{:08x}))",
                        a_val,
                        helper(w, row_id, n_rows, n_pvs, vars),
                        factor
                    );
                }

                let mut b_val = helper(&b_vec[0], row_id, n_rows, n_pvs, vars);
                let mut factor = 1;
                for w in b_vec.iter().skip(1) {
                    factor *= 256;
                    b_val = format!(
                        "(bvadd {} (bvmul {} #x{:08x}))",
                        b_val,
                        helper(w, row_id, n_rows, n_pvs, vars),
                        factor
                    );
                }

                format!("(ite (bvslt {} {}) #x00000000 #x00000001)", a_val, b_val,)
            }
            LatticeVMSymbolicExpr::WordSrl(a_vec, b_vec) => {
                let mut a_val = helper(&a_vec[0], row_id, n_rows, n_pvs, vars);
                let mut factor = 1;
                for w in a_vec.iter().skip(1) {
                    factor *= 256;
                    a_val = format!(
                        "(bvadd {} (bvmul {} #x{:08x}))",
                        a_val,
                        helper(w, row_id, n_rows, n_pvs, vars),
                        factor
                    );
                }

                let mut b_val = helper(&b_vec[0], row_id, n_rows, n_pvs, vars);
                let mut factor = 1;
                for w in b_vec.iter().skip(1) {
                    factor *= 256;
                    b_val = format!(
                        "(bvadd {} (bvmul {} #x{:08x}))",
                        b_val,
                        helper(w, row_id, n_rows, n_pvs, vars),
                        factor
                    );
                }

                format!("(bvlshr {} {})", a_val, b_val)
            }
            LatticeVMSymbolicExpr::WordAnd(a_vec, b_vec) => {
                let mut res = vec![];
                for (a, b) in a_vec.iter().zip(b_vec.iter()) {
                    let a_bv = helper(a, row_id, n_rows, n_pvs, vars);
                    let b_bv = helper(b, row_id, n_rows, n_pvs, vars);
                    res.push(format!("(bvand {} {})", a_bv, b_bv));
                }

                // ここで Vec<String> を返すか、再度 little-endian を 32bit word に畳むかは用途次第
                // WordAdd と同じく 32bit に畳むなら：
                let mut acc = res[0].clone();
                let mut factor: u32 = 1;
                for limb in res.iter().skip(1) {
                    factor = factor.wrapping_mul(256);
                    acc = format!("(bvadd {} (bvmul {} #x{:08x}))", acc, limb, factor);
                }
                acc
            }
            LatticeVMSymbolicExpr::WordOr(a_vec, b_vec) => {
                assert_eq!(
                    a_vec.len(),
                    b_vec.len(),
                    "WordAnd vectors must have same length"
                );

                let mut res = vec![];
                for (a, b) in a_vec.iter().zip(b_vec.iter()) {
                    let a_bv = helper(a, row_id, n_rows, n_pvs, vars);
                    let b_bv = helper(b, row_id, n_rows, n_pvs, vars);
                    res.push(format!("(bvor {} {})", a_bv, b_bv));
                }

                // ここで Vec<String> を返すか、再度 little-endian を 32bit word に畳むかは用途次第
                // WordAdd と同じく 32bit に畳むなら：
                let mut acc = res[0].clone();
                let mut factor: u32 = 1;
                for limb in res.iter().skip(1) {
                    factor = factor.wrapping_mul(256);
                    acc = format!("(bvadd {} (bvmul {} #x{:08x}))", acc, limb, factor);
                }
                acc
            }
            LatticeVMSymbolicExpr::WordXOr(a_vec, b_vec) => {
                assert_eq!(
                    a_vec.len(),
                    b_vec.len(),
                    "WordAnd vectors must have same length"
                );

                let mut res = vec![];
                for (a, b) in a_vec.iter().zip(b_vec.iter()) {
                    let a_bv = helper(a, row_id, n_rows, n_pvs, vars);
                    let b_bv = helper(b, row_id, n_rows, n_pvs, vars);
                    res.push(format!("(bvxor {} {})", a_bv, b_bv));
                }

                // ここで Vec<String> を返すか、再度 little-endian を 32bit word に畳むかは用途次第
                // WordAdd と同じく 32bit に畳むなら：
                let mut acc = res[0].clone();
                let mut factor: u32 = 1;
                for limb in res.iter().skip(1) {
                    factor = factor.wrapping_mul(256);
                    acc = format!("(bvadd {} (bvmul {} #x{:08x}))", acc, limb, factor);
                }
                acc
            }
            other => todo!("{:?} is not implemented", other),
        }
    }

    let mut vars = HashSet::new();
    let mut smt = String::new();
    smt.push_str("(set-logic QF_BV)\n");

    // Declare all trace variables as 32-bit bitvectors
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
        smt.push_str(&format!("(declare-fun {} () (_ BitVec 32))\n", v));
    }

    let prime_hex = format!("#x{:08x}", prime);

    // Add modular constraints using bvurem
    for expr in &constraints.air_constraints {
        for i in 0..n_rows {
            let body = helper(expr, i, n_rows, n_pvs, &mut vars);
            smt.push_str(&format!(
                "(assert (= (bvurem {} {}) #x00000000))\n",
                body, prime_hex
            ));
        }
    }

    for expr in &constraints.lookup_constraints {
        for i in 0..n_rows {
            let body = helper(expr, i, n_rows, n_pvs, &mut vars);
            smt.push_str(&format!("(assert (= {} #x00000000))\n", body));
        }
    }

    for expr in &constraints.pv_pos_constraints {
        for i in 0..n_rows {
            let body = helper(expr, i, n_rows, n_pvs, &mut vars);
            smt.push_str(&format!(
                "(assert (= (bvurem {} {}) #x00000000))\n",
                body, prime_hex
            ));
        }
    }

    for expr in &constraints.pv_neg_constraints {
        for i in 0..n_rows {
            let body = helper(expr, i, n_rows, n_pvs, &mut vars);
            smt.push_str(&format!(
                "(assert (not (= (bvurem {} {}) #x00000000)))\n",
                body, prime_hex
            ));
        }
    }

    // Constants
    for (i, j, v) in constants {
        smt.push_str(&format!(
            "(assert (= trace_{}_{} #x{:08x}))\n",
            i,
            j,
            v.as_canonical_u32(prime)
        ));
    }

    if !neg_constants.is_empty() {
        smt.push_str("(assert (not (and\n");
        for (i, j, v) in neg_constants {
            smt.push_str(&format!(
                "  (= trace_{}_{} #x{:08x})\n",
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
                smt.push_str(&format!("(assert (bvule trace_{}_{} #x000000ff))\n", i, j,));
            }
        }
    }

    smt.push_str("(check-sat)\n(get-model)\n");
    smt
}
