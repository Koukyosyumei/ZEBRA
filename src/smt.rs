use std::collections::{HashMap, HashSet};

use crate::interval::AbstractInterval;
use crate::solver::RangeType;
use crate::symbolic::LatticeVMConstraints;
use crate::symbolic::{LatticeVMSymbolicEntry, LatticeVMSymbolicExpr};

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
        prime: u32,
    ) -> String {
        let prime_hex = format!("#x{:016x}", prime);

        fn word_to_bv32(
            vec: &[Box<LatticeVMSymbolicExpr>],
            row_id: usize,
            n_rows: usize,
            n_pvs: usize,
            vars: &mut HashSet<String>,
            prime: u32,
        ) -> String {
            let mut acc = helper(&vec[0], row_id, n_rows, n_pvs, vars, prime);
            let mut factor: u32 = 1;

            for w in vec.iter().skip(1) {
                factor = factor.wrapping_mul(256);
                acc = format!(
                    "(bvadd {} (bvmul {} #x{:016x}))",
                    acc,
                    helper(w, row_id, n_rows, n_pvs, vars, prime),
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
                format!("#x{:016x}", lo)
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
                    helper(a, row_id, n_rows, n_pvs, vars, prime),
                    helper(b, row_id, n_rows, n_pvs, vars, prime),
                )
            }
            LatticeVMSymbolicExpr::Sub(a, b) => {
                format!(
                    "(bvurem (bvsub (bvadd {} {}) {}) {})",
                    helper(a, row_id, n_rows, n_pvs, vars, prime),
                    prime_hex,
                    helper(b, row_id, n_rows, n_pvs, vars, prime),
                    prime_hex,
                )
            }
            LatticeVMSymbolicExpr::Mul(a, b) => {
                format!(
                    "(bvurem (bvmul {} {}) {})",
                    helper(a, row_id, n_rows, n_pvs, vars, prime),
                    helper(b, row_id, n_rows, n_pvs, vars, prime),
                    prime_hex
                )
            }
            LatticeVMSymbolicExpr::Neg(a) => {
                // Two's complement negation
                format!("(bvneg {})", helper(a, row_id, n_rows, n_pvs, vars, prime))
            }
            LatticeVMSymbolicExpr::Msb(a) => {
                format!(
                    "((_ zero_extend {}) ((_ extract {} {}) {}))",
                    32 - 1,
                    32 - 1,
                    32 - 1,
                    helper(a, row_id, n_rows, n_pvs, vars, prime),
                )
            }
            LatticeVMSymbolicExpr::WhenNonZero(a, b) => {
                format!(
                    "(ite (= {} #x0000000000000000) #x0000000000000000 {})",
                    helper(a, row_id, n_rows, n_pvs, vars, prime),
                    helper(b, row_id, n_rows, n_pvs, vars, prime)
                )
            }
            LatticeVMSymbolicExpr::WhenZero(a, b) => {
                format!(
                    "(ite (= {} #x0000000000000000) {} #x0000000000000000)",
                    helper(a, row_id, n_rows, n_pvs, vars, prime),
                    helper(b, row_id, n_rows, n_pvs, vars, prime)
                )
            }
            LatticeVMSymbolicExpr::Lt(a, b) => {
                format!(
                    "(ite (bvult {} {}) #x0000000000000000 #x0000000000000001)",
                    helper(a, row_id, n_rows, n_pvs, vars, prime),
                    helper(b, row_id, n_rows, n_pvs, vars, prime),
                )
            }
            LatticeVMSymbolicExpr::Flip(a) => {
                format!(
                    "(ite (= {} #x0000000000000000) #x0000000000000001 #x0000000000000000)",
                    helper(a, row_id, n_rows, n_pvs, vars, prime),
                )
            }
            LatticeVMSymbolicExpr::WordAdd(a_vec, b_vec) => {
                let mut a_val = helper(&a_vec[0], row_id, n_rows, n_pvs, vars, prime);
                let mut factor = 1;
                for w in a_vec.iter().skip(1) {
                    factor *= 256;
                    a_val = format!(
                        "(bvadd {} (bvmul {} #x{:016x}))",
                        a_val,
                        helper(w, row_id, n_rows, n_pvs, vars, prime),
                        factor
                    );
                }

                let mut b_val = helper(&b_vec[0], row_id, n_rows, n_pvs, vars, prime);
                let mut factor = 1;
                for w in b_vec.iter().skip(1) {
                    factor *= 256;
                    b_val = format!(
                        "(bvadd {} (bvmul {} #x{:016x}))",
                        b_val,
                        helper(w, row_id, n_rows, n_pvs, vars, prime),
                        factor
                    );
                }

                format!("(bvadd {} {})", a_val, b_val)
            }
            LatticeVMSymbolicExpr::WordSubU(a_vec, b_vec) => {
                let a_val = word_to_bv32(a_vec, row_id, n_rows, n_pvs, vars, prime);
                let b_val = word_to_bv32(b_vec, row_id, n_rows, n_pvs, vars, prime);
                format!("(bvsub {} {})", a_val, b_val)
            }
            LatticeVMSymbolicExpr::WordMulhs(a_vec, b_vec) => {
                let a = word_to_bv32(a_vec, row_id, n_rows, n_pvs, vars, prime);
                let b = word_to_bv32(b_vec, row_id, n_rows, n_pvs, vars, prime);

                format!(
                    "((_ extract 63 32) \
            (bvmul ((_ sign_extend 32) {}) ((_ sign_extend 32) {})))",
                    a, b
                )
            }
            LatticeVMSymbolicExpr::WordMulhu(a_vec, b_vec) => {
                let a = word_to_bv32(a_vec, row_id, n_rows, n_pvs, vars, prime);
                let b = word_to_bv32(b_vec, row_id, n_rows, n_pvs, vars, prime);

                format!(
                    "((_ extract 63 32) \
            (bvmul ((_ zero_extend 32) {}) ((_ zero_extend 32) {})))",
                    a, b
                )
            }
            LatticeVMSymbolicExpr::WordMul(a_vec, b_vec) => {
                let mut a_val = helper(&a_vec[0], row_id, n_rows, n_pvs, vars, prime);
                let mut factor = 1;
                for w in a_vec.iter().skip(1) {
                    factor *= 256;
                    a_val = format!(
                        "(bvadd {} (bvmul {} #x{:016x}))",
                        a_val,
                        helper(w, row_id, n_rows, n_pvs, vars, prime),
                        factor
                    );
                }

                let mut b_val = helper(&b_vec[0], row_id, n_rows, n_pvs, vars, prime);
                let mut factor = 1;
                for w in b_vec.iter().skip(1) {
                    factor *= 256;
                    b_val = format!(
                        "(bvadd {} (bvmul {} #x{:016x}))",
                        b_val,
                        helper(w, row_id, n_rows, n_pvs, vars, prime),
                        factor
                    );
                }

                format!("(bvmul {} {})", a_val, b_val)
            }
            LatticeVMSymbolicExpr::WordSLt(a_vec, b_vec) => {
                let mut a_val = helper(&a_vec[0], row_id, n_rows, n_pvs, vars, prime);
                let mut factor = 1;
                for w in a_vec.iter().skip(1) {
                    factor *= 256;
                    a_val = format!(
                        "(bvadd {} (bvmul {} #x{:016x}))",
                        a_val,
                        helper(w, row_id, n_rows, n_pvs, vars, prime),
                        factor
                    );
                }

                let mut b_val = helper(&b_vec[0], row_id, n_rows, n_pvs, vars, prime);
                let mut factor = 1;
                for w in b_vec.iter().skip(1) {
                    factor *= 256;
                    b_val = format!(
                        "(bvadd {} (bvmul {} #x{:016x}))",
                        b_val,
                        helper(w, row_id, n_rows, n_pvs, vars, prime),
                        factor
                    );
                }

                format!(
                    "(ite (bvslt {} {}) #x0000000000000000 #x0000000000000001)",
                    a_val, b_val,
                )
            }
            LatticeVMSymbolicExpr::WordSrl(a_vec, b_vec) => {
                let mut a_val = helper(&a_vec[0], row_id, n_rows, n_pvs, vars, prime);
                let mut factor = 1;
                for w in a_vec.iter().skip(1) {
                    factor *= 256;
                    a_val = format!(
                        "(bvadd {} (bvmul {} #x{:016x}))",
                        a_val,
                        helper(w, row_id, n_rows, n_pvs, vars, prime),
                        factor
                    );
                }

                let mut b_val = helper(&b_vec[0], row_id, n_rows, n_pvs, vars, prime);
                let mut factor = 1;
                for w in b_vec.iter().skip(1) {
                    factor *= 256;
                    b_val = format!(
                        "(bvadd {} (bvmul {} #x{:016x}))",
                        b_val,
                        helper(w, row_id, n_rows, n_pvs, vars, prime),
                        factor
                    );
                }

                format!("(bvlshr {} {})", a_val, b_val)
            }
            LatticeVMSymbolicExpr::WordAnd(a_vec, b_vec) => {
                let mut res = vec![];
                for (a, b) in a_vec.iter().zip(b_vec.iter()) {
                    let a_bv = helper(a, row_id, n_rows, n_pvs, vars, prime);
                    let b_bv = helper(b, row_id, n_rows, n_pvs, vars, prime);
                    res.push(format!("(bvand {} {})", a_bv, b_bv));
                }

                // ここで Vec<String> を返すか、再度 little-endian を 32bit word に畳むかは用途次第
                // WordAdd と同じく 32bit に畳むなら：
                let mut acc = res[0].clone();
                let mut factor: u32 = 1;
                for limb in res.iter().skip(1) {
                    factor = factor.wrapping_mul(256);
                    acc = format!("(bvadd {} (bvmul {} #x{:016x}))", acc, limb, factor);
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
                    let a_bv = helper(a, row_id, n_rows, n_pvs, vars, prime);
                    let b_bv = helper(b, row_id, n_rows, n_pvs, vars, prime);
                    res.push(format!("(bvor {} {})", a_bv, b_bv));
                }

                // ここで Vec<String> を返すか、再度 little-endian を 32bit word に畳むかは用途次第
                // WordAdd と同じく 32bit に畳むなら：
                let mut acc = res[0].clone();
                let mut factor: u32 = 1;
                for limb in res.iter().skip(1) {
                    factor = factor.wrapping_mul(256);
                    acc = format!("(bvadd {} (bvmul {} #x{:016x}))", acc, limb, factor);
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
                    let a_bv = helper(a, row_id, n_rows, n_pvs, vars, prime);
                    let b_bv = helper(b, row_id, n_rows, n_pvs, vars, prime);
                    res.push(format!("(bvxor {} {})", a_bv, b_bv));
                }

                // ここで Vec<String> を返すか、再度 little-endian を 32bit word に畳むかは用途次第
                // WordAdd と同じく 32bit に畳むなら：
                let mut acc = res[0].clone();
                let mut factor: u32 = 1;
                for limb in res.iter().skip(1) {
                    factor = factor.wrapping_mul(256);
                    acc = format!("(bvadd {} (bvmul {} #x{:016x}))", acc, limb, factor);
                }
                acc
            }
            other => todo!("{:?} is not implemented", other),
        }
    }

    let prime_hex = format!("#x{:016x}", prime);

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
        smt.push_str(&format!("(declare-fun {} () (_ BitVec 64))\n", v));
    }

    // Add modular constraints using bvurem
    for expr in &constraints.air_constraints {
        for i in 0..n_rows {
            let body = helper(expr, i, n_rows, n_pvs, &mut vars, prime);
            smt.push_str(&format!(
                "(assert (= (bvurem {} {}) #x0000000000000000))\n",
                body, prime_hex
            ));
        }
    }

    for expr in &constraints.lookup_constraints {
        for i in 0..n_rows {
            let body = helper(expr, i, n_rows, n_pvs, &mut vars, prime);
            smt.push_str(&format!("(assert (= {} #x0000000000000000))\n", body));
        }
    }

    for expr in &constraints.pv_pos_constraints {
        for i in 0..n_rows {
            let body = helper(expr, i, n_rows, n_pvs, &mut vars, prime);
            smt.push_str(&format!(
                "(assert (= (bvurem {} {}) #x0000000000000000))\n",
                body, prime_hex
            ));
        }
    }

    for expr in &constraints.pv_neg_constraints {
        for i in 0..n_rows {
            let body = helper(expr, i, n_rows, n_pvs, &mut vars, prime);
            smt.push_str(&format!(
                "(assert (not (= (bvurem {} {}) #x0000000000000000)))\n",
                body, prime_hex
            ));
        }
    }

    // Constants
    for (i, j, v) in constants {
        smt.push_str(&format!(
            "(assert (= trace_{}_{} #x{:016x}))\n",
            i,
            j,
            v.as_canonical_u32(prime)
        ));
    }

    if !neg_constants.is_empty() {
        smt.push_str("(assert (not (and\n");
        for (i, j, v) in neg_constants {
            smt.push_str(&format!(
                "  (= trace_{}_{} #x{:016x})\n",
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
                smt.push_str(&format!(
                    "(assert (bvule trace_{}_{} #x00000000000000ff))\n",
                    i, j,
                ));
            }
        }
    }

    smt.push_str("(check-sat)\n(get-model)\n");
    smt
}
