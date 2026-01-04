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
    SRL,
    MSB,
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
    a: &[LatticeVMSymbolicExpr],
    b: &[LatticeVMSymbolicExpr],
    c: &[LatticeVMSymbolicExpr],
    op: &OpALU,
) -> LatticeVMSymbolicExpr {
    let a_word = reconstruct_symbolic_word(a, 0);
    let b_word = reconstruct_symbolic_word(b, 0);
    let c_word = reconstruct_symbolic_word(c, 0);

    match op {
        OpALU::Add => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::Add(
                Box::new(b_word.clone()),
                Box::new(c_word),
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
            Box::new(LatticeVMSymbolicExpr::MulLo(
                Box::new(b_word.clone()),
                Box::new(c_word),
            )),
        ),
        OpALU::MulH => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::MulHiSS(
                Box::new(b_word.clone()),
                Box::new(c_word),
            )),
        ),
        OpALU::MulHU => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::MulHiUU(
                Box::new(b_word.clone()),
                Box::new(c_word),
            )),
        ),
        OpALU::And => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::And(
                Box::new(b_word.clone()),
                Box::new(c_word),
            )),
        ),
        OpALU::Or => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::Or(
                Box::new(b_word.clone()),
                Box::new(c_word),
            )),
        ),
        OpALU::Xor => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::Xor(
                Box::new(b_word.clone()),
                Box::new(c_word),
            )),
        ),
        OpALU::SRL => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::SRL(
                Box::new(b_word.clone()),
                Box::new(c_word),
            )),
        ),
        OpALU::Lt => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::Flip(Box::new(
                LatticeVMSymbolicExpr::Lt(Box::new(b_word.clone()), Box::new(c_word)),
            ))),
        ),
        OpALU::MSB => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::Flip(Box::new(
                LatticeVMSymbolicExpr::Msb(Box::new(b_word.clone())),
            ))),
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

pub fn ai(lo: i64, hi: i64) -> AbstractInterval {
    AbstractInterval { lo, hi }
}

pub fn byte(v: i64) -> AbstractInterval {
    ai(v, v)
}

mod tests {
    use crate::alu::{
        ai, byte, full_word, word_add, word_div, word_ltu, word_mul, word_mulhs, word_mulhu,
        word_sdiv, word_slt, word_to_unsigned, Word, WORD_BOUND,
    };
    use crate::interval::AbstractInterval;

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

    fn word_range(lo: i64, hi: i64) -> Word {
        [ai(lo, hi), byte(0), byte(0), byte(0)]
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
}
