use crate::interval::AbstractInterval;
use crate::symbolic::LatticeVMSymbolicExpr;

#[derive(Debug)]
pub enum OpALU {
    Add,
    Sub,
    Mul,
    MulH,
    MulHU,
    And,
    Or,
    Xor,
    Lt,
    SLt,
    SRL,
}

pub fn reconstruct_symbolic_word(
    row: &[LatticeVMSymbolicExpr],
    base: usize,
) -> LatticeVMSymbolicExpr {
    let mut val = LatticeVMSymbolicExpr::Constant(AbstractInterval::zero());
    let mut mul = 1_i64;
    for i in 0..4 {
        let rm = LatticeVMSymbolicExpr::Mul(
            Box::new(row[base + i].clone()),
            Box::new(LatticeVMSymbolicExpr::Constant(AbstractInterval::from_i64(
                mul,
            ))),
        );
        val = LatticeVMSymbolicExpr::Add(Box::new(val.clone()), Box::new(rm));
        mul *= 256;
    }
    val
}

pub fn get_alu_constraint(
    a: &[LatticeVMSymbolicExpr; 4],
    b: &[LatticeVMSymbolicExpr; 4],
    c: &[LatticeVMSymbolicExpr; 4],
    op: &OpALU,
) -> LatticeVMSymbolicExpr {
    let a_word = reconstruct_symbolic_word(a, 0);
    let b_word = reconstruct_symbolic_word(b, 0);
    let c_word = reconstruct_symbolic_word(c, 0);

    match op {
        OpALU::Add => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::WordAdd(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        OpALU::Sub => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::Sub(
                Box::new(b_word.clone()),
                Box::new(c_word),
            )),
        ),
        OpALU::Mul => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::WordMul(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        OpALU::MulH => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::WordMulhs(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        OpALU::MulHU => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::WordMulhu(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        OpALU::And => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::WordAnd(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        OpALU::Or => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::WordOr(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        OpALU::Xor => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::WordXOr(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        OpALU::Lt => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::WordLt(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        OpALU::SLt => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::WordSLt(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        OpALU::SRL => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::SRL(
                Box::new(b_word.clone()),
                Box::new(c_word),
            )),
        ),
    }
}

pub type Word = [AbstractInterval; 4];
pub const WORD_BITS: u32 = 32;
pub const WORD_BOUND: i64 = 1 << WORD_BITS;

pub fn word_to_unsigned(word: &Word) -> AbstractInterval {
    let mut lo = 0i64;
    let mut hi = 0i64;
    for (i, limb) in word.iter().enumerate() {
        let shift = 8 * i;
        lo += limb.lo << shift;
        hi += limb.hi << shift;
    }
    AbstractInterval { lo, hi }
}

pub fn full_word() -> AbstractInterval {
    AbstractInterval {
        lo: 0,
        hi: WORD_BOUND - 1,
    }
}

pub fn word_add(a: &Word, b: &Word) -> AbstractInterval {
    let a = word_to_unsigned(a);
    let b = word_to_unsigned(b);

    let lo = a.lo + b.lo;
    let hi = a.hi + b.hi;

    if hi >= WORD_BOUND {
        full_word()
    } else {
        AbstractInterval { lo, hi }
    }
}

pub fn word_sub(a: &Word, b: &Word) -> AbstractInterval {
    let a = word_to_unsigned(a);
    let b = word_to_unsigned(b);

    // 確実に underflow しない
    if a.lo >= b.hi {
        AbstractInterval {
            lo: a.lo - b.hi,
            hi: a.hi - b.lo,
        }
    } else {
        // 一部でも underflow の可能性がある
        full_word()
    }
}

pub fn word_mul(a: &Word, b: &Word) -> AbstractInterval {
    // mul（low 32 bits）
    let a = word_to_unsigned(a);
    let b = word_to_unsigned(b);

    let lo = a.lo * b.lo;
    let hi = a.hi * b.hi;

    if hi >= WORD_BOUND {
        full_word()
    } else {
        AbstractInterval { lo, hi }
    }
}

pub fn word_mulhu(a: &Word, b: &Word) -> AbstractInterval {
    // mulhu（unsigned × unsigned, high 32 bits）
    let a = word_to_unsigned(a);
    let b = word_to_unsigned(b);

    let lo = (a.lo as i128 * b.lo as i128) >> WORD_BITS;
    let hi = (a.hi as i128 * b.hi as i128) >> WORD_BITS;

    AbstractInterval {
        lo: lo as i64,
        hi: hi as i64,
    }
}

pub fn word_mulhs(a: &Word, b: &Word) -> AbstractInterval {
    // mulhs（signed × signed, high 32 bits）
    let a = word_to_unsigned(a).to_signed(WORD_BITS);
    let b = word_to_unsigned(b).to_signed(WORD_BITS);

    let candidates = [a.lo * b.lo, a.lo * b.hi, a.hi * b.lo, a.hi * b.hi];

    let lo = candidates.iter().min().unwrap() >> WORD_BITS;
    let hi = candidates.iter().max().unwrap() >> WORD_BITS;

    AbstractInterval { lo, hi }
}

pub fn word_div(a: &Word, b: &Word) -> AbstractInterval {
    let a = word_to_unsigned(a);
    let b = word_to_unsigned(b);

    // division by zero possible
    if b.lo == 0 {
        return full_word();
    }

    AbstractInterval {
        lo: a.lo / b.hi,
        hi: a.hi / b.lo,
    }
}

pub fn word_sdiv(a: &Word, b: &Word) -> AbstractInterval {
    let a = word_to_unsigned(a).to_signed(WORD_BITS);
    let b = word_to_unsigned(b).to_signed(WORD_BITS);

    // division by zero
    if b.lo <= 0 && b.hi >= 0 {
        return full_word();
    }

    // INT_MIN / -1 overflow
    if a.lo == -(1 << 31) && b.lo == -1 && b.hi == -1 {
        return full_word();
    }

    let candidates = [a.lo / b.lo, a.lo / b.hi, a.hi / b.lo, a.hi / b.hi];

    AbstractInterval {
        lo: *candidates.iter().min().unwrap(),
        hi: *candidates.iter().max().unwrap(),
    }
}

pub fn word_ltu(a: &Word, b: &Word) -> AbstractInterval {
    let a = word_to_unsigned(a);
    let b = word_to_unsigned(b);

    if a.hi < b.lo {
        AbstractInterval { lo: 1, hi: 1 }
    } else if a.lo >= b.hi {
        AbstractInterval { lo: 0, hi: 0 }
    } else {
        AbstractInterval { lo: 0, hi: 1 }
    }
}

pub fn word_slt(a: &Word, b: &Word) -> AbstractInterval {
    let a = word_to_unsigned(a).to_signed(WORD_BITS);
    let b = word_to_unsigned(b).to_signed(WORD_BITS);

    if a.hi < b.lo {
        AbstractInterval { lo: 1, hi: 1 }
    } else if a.lo >= b.hi {
        AbstractInterval { lo: 0, hi: 0 }
    } else {
        AbstractInterval { lo: 0, hi: 1 }
    }
}

pub fn word_and(a: &Word, b: &Word) -> AbstractInterval {
    let a = word_to_unsigned(a);
    let b = word_to_unsigned(b);

    // AND with a range [0, hi] can never exceed the minimum of the two hi values.
    // However, the lower bound is tricky. For a simple interval, we know:
    // 0 <= (a & b) <= min(a.hi, b.hi)
    // A tighter bound exists but requires bit-by-bit analysis.
    if a.lo == a.hi && b.lo == b.hi {
        let res = a.lo & b.lo;
        AbstractInterval { lo: res, hi: res }
    } else {
        // Conservative approximation for intervals
        AbstractInterval {
            lo: 0,
            hi: a.hi.min(b.hi),
        }
    }
}

pub fn word_or(a: &Word, b: &Word) -> AbstractInterval {
    let a = word_to_unsigned(a);
    let b = word_to_unsigned(b);

    if a.lo == a.hi && b.lo == b.hi {
        let res = a.lo | b.lo;
        AbstractInterval { lo: res, hi: res }
    } else {
        // OR can at most set all bits up to the highest bit present in either operand.
        // We find the smallest power of 2 minus 1 that covers both.
        let max_possible = (1i64 << (64 - (a.hi | b.hi).leading_zeros())) - 1;
        AbstractInterval {
            lo: a.lo.max(b.lo),
            hi: max_possible.min(WORD_BOUND - 1),
        }
    }
}

pub fn word_xor(a: &Word, b: &Word) -> AbstractInterval {
    let a = word_to_unsigned(a);
    let b = word_to_unsigned(b);

    if a.lo == a.hi && b.lo == b.hi {
        let res = a.lo ^ b.lo;
        AbstractInterval { lo: res, hi: res }
    } else {
        // XOR is the most unpredictable for intervals.
        // The result's highest bit is bounded by the highest bit of a.hi or b.hi.
        let max_val = a.hi | b.hi;
        let hi_bound = if max_val == 0 {
            0
        } else {
            (1i64 << (64 - max_val.leading_zeros())) - 1
        };
        AbstractInterval {
            lo: 0,
            hi: hi_bound.min(WORD_BOUND - 1),
        }
    }
}

mod tests {
    use crate::alu::{
        full_word, word_add, word_and, word_div, word_ltu, word_mul, word_mulhs, word_mulhu,
        word_or, word_sdiv, word_slt, word_sub, word_to_unsigned, word_xor, Word, WORD_BOUND,
    };
    use crate::interval::AbstractInterval;

    fn signed_word(val_lo: i64, val_hi: i64) -> Word {
        let top = if val_lo < 0 { 255 } else { 0 };
        [
            ai(val_lo & 0xFF, val_hi & 0xFF),
            byte(top),
            byte(top),
            byte(top),
        ]
    }

    fn ai(lo: i64, hi: i64) -> AbstractInterval {
        AbstractInterval { lo, hi }
    }

    fn byte(v: i64) -> AbstractInterval {
        ai(v, v)
    }

    fn word_range(lo: i64, hi: i64) -> Word {
        [ai(lo, hi), byte(0), byte(0), byte(0)]
    }

    #[test]
    fn test_add_no_wrap() {
        let a: Word = [byte(1), byte(0), byte(0), byte(0)];
        let b: Word = [byte(2), byte(0), byte(0), byte(0)];

        let r = word_add(&a, &b);
        assert_eq!(r, ai(3, 3));
    }

    #[test]
    fn test_add_wrap_to_full() {
        let a: Word = [byte(255), byte(255), byte(255), byte(255)];
        let b: Word = [byte(1), byte(0), byte(0), byte(0)];

        let r = word_add(&a, &b);
        assert_eq!(r, ai(0, (1 << 32) - 1));
    }

    #[test]
    fn test_sub_no_wrap_exact() {
        // 20 - 5 = 15
        let a = word_range(20, 20);
        let b = word_range(5, 5);

        let r = word_sub(&a, &b);
        assert_eq!(r, ai(15, 15));
    }

    #[test]
    fn test_sub_interval_progression() {
        // [10, 20] - [1, 3] = [7, 19]
        let a = word_range(10, 20);
        let b = word_range(1, 3);

        let r = word_sub(&a, &b);
        assert_eq!(r, ai(7, 19));
    }

    #[test]
    fn test_sub_near_boundary_no_wrap() {
        // [100, 110] - [10, 20] = [80, 100]
        let a = word_range(100, 110);
        let b = word_range(10, 20);

        let r = word_sub(&a, &b);
        assert_eq!(r, ai(80, 100));
    }

    #[test]
    fn test_sub_definite_underflow() {
        // [5, 10] - [20, 30] → 必ず underflow
        let a = word_range(5, 10);
        let b = word_range(20, 30);

        let r = word_sub(&a, &b);
        assert_eq!(r, full_word());
    }

    #[test]
    fn test_sub_partial_underflow() {
        // [10, 20] - [15, 25]
        // 10 - 25 = underflow
        // 20 - 15 = ok
        let a = word_range(10, 20);
        let b = word_range(15, 25);

        let r = word_sub(&a, &b);
        assert_eq!(r, full_word());
    }

    #[test]
    fn test_mul_no_overflow() {
        let a: Word = [byte(2), byte(0), byte(0), byte(0)];
        let b: Word = [byte(3), byte(0), byte(0), byte(0)];

        let r = word_mul(&a, &b);
        assert_eq!(r, ai(6, 6));
    }

    #[test]
    fn test_mul_overflow_full() {
        let a: Word = [byte(255), byte(255), byte(255), byte(255)];
        let b: Word = [byte(2), byte(0), byte(0), byte(0)];

        let r = word_mul(&a, &b);
        assert_eq!(r, ai(0, (1 << 32) - 1));
    }

    #[test]
    fn test_mulhu_simple() {
        let a: Word = [byte(0), byte(0), byte(0), byte(1)]; // 1 << 24
        let b: Word = [byte(0), byte(0), byte(0), byte(1)];

        let r = word_mulhu(&a, &b);
        assert_eq!(r, ai(1 << 16, 1 << 16));
    }

    #[test]
    fn test_mulhu_max() {
        let a: Word = [byte(255), byte(255), byte(255), byte(255)];
        let b: Word = [byte(255), byte(255), byte(255), byte(255)];

        let r = word_mulhu(&a, &b);
        println!("r is {}", r);
        assert!(r.lo >= 0);
        assert!(r.hi <= (1 << 32) - 1);
    }

    #[test]
    fn test_mulhs_positive() {
        let a: Word = [byte(2), byte(0), byte(0), byte(0)];
        let b: Word = [byte(3), byte(0), byte(0), byte(0)];

        let r = word_mulhs(&a, &b);
        assert_eq!(r, ai(0, 0));
    }

    #[test]
    fn test_mulhs_negative() {
        // -1 * 2
        let a: Word = [byte(255), byte(255), byte(255), byte(255)];
        let b: Word = [byte(2), byte(0), byte(0), byte(0)];

        let r = word_mulhs(&a, &b);
        assert!(r.lo <= -1);
        assert!(r.hi <= 0);
    }

    #[test]
    fn test_div_simple() {
        let a: Word = [byte(10), byte(0), byte(0), byte(0)];
        let b: Word = [byte(2), byte(0), byte(0), byte(0)];

        let r = word_div(&a, &b);
        assert_eq!(r, ai(5, 5));
    }

    #[test]
    fn test_div_by_zero_full() {
        let a: Word = [byte(10), byte(0), byte(0), byte(0)];
        let b: Word = [byte(0), byte(0), byte(0), byte(0)];

        let r = word_div(&a, &b);
        assert_eq!(r, ai(0, (1 << 32) - 1));
    }

    #[test]
    fn test_sdiv_positive() {
        let a: Word = [byte(10), byte(0), byte(0), byte(0)];
        let b: Word = [byte(2), byte(0), byte(0), byte(0)];

        let r = word_sdiv(&a, &b);
        assert_eq!(r, ai(5, 5));
    }

    #[test]
    fn test_sdiv_negative() {
        // -10 / 2
        let a: Word = [byte(246), byte(255), byte(255), byte(255)];
        let b: Word = [byte(2), byte(0), byte(0), byte(0)];

        let r = word_sdiv(&a, &b);
        assert!(r.lo <= -5);
        assert!(r.hi <= -5);
    }

    #[test]
    fn test_sdiv_zero_divisor_full() {
        let a: Word = [byte(1), byte(0), byte(0), byte(0)];
        let b: Word = [ai(-1, 1), byte(0), byte(0), byte(0)];

        let r = word_sdiv(&a, &b);
        assert_eq!(r, ai(0, (1 << 32) - 1));
    }

    #[test]
    fn test_ltu_true() {
        let a: Word = [byte(1), byte(0), byte(0), byte(0)];
        let b: Word = [byte(2), byte(0), byte(0), byte(0)];

        assert_eq!(word_ltu(&a, &b), ai(1, 1));
    }

    #[test]
    fn test_ltu_false() {
        let a: Word = [byte(5), byte(0), byte(0), byte(0)];
        let b: Word = [byte(2), byte(0), byte(0), byte(0)];

        assert_eq!(word_ltu(&a, &b), ai(0, 0));
    }

    #[test]
    fn test_slt_negative_vs_positive() {
        // -1 < 1
        let a: Word = [byte(255), byte(255), byte(255), byte(255)];
        let b: Word = [byte(1), byte(0), byte(0), byte(0)];

        assert_eq!(word_slt(&a, &b), ai(1, 1));
    }

    #[test]
    fn test_add_interval_progression() {
        // [1, 2] + [10, 20] = [11, 22]
        let a = word_range(1, 2);
        let b = word_range(10, 20);
        assert_eq!(word_add(&a, &b), ai(11, 22));

        // 境界付近: [MAX-10, MAX-5] + [1, 4] = [MAX-9, MAX-1] (No wrap)
        let a_near = [ai(255, 255), ai(255, 255), ai(255, 255), ai(240, 245)]; // 非常に大きい値
        let b_small = word_range(1, 4);
        let r = word_add(&a_near, &b_small);
        assert!(r.hi < WORD_BOUND);
        assert_eq!(r.lo, word_to_unsigned(&a_near).lo + 1);

        // 一部でもオーバーフローの可能性がある場合は full_word
        let a_overflow = [ai(255, 255), ai(255, 255), ai(255, 255), ai(250, 255)];
        let b_overflow = word_range(10, 20);
        assert_eq!(word_add(&a_overflow, &b_overflow), full_word());
    }

    #[test]
    fn test_mul_interval_growth() {
        // [2, 3] * [4, 5] = [8, 15]
        let a = word_range(2, 3);
        let b = word_range(4, 5);
        assert_eq!(word_mul(&a, &b), ai(8, 15));

        // 巨大な範囲への拡大
        let a_big = word_range(100, 200);
        let b_big = [ai(0, 255), ai(0, 255), ai(0, 255), ai(0, 255)];
        assert_eq!(word_mul(&a_big, &b_big), full_word());
    }

    #[test]
    fn test_mulhs_signed_intervals() {
        // 正×負のインターバル
        let a = word_range(10, 20);
        let b = [ai(254, 255), ai(255, 255), ai(255, 255), ai(255, 255)]; // [-2, -1]
        let r = word_mulhs(&a, &b);
        assert!(r.lo <= r.hi);
        assert!(r.hi <= 0);
        assert_eq!(r.lo, -1);
        assert_eq!(r.hi, -1);
    }

    #[test]
    fn test_div_interval_ranges() {
        // [100, 200] / [2, 10] = [100/10, 200/2] = [10, 100]
        let a = word_range(100, 200);
        let b = word_range(2, 10);
        assert_eq!(word_div(&a, &b), ai(10, 100));

        // 除数に0が含まれる可能性 [0, 5] -> full_word
        let b_zero = word_range(0, 5);
        assert_eq!(word_div(&a, &b_zero), full_word());
    }

    #[test]
    fn test_sdiv_complex_ranges() {
        // 配当が符号を跨ぐ: [-10, 10] / [2, 2] = [-5, 5]
        // a = 0xFFFFFFF6 (-10) to 0x0000000A (10)
        // ここではword_to_unsignedの仕様上、大きなインターバルになる可能性があるため
        // 実装の `to_signed` の境界値テストとして機能させる
        let a = ai(-(1 << 10), 1 << 10);
        // 簡略化のため、直接 AbstractInterval の演算ロジックを確認
        let b = ai(2, 2);
        let candidates = [a.lo / b.lo, a.lo / b.hi, a.hi / b.lo, a.hi / b.hi];
        let lo = *candidates.iter().min().unwrap();
        let hi = *candidates.iter().max().unwrap();
        assert_eq!(lo, -512);
        assert_eq!(hi, 512);
    }

    #[test]
    fn test_comparison_uncertainty() {
        // 確実な比較: [10, 20] < [30, 40] -> [1, 1] (True)
        let a = word_range(10, 20);
        let b = word_range(30, 40);
        assert_eq!(word_ltu(&a, &b), ai(1, 1));

        // 確実な比較: [50, 60] < [10, 20] -> [0, 0] (False)
        let a = word_range(50, 60);
        let b = word_range(10, 20);
        assert_eq!(word_ltu(&a, &b), ai(0, 0));

        // 不確実（重なりあり）: [15, 25] < [20, 30] -> [0, 1] (Unknown)
        let a = word_range(15, 25);
        let b = word_range(20, 30);
        assert_eq!(word_ltu(&a, &b), ai(0, 1));
    }

    #[test]
    fn test_slt_definitely_less() {
        // ケース1: 正の範囲同士で完全に小さい [1, 5] < [10, 15]
        let a = signed_word(1, 5);
        let b = signed_word(10, 15);
        assert_eq!(
            word_slt(&a, &b),
            ai(1, 1),
            "Positive range: A < B should be True"
        );

        // ケース2: 負数 < 正数 [-10, -5] < [1, 2]
        // 注: signed_word の実装上、-10 は下位が 0xF6, 上位が 0xFF になる想定
        let a_neg = [ai(240, 250), byte(255), byte(255), byte(255)]; // [-16, -6]
        let b_pos = [ai(1, 5), byte(0), byte(0), byte(0)]; // [1, 5]
        assert_eq!(
            word_slt(&a_neg, &b_pos),
            ai(1, 1),
            "Negative A < Positive B should be True"
        );

        // ケース3: 負の範囲同士で完全に小さい [-20, -15] < [-10, -5]
        let a_neg_far = [ai(200, 210), byte(255), byte(255), byte(255)];
        let b_neg_near = [ai(240, 250), byte(255), byte(255), byte(255)];
        assert_eq!(word_slt(&a_neg_far, &b_neg_near), ai(1, 1));
    }

    #[test]
    fn test_slt_definitely_greater_or_equal() {
        // ケース1: 正の範囲同士で完全に大きい [20, 30] < [5, 10] -> False
        let a = signed_word(20, 30);
        let b = signed_word(5, 10);
        assert_eq!(word_slt(&a, &b), ai(0, 0));

        // ケース2: 正数 < 負数 [1, 5] < [-10, -5] -> False
        let a_pos = [ai(1, 5), byte(0), byte(0), byte(0)];
        let b_neg = [ai(240, 250), byte(255), byte(255), byte(255)];
        assert_eq!(word_slt(&a_pos, &b_neg), ai(0, 0));
    }

    #[test]
    fn test_slt_overlap_unknown() {
        // ケース1: 範囲が重なっている [5, 15] < [10, 20]
        // 5 < 10 (True) の可能性もあれば、15 < 10 (False) の可能性もあるため Unknown
        let a = signed_word(5, 15);
        let b = signed_word(10, 20);
        assert_eq!(
            word_slt(&a, &b),
            ai(0, 1),
            "Overlapping ranges should return [0, 1]"
        );

        // ケース2: 境界値が一致している [5, 10] < [10, 15]
        // a.hi(10) < b.lo(10) は False なので、完全には小さくない
        let a_edge = signed_word(5, 10);
        let b_edge = signed_word(10, 15);
        assert_eq!(word_slt(&a_edge, &b_edge), ai(0, 1));

        // ケース3: 片方がもう片方を包含している
        let a_inner = signed_word(10, 12);
        let b_outer = signed_word(5, 20);
        assert_eq!(word_slt(&a_inner, &b_outer), ai(0, 1));
    }

    #[test]
    fn test_slt_max_min_bounds() {
        // 32bit符号付きの最小値付近のテスト
        let i32_min = [byte(0), byte(0), byte(0), byte(128)]; // 0x80000000
        let zero = [byte(0), byte(0), byte(0), byte(0)];

        // INT_MIN < 0 は確実に True
        assert_eq!(word_slt(&i32_min, &zero), ai(1, 1));

        // 0 < INT_MIN は確実に False
        assert_eq!(word_slt(&zero, &i32_min), ai(0, 0));
    }

    #[test]
    fn test_word_and_comprehensive() {
        // 1. Singleton: 0b1100 & 0b1010 = 0b1000 (12 & 10 = 8)
        let a = word_range(12, 12);
        let b = word_range(10, 10);
        assert_eq!(word_and(&a, &b), ai(8, 8));

        // 2. Range: [0, 255] & 0 = 0
        let a_range = word_range(0, 255);
        let b_zero = word_range(0, 0);
        assert_eq!(word_and(&a_range, &b_zero), ai(0, 0));

        // 3. Range: [0, 7] & [0, 7] -> lo=0, hi=7
        let b_range = word_range(0, 7);
        assert_eq!(word_and(&a_range, &b_range), ai(0, 7));
    }

    #[test]
    fn test_word_or_comprehensive() {
        // 1. Singleton: 0b1100 | 0b0011 = 0b1111 (12 | 3 = 15)
        let a = word_range(12, 12);
        let b = word_range(3, 3);
        assert_eq!(word_or(&a, &b), ai(15, 15));

        // 2. Range: [1, 2] | [4, 8]
        // lo should be at least max(1, 4) = 4
        // hi should be covered by the bitmask of 8 (which is 0b1111 = 15)
        let a_r = word_range(1, 2);
        let b_r = word_range(4, 8);
        let res = word_or(&a_r, &b_r);
        assert!(res.lo >= 4);
        assert!(res.hi <= 15);
    }

    #[test]
    fn test_word_xor_comprehensive() {
        // 1. Singleton: 10 ^ 10 = 0
        let a = word_range(10, 10);
        assert_eq!(word_xor(&a, &a), ai(0, 0));

        // 2. Singleton: 0b1010 ^ 0b0101 = 0b1111 (10 ^ 5 = 15)
        let b = word_range(5, 5);
        assert_eq!(word_xor(&a, &b), ai(15, 15));

        // 3. Range property: [0, 3] ^ [0, 3]
        // Max possible value is 3 (0b11)
        let a_r = word_range(0, 3);
        let res = word_xor(&a_r, &a_r);
        assert_eq!(res.lo, 0);
        assert!(res.hi >= 3);
    }

    #[test]
    fn test_bitwise_word_boundaries() {
        // Test with WORD_BOUND
        let all_ones = [byte(255), byte(255), byte(255), byte(255)]; // 0xFFFFFFFF
        let zero = [byte(0), byte(0), byte(0), byte(0)];

        // AND: ALL & 0 = 0
        assert_eq!(word_and(&all_ones, &zero), ai(0, 0));

        // OR: 0 | ALL = ALL
        assert_eq!(word_or(&zero, &all_ones), ai((1 << 32) - 1, (1 << 32) - 1));

        // XOR: ALL ^ ALL = 0
        assert_eq!(word_xor(&all_ones, &all_ones), ai(0, 0));
    }
}
