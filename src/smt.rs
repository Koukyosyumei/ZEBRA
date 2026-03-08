use std::collections::{HashMap, HashSet};

use crate::constraint::LatticeVMConstraints;
use crate::interval::AbstractInterval;
use crate::solver::RangeType;
use crate::symbolic::{LatticeVMSymbolicEntry, LatticeVMSymbolicExpr};

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

        let mut rec = |e| helper(e, row_id, n_rows, n_pvs, vars);

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
            LatticeVMSymbolicExpr::WhenNonZero(a, b) => {
                format!("(ite (= {} 0) 0 {})", rec(a), rec(b))
            }
            LatticeVMSymbolicExpr::WhenZero(a, b) => {
                format!("(ite (= {} 0) {} 0)", rec(a), rec(b))
            }
            LatticeVMSymbolicExpr::Add(a, b) => {
                format!("(+ {} {})", rec(a), rec(b))
            }
            LatticeVMSymbolicExpr::Sub(a, b) => {
                format!("(- {} {})", rec(a), rec(b))
            }
            LatticeVMSymbolicExpr::Mul(a, b) => {
                format!("(* {} {})", rec(a), rec(b))
            }
            LatticeVMSymbolicExpr::Lt(a, b) => {
                format!("(ite (< {} {}) 1 0)", rec(a), rec(b),)
            }
            LatticeVMSymbolicExpr::Flip(a) => {
                format!("(ite (= {} 0) 1 0)", rec(a),)
            }
            LatticeVMSymbolicExpr::And(a, b) => {
                format!("(bitwise_and {} {})", rec(a), rec(b))
            }
            LatticeVMSymbolicExpr::Or(a, b) => {
                format!("(bitwise_or {} {})", rec(a), rec(b))
            }
            LatticeVMSymbolicExpr::Xor(a, b) => {
                format!("(bitwise_xor {} {})", rec(a), rec(b))
            }
            LatticeVMSymbolicExpr::Neg(a) => {
                format!("(- {})", rec(a))
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
            other => {
                todo!("not yet implemented: {:?}", other)
            }
        }
    }

    let mut vars = HashSet::new();
    let mut smt = String::new();
    smt.push_str("(set-logic QF_NIA)\n");
    // smt.push_str("(set-option :incremental true)\n");
    // smt.push_str(&format!("(define-sort F () (_ FiniteField {}))\n", prime));

    smt.push_str(
        r#"
(define-fun bitwise_and ((a Int) (b Int)) Int
  (+ 
"#,
    );
    for i in 0..32 {
        let factor = 2u64.pow(i);
        smt.push_str(&format!(
            "    (* (ite (>= (mod a {}) {}) 1 0) (ite (>= (mod b {}) {}) 1 0) {})\n",
            2u64.pow(i + 1),
            factor,
            2u64.pow(i + 1),
            factor,
            factor
        ));
    }
    smt.push_str("))\n");

    smt.push_str(
        r#"
(define-fun bitwise_or ((a Int) (b Int)) Int
  (+ 
"#,
    );

    for i in 0..32 {
        let factor = 2u64.pow(i);
        smt.push_str(&format!(
            "    (* (ite (or (>= (mod a {}) {}) (>= (mod b {}) {})) 1 0) {})\n",
            2u64.pow(i + 1),
            factor,
            2u64.pow(i + 1),
            factor,
            factor
        ));
    }
    smt.push_str("))\n");

    smt.push_str(
        r#"
(define-fun bitwise_xor ((a Int) (b Int)) Int
  (+ 
"#,
    );

    for i in 0..32 {
        let factor = 2u64.pow(i);
        smt.push_str(&format!(
            "    (* (ite (= (ite (>= (mod a {}) {}) 1 0) (ite (>= (mod b {}) {}) 1 0)) 0 1) {})\n",
            2u64.pow(i + 1),
            factor,
            2u64.pow(i + 1),
            factor,
            factor
        ));
    }
    smt.push_str("))\n");

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

    for expr in &constraints.lookup_constraints {
        for i in 0..n_rows {
            let body = helper(expr, i, n_rows, n_pvs, &mut vars);
            smt.push_str(&format!("(assert (= {} 0))\n", body));
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
    fn word_to_bv32(
        vec: &[Box<LatticeVMSymbolicExpr>],
        row_id: usize,
        n_rows: usize,
        n_pvs: usize,
        vars: &mut HashSet<String>,
        prime: u32,
        not_field_op: bool,
    ) -> String {
        let mut acc = helper(&vec[0], row_id, n_rows, n_pvs, vars, prime, not_field_op);
        let mut factor: u32 = 1;

        for w in vec.iter().skip(1) {
            factor = factor.wrapping_mul(256);
            acc = format!(
                "(bvadd {} (bvmul {} #x{:08x}))",
                acc,
                helper(w, row_id, n_rows, n_pvs, vars, prime, not_field_op),
                factor
            );
        }
        acc
    }

    fn helper(
        expr: &LatticeVMSymbolicExpr,
        row_id: usize,
        n_rows: usize,
        n_pvs: usize,
        vars: &mut HashSet<String>,
        prime: u32,
        not_field_op: bool,
    ) -> String {
        let zero_hex = format!("#x{:08x}", 0);
        let one_hex = format!("#x{:08x}", 1);
        let mut rec = |e| helper(e, row_id, n_rows, n_pvs, vars, prime, not_field_op);

        match expr {
            LatticeVMSymbolicExpr::IsFirstRow => {
                if row_id == 0 {
                    one_hex
                } else {
                    zero_hex
                }
            }
            LatticeVMSymbolicExpr::IsTransition => {
                if row_id < n_rows - 1 {
                    one_hex
                } else {
                    zero_hex
                }
            }
            LatticeVMSymbolicExpr::IsLastRow => {
                if row_id == n_rows - 1 {
                    one_hex
                } else {
                    zero_hex
                }
            }
            LatticeVMSymbolicExpr::WhenNonZero(a, b) => {
                format!("(ite (= {} {}) {} {})", rec(a), zero_hex, zero_hex, rec(b))
            }
            LatticeVMSymbolicExpr::WhenZero(a, b) => {
                format!("(ite (= {} {}) {} {})", rec(a), zero_hex, rec(b), zero_hex)
            }
            LatticeVMSymbolicExpr::Constant(AbstractInterval { lo, hi: _ }) => {
                format!("#x{:08x}", lo)
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
                if not_field_op {
                    format!("(bvadd {} {})", rec(a), rec(b),)
                } else {
                    format!("(ff_add {} {})", rec(a), rec(b),)
                }
            }
            LatticeVMSymbolicExpr::Sub(a, b) => {
                if not_field_op {
                    format!("(bvsub {} {})", rec(a), rec(b),) // TODO: fix potential overflow
                } else {
                    format!("(ff_sub {} {})", rec(a), rec(b),)
                }
            }
            LatticeVMSymbolicExpr::Mul(a, b) => {
                if not_field_op {
                    format!("(bvmul {} {})", rec(a), rec(b),) // TODO: fix potential overflow
                } else {
                    format!("(ff_mul {} {})", rec(a), rec(b),)
                }
            }
            LatticeVMSymbolicExpr::MulLo(a, b) => {
                // low 32 bits of 32x32 multiplication
                format!("((_ extract 31 0) (bvmul {} {}))", rec(a), rec(b),)
            }
            LatticeVMSymbolicExpr::MulHiSS(a, b) => {
                // signed * signed, high 32 bits
                format!(
                    "((_ extract 63 32) \
          (bvmul ((_ sign_extend 32) {}) ((_ sign_extend 32) {})))",
                    rec(a),
                    rec(b),
                )
            }
            LatticeVMSymbolicExpr::MulHiUU(a, b) => {
                // unsigned * unsigned, high 32 bits
                format!(
                    "((_ extract 63 32) \
          (bvmul ((_ zero_extend 32) {}) ((_ zero_extend 32) {})))",
                    rec(a),
                    rec(b),
                )
            }
            LatticeVMSymbolicExpr::Lt(a, b) => {
                format!(
                    "(ite (bvult {} {}) {} {})",
                    rec(a),
                    rec(b),
                    one_hex,
                    zero_hex
                )
            }
            LatticeVMSymbolicExpr::And(a, b) => {
                format!("(bvand {} {})", rec(a), rec(b),)
            }
            LatticeVMSymbolicExpr::Or(a, b) => {
                format!("(bvor {} {})", rec(a), rec(b),)
            }
            LatticeVMSymbolicExpr::Xor(a, b) => {
                format!("(bvxor {} {})", rec(a), rec(b),)
            }
            LatticeVMSymbolicExpr::SRL(a, b) => {
                format!("(bvlshr {} {})", rec(a), rec(b),)
            }
            LatticeVMSymbolicExpr::SRLCarry(a, b) => {
                format!(
                    "(bvand {} (bvsub (bvshl {} {}) {}))",
                    rec(a),
                    one_hex,
                    rec(b),
                    one_hex
                )
            }
            LatticeVMSymbolicExpr::Flip(a) => {
                format!("(ite (= {} {}) {} {})", rec(a), zero_hex, one_hex, zero_hex)
            }
            LatticeVMSymbolicExpr::Neg(a) => {
                // Two's complement negation
                format!("(bvneg {})", rec(a))
            }
            LatticeVMSymbolicExpr::KoalaBearRange(a) => {
                let mut rec_t = |e| helper(e, row_id, n_rows, n_pvs, vars, prime, true);
                format!(
                    "(ite (bvult {} #x7f000001) {} {})",
                    rec_t(a),
                    zero_hex,
                    one_hex
                )
            }
            LatticeVMSymbolicExpr::BabyBearRange(a) => {
                let mut rec_t = |e| helper(e, row_id, n_rows, n_pvs, vars, prime, true);
                format!(
                    "(ite (bvult {} #x78000001) {} {})",
                    rec_t(a),
                    zero_hex,
                    one_hex
                )
            }
            LatticeVMSymbolicExpr::Msb(a) => {
                format!(
                    "((_ zero_extend {}) ((_ extract {} {}) {}))",
                    32 - 1,
                    32 - 1,
                    32 - 1,
                    rec(a),
                )
            }
            LatticeVMSymbolicExpr::WordAddU(a_vec, b_vec) => {
                let a_val = word_to_bv32(a_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                let b_val = word_to_bv32(b_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                format!("(bvadd {} {})", a_val, b_val)
            }
            LatticeVMSymbolicExpr::WordSubU(a_vec, b_vec) => {
                let a_val = word_to_bv32(a_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                let b_val = word_to_bv32(b_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                format!("(bvsub {} {})", a_val, b_val)
            }
            LatticeVMSymbolicExpr::WordMulhs(a_vec, b_vec) => {
                let a_val = word_to_bv32(a_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                let b_val = word_to_bv32(b_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                format!(
                    "((_ extract 63 32) \
            (bvmul ((_ sign_extend 32) {}) ((_ sign_extend 32) {})))",
                    a_val, b_val
                )
            }
            LatticeVMSymbolicExpr::WordMulhu(a_vec, b_vec) => {
                let a_val = word_to_bv32(a_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                let b_val = word_to_bv32(b_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                format!(
                    "((_ extract 63 32) \
            (bvmul ((_ zero_extend 32) {}) ((_ zero_extend 32) {})))",
                    a_val, b_val
                )
            }
            // 符号付き乗算の低位32ビット (bvmulは下位ビットに関しては符号の有無を問わない)
            LatticeVMSymbolicExpr::WordMultl(a_vec, b_vec) => {
                let a_val = word_to_bv32(a_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                let b_val = word_to_bv32(b_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                format!("(bvmul {} {})", a_val, b_val)
            }
            // 符号付き乗算の高位32ビット (WordMulhs と同じ挙動)
            LatticeVMSymbolicExpr::WordMulth(a_vec, b_vec) => {
                let a_val = word_to_bv32(a_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                let b_val = word_to_bv32(b_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                format!(
                    "((_ extract 63 32) (bvmul ((_ sign_extend 32) {}) ((_ sign_extend 32) {})))",
                    a_val, b_val
                )
            }
            // 符号なし乗算の低位32ビット
            LatticeVMSymbolicExpr::WordMultul(a_vec, b_vec) => {
                let a_val = word_to_bv32(a_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                let b_val = word_to_bv32(b_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                format!("(bvmul {} {})", a_val, b_val)
            }
            // 符号なし乗算の高位32ビット (WordMulhu と同じ挙動)
            LatticeVMSymbolicExpr::WordMultuh(a_vec, b_vec) => {
                let a_val = word_to_bv32(a_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                let b_val = word_to_bv32(b_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                format!(
                    "((_ extract 63 32) (bvmul ((_ zero_extend 32) {}) ((_ zero_extend 32) {})))",
                    a_val, b_val
                )
            }
            LatticeVMSymbolicExpr::WordMul(a_vec, b_vec) => {
                let a_val = word_to_bv32(a_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                let b_val = word_to_bv32(b_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                format!("(bvmul {} {})", a_val, b_val)
            }
            LatticeVMSymbolicExpr::WordSrl(a_vec, b_vec) => {
                let a_val = word_to_bv32(a_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                let b_val = word_to_bv32(b_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                format!("(bvlshr {} {})", a_val, b_val)
            }
            LatticeVMSymbolicExpr::WordAnd(a_vec, b_vec) => {
                let mut res = vec![];
                for (a, b) in a_vec.iter().zip(b_vec.iter()) {
                    let a_bv = rec(a);
                    let b_bv = rec(b);
                    res.push(format!("(bvand {} {})", a_bv, b_bv));
                }

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
                    let a_bv = rec(a);
                    let b_bv = rec(b);
                    res.push(format!("(bvor {} {})", a_bv, b_bv));
                }

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
                    let a_bv = rec(a);
                    let b_bv = rec(b);
                    res.push(format!("(bvxor {} {})", a_bv, b_bv));
                }

                let mut acc = res[0].clone();
                let mut factor: u32 = 1;
                for limb in res.iter().skip(1) {
                    factor = factor.wrapping_mul(256);
                    acc = format!("(bvadd {} (bvmul {} #x{:08x}))", acc, limb, factor);
                }
                acc
            }
            LatticeVMSymbolicExpr::WordDiv(a_vec, b_vec) => {
                let a_val = word_to_bv32(a_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                let b_val = word_to_bv32(b_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                format!("(bvudiv {} {})", a_val, b_val)
            }
            LatticeVMSymbolicExpr::WordSDiv(a_vec, b_vec) => {
                let a_val = word_to_bv32(a_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                let b_val = word_to_bv32(b_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                format!("(bvsdiv {} {})", a_val, b_val)
            }
            LatticeVMSymbolicExpr::WordLt(a_vec, b_vec) => {
                let a_val = word_to_bv32(a_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                let b_val = word_to_bv32(b_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                format!("(ite (bvult {} {}) {} {})", a_val, b_val, one_hex, zero_hex)
            }
            LatticeVMSymbolicExpr::WordSLt(a_vec, b_vec) => {
                let a_val = word_to_bv32(a_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                let b_val = word_to_bv32(b_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                format!("(ite (bvslt {} {}) {} {})", a_val, b_val, one_hex, zero_hex)
            }
            LatticeVMSymbolicExpr::WordSLe(a_vec, b_vec) => {
                let a_val = word_to_bv32(a_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                let b_val = word_to_bv32(b_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                format!("(ite (bvsle {} {}) {} {})", a_val, b_val, one_hex, zero_hex)
            }
            LatticeVMSymbolicExpr::WordEq(a_vec, b_vec) => {
                let a_val = word_to_bv32(a_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                let b_val = word_to_bv32(b_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                format!("(ite (= {} {}) {} {})", a_val, b_val, one_hex, zero_hex)
            }
            LatticeVMSymbolicExpr::WordNEq(a_vec, b_vec) => {
                let a_val = word_to_bv32(a_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                let b_val = word_to_bv32(b_vec, row_id, n_rows, n_pvs, vars, prime, not_field_op);
                format!("(ite (= {} {}) {} {})", a_val, b_val, zero_hex, one_hex)
            }
        }
    }

    let prime_hex = format!("#x{:08x}", prime);
    let zero_hex = format!("#x{:08x}", 0);
    let _one_hex = format!("#x{:08x}", 1);

    let mut vars = HashSet::new();
    let mut smt = String::new();
    smt.push_str("(set-logic QF_BV)\n");
    smt.push_str(&format!("(define-fun P () (_ BitVec 32) {})\n", prime_hex));
    smt.push_str(
        ";; finite field addition: (a + b) mod p
(define-fun ff_add ((a (_ BitVec 32)) (b (_ BitVec 32))) (_ BitVec 32)
  ((_ extract 31 0)
    (bvurem
      (bvadd ((_ zero_extend 1) a)
             ((_ zero_extend 1) b))
      ((_ zero_extend 1) P))))

;; finite field subtraction: (a - b) mod p
(define-fun ff_sub ((a (_ BitVec 32)) (b (_ BitVec 32))) (_ BitVec 32)
  ((_ extract 31 0)
    (bvurem
      (bvadd
        ((_ zero_extend 1) a)
        (bvsub ((_ zero_extend 1) P)
               ((_ zero_extend 1) b)))
      ((_ zero_extend 1) P))))

;; finite field multiplication: (a * b) mod p
(define-fun ff_mul ((a (_ BitVec 32)) (b (_ BitVec 32))) (_ BitVec 32)
  ((_ extract 31 0)
    (bvurem
      (bvmul ((_ zero_extend 32) a)
             ((_ zero_extend 32) b))
      ((_ zero_extend 32) P))))\n",
    );

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

    // Add modular constraints using bvurem
    for expr in &constraints.air_constraints {
        for i in 0..n_rows {
            let body = helper(expr, i, n_rows, n_pvs, &mut vars, prime, false);
            smt.push_str(&format!("(assert (= {} {}))\n", body, zero_hex));
        }
    }

    for expr in &constraints.lookup_constraints {
        for i in 0..n_rows {
            let body = helper(expr, i, n_rows, n_pvs, &mut vars, prime, false);
            smt.push_str(&format!("(assert (= {} {}))\n", body, zero_hex));
        }
    }

    for expr in &constraints.pv_pos_constraints {
        for i in 0..n_rows {
            let body = helper(expr, i, n_rows, n_pvs, &mut vars, prime, false);
            smt.push_str(&format!("(assert (= {} {}))\n", body, zero_hex));
        }
    }

    for expr in &constraints.pv_neg_constraints {
        for i in 0..n_rows {
            let body = helper(expr, i, n_rows, n_pvs, &mut vars, prime, false);
            smt.push_str(&format!("(assert (not (= {} {})))\n", body, zero_hex));
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
                "  (= (bvurem trace_{}_{} P) #x{:08x})\n",
                i,
                j,
                v.as_canonical_u32(prime)
            ));
        }
        smt.push_str(")))\n");
    }

    // ranges
    for (j, k) in range_types {
        if let RangeType::U16 = k {
            for i in 0..n_rows {
                smt.push_str(&format!("(assert (bvule trace_{}_{} #x0000ffff))\n", i, j,));
            }
        }
        if let RangeType::U8 = k {
            for i in 0..n_rows {
                smt.push_str(&format!("(assert (bvule trace_{}_{} #x000000ff))\n", i, j,));
            }
        }
        if let RangeType::U7 = k {
            for i in 0..n_rows {
                smt.push_str(&format!("(assert (bvule trace_{}_{} #x0000007f))\n", i, j,));
            }
        }
    }

    smt.push_str("(check-sat)\n(get-model)\n");
    smt
}
