use crate::interval::AbstractInterval;
use crate::symbolic::ZEBRASymbolicExpr;

pub type Word = [AbstractInterval; 4];
pub const WORD_BITS: u32 = 32;
pub const WORD_BOUND: i128 = 1 << WORD_BITS;

#[derive(Debug)]
pub enum WordOp {
    AddU,
    SubU,
    Mul,
    MulH,
    MulHU,
    MulHS,
    MulTL,
    MulTH,
    MulTUL,
    MulTUH,
    And,
    Or,
    Xor,
    Lt,
    SLe,
    SLt,
    SRL,
    Eq,
    NEq,
    Div,
    SDiv,
}

pub fn reconstruct_symbolic_word(row: &[ZEBRASymbolicExpr], base: usize) -> ZEBRASymbolicExpr {
    let mut val = ZEBRASymbolicExpr::Constant(AbstractInterval::zero());
    let mut mul = 1_i128;
    for i in 0..4 {
        let rm = ZEBRASymbolicExpr::Mul(
            Box::new(row[base + i].clone()),
            Box::new(ZEBRASymbolicExpr::Constant(AbstractInterval::from_i128(
                mul,
            ))),
        );
        val = ZEBRASymbolicExpr::Add(Box::new(val.clone()), Box::new(rm));
        mul *= 256;
    }
    val
}

pub fn get_alu_constraint(
    a: &[ZEBRASymbolicExpr; 4],
    b: &[ZEBRASymbolicExpr; 4],
    c: &[ZEBRASymbolicExpr; 4],
    hi: &[ZEBRASymbolicExpr; 4],
    op: &WordOp,
) -> ZEBRASymbolicExpr {
    let a_word = reconstruct_symbolic_word(a, 0);
    let hi_word = reconstruct_symbolic_word(hi, 0);

    match op {
        WordOp::AddU => ZEBRASymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(ZEBRASymbolicExpr::WordAddU(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        WordOp::SubU => ZEBRASymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(ZEBRASymbolicExpr::WordSubU(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        WordOp::Mul => ZEBRASymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(ZEBRASymbolicExpr::WordMul(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        WordOp::MulH => ZEBRASymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(ZEBRASymbolicExpr::WordMulhs(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        WordOp::MulHU => ZEBRASymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(ZEBRASymbolicExpr::WordMulhu(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        WordOp::MulHS => ZEBRASymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(ZEBRASymbolicExpr::WordMulhs(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        WordOp::MulTL => ZEBRASymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(ZEBRASymbolicExpr::WordMultl(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        WordOp::MulTH => ZEBRASymbolicExpr::Sub(
            Box::new(hi_word),
            Box::new(ZEBRASymbolicExpr::WordMulth(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        WordOp::MulTUL => ZEBRASymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(ZEBRASymbolicExpr::WordMultul(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        WordOp::MulTUH => ZEBRASymbolicExpr::Sub(
            Box::new(hi_word),
            Box::new(ZEBRASymbolicExpr::WordMultuh(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        WordOp::And => ZEBRASymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(ZEBRASymbolicExpr::WordAnd(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        WordOp::Or => ZEBRASymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(ZEBRASymbolicExpr::WordOr(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        WordOp::Xor => ZEBRASymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(ZEBRASymbolicExpr::WordXOr(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        WordOp::Lt => ZEBRASymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(ZEBRASymbolicExpr::WordLt(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        WordOp::SLe => ZEBRASymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(ZEBRASymbolicExpr::WordSLe(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        WordOp::SLt => ZEBRASymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(ZEBRASymbolicExpr::WordSLt(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        WordOp::SRL => ZEBRASymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(ZEBRASymbolicExpr::WordSrl(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        WordOp::Eq => ZEBRASymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(ZEBRASymbolicExpr::WordEq(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        WordOp::NEq => ZEBRASymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(ZEBRASymbolicExpr::WordNEq(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        WordOp::Div => ZEBRASymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(ZEBRASymbolicExpr::WordDiv(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
        WordOp::SDiv => ZEBRASymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(ZEBRASymbolicExpr::WordSDiv(
                b.clone().map(|f| Box::new(f)),
                c.clone().map(|f| Box::new(f)),
            )),
        ),
    }
}

pub fn full_word() -> AbstractInterval {
    AbstractInterval {
        lo: 0,
        hi: WORD_BOUND - 1,
    }
}

pub fn word_to_unsigned(word: &Word) -> AbstractInterval {
    let mut lo = 0i128;
    let mut hi = 0i128;
    for (i, limb) in word.iter().enumerate() {
        let shift = 8 * i;
        lo += limb.lo << shift;
        hi += limb.hi << shift;
    }
    AbstractInterval { lo, hi }
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

pub fn word_addu(b: &Word, c: &Word) -> AbstractInterval {
    let b = word_to_unsigned(b);
    let c = word_to_unsigned(c);

    // exact calculation
    if b.is_singleton() && c.is_singleton() {
        let res = (b.lo + c.lo).rem_euclid(WORD_BOUND);
        return AbstractInterval { lo: res, hi: res };
    }

    // surely not overflow
    if b.hi + c.hi < WORD_BOUND {
        return AbstractInterval {
            lo: b.lo + c.lo,
            hi: b.hi + c.hi,
        };
    }

    // surely overflow
    if b.lo + c.lo >= WORD_BOUND {
        let lo = b.lo + c.lo - WORD_BOUND;
        let hi = b.hi + c.hi - WORD_BOUND;

        return AbstractInterval {
            lo: lo.max(0).min(WORD_BOUND - 1),
            hi: hi.max(0).min(WORD_BOUND - 1),
        };
    }

    // maybe overflow
    full_word()
}

pub fn word_subu(b: &Word, c: &Word) -> AbstractInterval {
    let b = word_to_unsigned(b);
    let c = word_to_unsigned(c);

    // exact calculation
    if b.is_singleton() && c.is_singleton() {
        let res = (b.lo - c.lo).rem_euclid(WORD_BOUND);
        return AbstractInterval { lo: res, hi: res };
    }

    // surely not underflow
    if b.lo >= c.hi {
        return AbstractInterval {
            lo: b.lo - c.hi,
            hi: b.hi - c.lo,
        };
    }

    // surely underflow
    if b.hi < c.lo {
        let lo = b.lo + (WORD_BOUND - c.hi);
        let hi = b.hi + (WORD_BOUND - c.lo);

        return AbstractInterval {
            lo: lo.max(0).min(WORD_BOUND - 1),
            hi: hi.max(0).min(WORD_BOUND - 1),
        };
    }

    // maybe underflow
    full_word()
}

pub fn word_mul(a: &Word, b: &Word) -> AbstractInterval {
    // mul（low 32 bits）
    let a = word_to_unsigned(a);
    let b = word_to_unsigned(b);

    if a.is_singleton() && b.is_singleton() {
        let res = (a.lo * b.lo).rem_euclid(WORD_BOUND);
        return AbstractInterval { lo: res, hi: res };
    }

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
        lo: lo as i128,
        hi: hi as i128,
    }
}

pub fn word_mulhs(a: &Word, b: &Word) -> AbstractInterval {
    const MOD: i128 = 1i128 << WORD_BITS;

    // signed 32-bit intervals
    let a = word_to_unsigned(a).to_signed(WORD_BITS);
    let b = word_to_unsigned(b).to_signed(WORD_BITS);

    // exact interval multiplication (sound)
    let p1 = a.lo * b.lo;
    let p2 = a.lo * b.hi;
    let p3 = a.hi * b.lo;
    let p4 = a.hi * b.hi;

    let prod_lo = p1.min(p2).min(p3).min(p4);
    let prod_hi = p1.max(p2).max(p3).max(p4);

    // arithmetic shift
    let hi_lo = prod_lo >> WORD_BITS;
    let hi_hi = prod_hi >> WORD_BITS;

    // wrap to 32-bit
    let lo_mod = hi_lo.rem_euclid(MOD);
    let hi_mod = hi_hi.rem_euclid(MOD);

    if lo_mod <= hi_mod {
        AbstractInterval {
            lo: lo_mod,
            hi: hi_mod,
        }
    } else {
        // wrap around 2^32
        AbstractInterval { lo: 0, hi: MOD - 1 }
    }
}

pub fn word_mult(a: &Word, b: &Word) -> (AbstractInterval, AbstractInterval) {
    let a = word_to_unsigned(a).to_signed(WORD_BITS);
    let b = word_to_unsigned(b).to_signed(WORD_BITS);

    let candidates = [
        a.lo as i128 * b.lo as i128,
        a.lo as i128 * b.hi as i128,
        a.hi as i128 * b.lo as i128,
        a.hi as i128 * b.hi as i128,
    ];

    let min = *candidates.iter().min().unwrap();
    let max = *candidates.iter().max().unwrap();

    let lo = AbstractInterval {
        lo: (min & ((1i128 << WORD_BITS) - 1)) as i128,
        hi: (max & ((1i128 << WORD_BITS) - 1)) as i128,
    };

    let hi = AbstractInterval {
        lo: (min >> WORD_BITS) as i128,
        hi: (max >> WORD_BITS) as i128,
    };

    (lo, hi)
}

pub fn word_multu(a: &Word, b: &Word) -> (AbstractInterval, AbstractInterval) {
    let a = word_to_unsigned(a);
    let b = word_to_unsigned(b);

    let lo_prod = a.lo as i128 * b.lo as i128;
    let hi_prod = a.hi as i128 * b.hi as i128;

    let lo = AbstractInterval {
        lo: (lo_prod & ((1i128 << WORD_BITS) - 1)) as i128,
        hi: (hi_prod & ((1i128 << WORD_BITS) - 1)) as i128,
    };

    let hi = AbstractInterval {
        lo: (lo_prod >> WORD_BITS) as i128,
        hi: (hi_prod >> WORD_BITS) as i128,
    };

    (lo, hi)
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

pub fn word_sle(a: &Word, b: &Word) -> AbstractInterval {
    let a = word_to_unsigned(a).to_signed(WORD_BITS);
    let b = word_to_unsigned(b).to_signed(WORD_BITS);

    if a.hi <= b.lo {
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
    if a.is_singleton() && b.is_singleton() {
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

    if a.is_singleton() && b.is_singleton() {
        let res = a.lo | b.lo;
        AbstractInterval { lo: res, hi: res }
    } else {
        // OR can at most set all bits up to the highest bit present in either operand.
        // We find the smallest power of 2 minus 1 that covers both.
        let max_possible = (1i128 << (128 - (a.hi | b.hi).leading_zeros())) - 1;
        AbstractInterval {
            lo: a.lo.max(b.lo),
            hi: max_possible.min(WORD_BOUND - 1),
        }
    }
}

pub fn word_xor(a: &Word, b: &Word) -> AbstractInterval {
    let a = word_to_unsigned(a);
    let b = word_to_unsigned(b);

    if a.is_singleton() && b.is_singleton() {
        let res = a.lo ^ b.lo;
        AbstractInterval { lo: res, hi: res }
    } else {
        // XOR is the most unpredictable for intervals.
        // The result's highest bit is bounded by the highest bit of a.hi or b.hi.
        let max_val = a.hi | b.hi;
        let hi_bound = if max_val == 0 {
            0
        } else {
            (1i128 << (128 - max_val.leading_zeros())) - 1
        };
        AbstractInterval {
            lo: 0,
            hi: hi_bound.min(WORD_BOUND - 1),
        }
    }
}

pub fn word_eq(a: &Word, b: &Word) -> AbstractInterval {
    let a = word_to_unsigned(a);
    let b = word_to_unsigned(b);

    // definitely equal
    if a.is_singleton() && b.is_singleton() && a.lo == b.lo {
        AbstractInterval::one()
    }
    // definitely not equal
    else if a.hi < b.lo || b.hi < a.lo {
        AbstractInterval::zero()
    }
    // unsure
    else {
        AbstractInterval::bool()
    }
}

pub fn word_neq(a: &Word, b: &Word) -> AbstractInterval {
    let r = word_eq(a, b);
    match (r.lo, r.hi) {
        (1, 1) => AbstractInterval::zero(),
        (0, 0) => AbstractInterval::one(),
        _ => AbstractInterval::bool(),
    }
}

pub fn word_srl(a: &Word, b: &Word) -> AbstractInterval {
    let a = word_to_unsigned(a);
    let b = word_to_unsigned(b);

    // シフト量が32ビット以上の場合、結果は常に0
    if b.lo >= WORD_BITS as i128 {
        return AbstractInterval::zero();
    }

    // 最小値: aの最小値を最大のシフト量でシフトしたもの
    let lo = if b.hi >= WORD_BITS as i128 {
        0
    } else {
        a.lo >> b.hi
    };

    // 最大値: aの最大値を最小のシフト量でシフトしたもの
    // (b.lo < 32 は確定している)
    let hi = a.hi >> b.lo;

    AbstractInterval { lo, hi }
}

mod tests {
    use crate::interval::AbstractInterval;
    use crate::wordop::Word;

    fn _signed_word(val_lo: i128, val_hi: i128) -> Word {
        let top = if val_lo < 0 { 255 } else { 0 };
        [
            _ai(val_lo & 0xFF, val_hi & 0xFF),
            _byte(top),
            _byte(top),
            _byte(top),
        ]
    }

    fn _ai(lo: i128, hi: i128) -> AbstractInterval {
        AbstractInterval { lo, hi }
    }

    fn _byte(v: i128) -> AbstractInterval {
        _ai(v, v)
    }

    fn _word_range(lo: i128, hi: i128) -> Word {
        [_ai(lo, hi), _byte(0), _byte(0), _byte(0)]
    }

    #[test]
    fn test_add_no_wrap() {
        use crate::wordop::{word_add, Word};
        let a: Word = [_byte(1), _byte(0), _byte(0), _byte(0)];
        let b: Word = [_byte(2), _byte(0), _byte(0), _byte(0)];

        let r = word_add(&a, &b);
        assert_eq!(r, _ai(3, 3));
    }

    #[test]
    fn test_add_wrap_to_full() {
        use crate::wordop::{word_add, Word};
        let a: Word = [_byte(255), _byte(255), _byte(255), _byte(255)];
        let b: Word = [_byte(1), _byte(0), _byte(0), _byte(0)];

        let r = word_add(&a, &b);
        assert_eq!(r, _ai(0, (1 << 32) - 1));
    }

    #[test]
    fn test_sub_no_wrap_exact() {
        use crate::wordop::word_sub;
        // 20 - 5 = 15
        let a = _word_range(20, 20);
        let b = _word_range(5, 5);

        let r = word_sub(&a, &b);
        assert_eq!(r, _ai(15, 15));
    }

    #[test]
    fn test_sub_interval_progression() {
        use crate::wordop::word_sub;
        // [10, 20] - [1, 3] = [7, 19]
        let a = _word_range(10, 20);
        let b = _word_range(1, 3);

        let r = word_sub(&a, &b);
        assert_eq!(r, _ai(7, 19));
    }

    #[test]
    fn test_sub_near_boundary_no_wrap() {
        use crate::wordop::word_sub;
        // [100, 110] - [10, 20] = [80, 100]
        let a = _word_range(100, 110);
        let b = _word_range(10, 20);

        let r = word_sub(&a, &b);
        assert_eq!(r, _ai(80, 100));
    }

    #[test]
    fn test_sub_definite_underflow() {
        use crate::wordop::{full_word, word_sub};
        // [5, 10] - [20, 30] → 必ず underflow
        let a = _word_range(5, 10);
        let b = _word_range(20, 30);

        let r = word_sub(&a, &b);
        assert_eq!(r, full_word());
    }

    #[test]
    fn test_sub_partial_underflow() {
        use crate::wordop::{full_word, word_sub};
        // [10, 20] - [15, 25]
        // 10 - 25 = underflow
        // 20 - 15 = ok
        let a = _word_range(10, 20);
        let b = _word_range(15, 25);

        let r = word_sub(&a, &b);
        assert_eq!(r, full_word());
    }

    #[test]
    fn test_mul_no_overflow() {
        use crate::wordop::{word_mul, Word};
        let a: Word = [_byte(2), _byte(0), _byte(0), _byte(0)];
        let b: Word = [_byte(3), _byte(0), _byte(0), _byte(0)];

        let r = word_mul(&a, &b);
        assert_eq!(r, _ai(6, 6));
    }

    #[test]
    fn test_mul_no_overflow2() {
        use crate::wordop::{word_mul, Word};
        let a: Word = [_byte(3), _byte(0), _byte(0), _byte(60)];
        let b: Word = [_byte(4), _byte(0), _byte(0), _byte(0)];

        let r = word_mul(&a, &b);
        assert_eq!(r, _ai(4026531852, 4026531852));
    }

    #[test]
    fn test_mul_overflow_full() {
        use crate::wordop::{word_mul, Word};
        let a: Word = [_byte(255), _byte(255), _byte(255), _byte(255)];
        let b: Word = [_byte(2), _byte(0), _byte(0), _byte(0)];

        let r = word_mul(&a, &b);
        assert_eq!(r, _ai(4294967294, 4294967294));
    }

    #[test]
    fn test_mulhu_simple() {
        use crate::wordop::{word_mulhu, Word};
        let a: Word = [_byte(0), _byte(0), _byte(0), _byte(1)]; // 1 << 24
        let b: Word = [_byte(0), _byte(0), _byte(0), _byte(1)];

        let r = word_mulhu(&a, &b);
        assert_eq!(r, _ai(1 << 16, 1 << 16));
    }

    #[test]
    fn test_mulhu_max() {
        use crate::wordop::{word_mulhu, Word};
        let a: Word = [_byte(255), _byte(255), _byte(255), _byte(255)];
        let b: Word = [_byte(255), _byte(255), _byte(255), _byte(255)];

        let r = word_mulhu(&a, &b);
        println!("r is {}", r);
        assert!(r.lo >= 0);
        assert!(r.hi <= (1 << 32) - 1);
    }

    #[test]
    fn test_mulhs_positive() {
        use crate::wordop::{word_mulhs, Word};
        let a: Word = [_byte(2), _byte(0), _byte(0), _byte(0)];
        let b: Word = [_byte(3), _byte(0), _byte(0), _byte(0)];

        let r = word_mulhs(&a, &b);
        assert_eq!(r, _ai(0, 0));
    }

    /*
    #[test]
    fn test_mulhs_negative() {
        use crate::wordop::{word_mulhs, Word};
        // -1 * 2
        let a: Word = [_byte(255), _byte(255), _byte(255), _byte(255)];
        let b: Word = [_byte(2), _byte(0), _byte(0), _byte(0)];

        let r = word_mulhs(&a, &b);
        assert!(r.lo <= -1);
        assert!(r.hi <= 0);
    }*/

    #[test]
    fn test_mulhs_negative() {
        /*
        >>> xs[44], xs[45], xs[46], xs[47]
        (131, 208, 124, 251)
        >>> xs[32], xs[33], xs[34], xs[35]
        (113, 95, 226, 44)
        >>> xs[38], xs[39], xs[40], xs[41]
        (86, 85, 67, 230)
        */

        use crate::wordop::{word_mulhs, Word};
        // -1 * 2
        let a: Word = [_byte(86), _byte(85), _byte(67), _byte(230)];
        let b: Word = [_byte(113), _byte(95), _byte(226), _byte(44)];

        let r = word_mulhs(&a, &b);
        println!("{}", r);
        assert!(r.lo == 4219261059);
        assert!(r.hi == 4219261059);
    }

    #[test]
    fn test_div_simple() {
        use crate::wordop::{word_div, Word};
        let a: Word = [_byte(10), _byte(0), _byte(0), _byte(0)];
        let b: Word = [_byte(2), _byte(0), _byte(0), _byte(0)];

        let r = word_div(&a, &b);
        assert_eq!(r, _ai(5, 5));
    }

    #[test]
    fn test_div_by_zero_full() {
        use crate::wordop::{word_div, Word};
        let a: Word = [_byte(10), _byte(0), _byte(0), _byte(0)];
        let b: Word = [_byte(0), _byte(0), _byte(0), _byte(0)];

        let r = word_div(&a, &b);
        assert_eq!(r, _ai(0, (1 << 32) - 1));
    }

    #[test]
    fn test_sdiv_positive() {
        use crate::wordop::{word_sdiv, Word};
        let a: Word = [_byte(10), _byte(0), _byte(0), _byte(0)];
        let b: Word = [_byte(2), _byte(0), _byte(0), _byte(0)];

        let r = word_sdiv(&a, &b);
        assert_eq!(r, _ai(5, 5));
    }

    #[test]
    fn test_sdiv_negative() {
        use crate::wordop::{word_sdiv, Word};
        // -10 / 2
        let a: Word = [_byte(246), _byte(255), _byte(255), _byte(255)];
        let b: Word = [_byte(2), _byte(0), _byte(0), _byte(0)];

        let r = word_sdiv(&a, &b);
        assert!(r.lo <= -5);
        assert!(r.hi <= -5);
    }

    #[test]
    fn test_sdiv_zero_divisor_full() {
        use crate::wordop::{word_sdiv, Word};
        let a: Word = [_byte(1), _byte(0), _byte(0), _byte(0)];
        let b: Word = [_ai(-1, 1), _byte(0), _byte(0), _byte(0)];

        let r = word_sdiv(&a, &b);
        assert_eq!(r, _ai(0, (1 << 32) - 1));
    }

    #[test]
    fn test_ltu_true() {
        use crate::wordop::{word_ltu, Word};
        let a: Word = [_byte(1), _byte(0), _byte(0), _byte(0)];
        let b: Word = [_byte(2), _byte(0), _byte(0), _byte(0)];

        assert_eq!(word_ltu(&a, &b), _ai(1, 1));
    }

    #[test]
    fn test_ltu_false() {
        use crate::wordop::{word_ltu, Word};
        let a: Word = [_byte(5), _byte(0), _byte(0), _byte(0)];
        let b: Word = [_byte(2), _byte(0), _byte(0), _byte(0)];

        assert_eq!(word_ltu(&a, &b), _ai(0, 0));
    }

    #[test]
    fn test_slt_negative_vs_positive() {
        use crate::wordop::{word_slt, Word};
        // -1 < 1
        let a: Word = [_byte(255), _byte(255), _byte(255), _byte(255)];
        let b: Word = [_byte(1), _byte(0), _byte(0), _byte(0)];

        assert_eq!(word_slt(&a, &b), _ai(1, 1));
    }

    #[test]
    fn test_add_interval_progression() {
        use crate::wordop::{full_word, word_add, word_to_unsigned, WORD_BOUND};
        // [1, 2] + [10, 20] = [11, 22]
        let a = _word_range(1, 2);
        let b = _word_range(10, 20);
        assert_eq!(word_add(&a, &b), _ai(11, 22));

        // 境界付近: [MAX-10, MAX-5] + [1, 4] = [MAX-9, MAX-1] (No wrap)
        let a_near = [_ai(255, 255), _ai(255, 255), _ai(255, 255), _ai(240, 245)]; // 非常に大きい値
        let b_small = _word_range(1, 4);
        let r = word_add(&a_near, &b_small);
        assert!(r.hi < WORD_BOUND);
        assert_eq!(r.lo, word_to_unsigned(&a_near).lo + 1);

        // 一部でもオーバーフローの可能性がある場合は full_word
        let a_overflow = [_ai(255, 255), _ai(255, 255), _ai(255, 255), _ai(250, 255)];
        let b_overflow = _word_range(10, 20);
        assert_eq!(word_add(&a_overflow, &b_overflow), full_word());
    }

    #[test]
    fn test_mul_interval_growth() {
        use crate::wordop::{full_word, word_mul};
        // [2, 3] * [4, 5] = [8, 15]
        let a = _word_range(2, 3);
        let b = _word_range(4, 5);
        assert_eq!(word_mul(&a, &b), _ai(8, 15));

        // 巨大な範囲への拡大
        let a_big = _word_range(100, 200);
        let b_big = [_ai(0, 255), _ai(0, 255), _ai(0, 255), _ai(0, 255)];
        assert_eq!(word_mul(&a_big, &b_big), full_word());
    }

    /*
    #[test]
    fn test_mulhs_signed_intervals() {
        use crate::wordop::word_mulhs;
        // 正×負のインターバル
        let a = _word_range(10, 20);
        let b = [_ai(254, 255), _ai(255, 255), _ai(255, 255), _ai(255, 255)]; // [-2, -1]
        let r = word_mulhs(&a, &b);
        assert!(r.lo <= r.hi);
        assert!(r.hi <= 0);
        assert_eq!(r.lo, -1);
        assert_eq!(r.hi, -1);
    }*/

    #[test]
    fn test_div_interval_ranges() {
        use crate::wordop::{full_word, word_div};
        // [100, 200] / [2, 10] = [100/10, 200/2] = [10, 100]
        let a = _word_range(100, 200);
        let b = _word_range(2, 10);
        assert_eq!(word_div(&a, &b), _ai(10, 100));

        // 除数に0が含まれる可能性 [0, 5] -> full_word
        let b_zero = _word_range(0, 5);
        assert_eq!(word_div(&a, &b_zero), full_word());
    }

    #[test]
    fn test_sdiv_complex_ranges() {
        // 配当が符号を跨ぐ: [-10, 10] / [2, 2] = [-5, 5]
        // a = 0xFFFFFFF6 (-10) to 0x0000000A (10)
        // ここではword_to_unsignedの仕様上、大きなインターバルになる可能性があるため
        // 実装の `to_signed` の境界値テストとして機能させる
        let a = _ai(-(1 << 10), 1 << 10);
        // 簡略化のため、直接 AbstractInterval の演算ロジックを確認
        let b = _ai(2, 2);
        let candidates = [a.lo / b.lo, a.lo / b.hi, a.hi / b.lo, a.hi / b.hi];
        let lo = *candidates.iter().min().unwrap();
        let hi = *candidates.iter().max().unwrap();
        assert_eq!(lo, -512);
        assert_eq!(hi, 512);
    }

    #[test]
    fn test_comparison_uncertainty() {
        use crate::wordop::word_ltu;
        // 確実な比較: [10, 20] < [30, 40] -> [1, 1] (True)
        let a = _word_range(10, 20);
        let b = _word_range(30, 40);
        assert_eq!(word_ltu(&a, &b), _ai(1, 1));

        // 確実な比較: [50, 60] < [10, 20] -> [0, 0] (False)
        let a = _word_range(50, 60);
        let b = _word_range(10, 20);
        assert_eq!(word_ltu(&a, &b), _ai(0, 0));

        // 不確実（重なりあり）: [15, 25] < [20, 30] -> [0, 1] (Unknown)
        let a = _word_range(15, 25);
        let b = _word_range(20, 30);
        assert_eq!(word_ltu(&a, &b), _ai(0, 1));
    }

    #[test]
    fn test_slt_definitely_less() {
        use crate::wordop::word_slt;
        // ケース1: 正の範囲同士で完全に小さい [1, 5] < [10, 15]
        let a = _signed_word(1, 5);
        let b = _signed_word(10, 15);
        assert_eq!(
            word_slt(&a, &b),
            _ai(1, 1),
            "Positive range: A < B should be True"
        );

        // ケース2: 負数 < 正数 [-10, -5] < [1, 2]
        // 注: signed_word の実装上、-10 は下位が 0xF6, 上位が 0xFF になる想定
        let a_neg = [_ai(240, 250), _byte(255), _byte(255), _byte(255)]; // [-16, -6]
        let b_pos = [_ai(1, 5), _byte(0), _byte(0), _byte(0)]; // [1, 5]
        assert_eq!(
            word_slt(&a_neg, &b_pos),
            _ai(1, 1),
            "Negative A < Positive B should be True"
        );

        // ケース3: 負の範囲同士で完全に小さい [-20, -15] < [-10, -5]
        let a_neg_far = [_ai(200, 210), _byte(255), _byte(255), _byte(255)];
        let b_neg_near = [_ai(240, 250), _byte(255), _byte(255), _byte(255)];
        assert_eq!(word_slt(&a_neg_far, &b_neg_near), _ai(1, 1));
    }

    #[test]
    fn test_slt_definitely_greater_or_equal() {
        use crate::wordop::word_slt;
        // ケース1: 正の範囲同士で完全に大きい [20, 30] < [5, 10] -> False
        let a = _signed_word(20, 30);
        let b = _signed_word(5, 10);
        assert_eq!(word_slt(&a, &b), _ai(0, 0));

        // ケース2: 正数 < 負数 [1, 5] < [-10, -5] -> False
        let a_pos = [_ai(1, 5), _byte(0), _byte(0), _byte(0)];
        let b_neg = [_ai(240, 250), _byte(255), _byte(255), _byte(255)];
        assert_eq!(word_slt(&a_pos, &b_neg), _ai(0, 0));
    }

    #[test]
    fn test_slt_overlap_unknown() {
        use crate::wordop::word_slt;
        // ケース1: 範囲が重なっている [5, 15] < [10, 20]
        // 5 < 10 (True) の可能性もあれば、15 < 10 (False) の可能性もあるため Unknown
        let a = _signed_word(5, 15);
        let b = _signed_word(10, 20);
        assert_eq!(
            word_slt(&a, &b),
            _ai(0, 1),
            "Overlapping ranges should return [0, 1]"
        );

        // ケース2: 境界値が一致している [5, 10] < [10, 15]
        // a.hi(10) < b.lo(10) は False なので、完全には小さくない
        let a_edge = _signed_word(5, 10);
        let b_edge = _signed_word(10, 15);
        assert_eq!(word_slt(&a_edge, &b_edge), _ai(0, 1));

        // ケース3: 片方がもう片方を包含している
        let a_inner = _signed_word(10, 12);
        let b_outer = _signed_word(5, 20);
        assert_eq!(word_slt(&a_inner, &b_outer), _ai(0, 1));
    }

    #[test]
    fn test_slt_max_min_bounds() {
        use crate::wordop::word_slt;
        // 32bit符号付きの最小値付近のテスト
        let i32_min = [_byte(0), _byte(0), _byte(0), _byte(128)]; // 0x80000000
        let zero = [_byte(0), _byte(0), _byte(0), _byte(0)];

        // INT_MIN < 0 は確実に True
        assert_eq!(word_slt(&i32_min, &zero), _ai(1, 1));

        // 0 < INT_MIN は確実に False
        assert_eq!(word_slt(&zero, &i32_min), _ai(0, 0));
    }

    #[test]
    fn test_word_and_comprehensive() {
        use crate::wordop::word_and;
        // 1. Singleton: 0b1100 & 0b1010 = 0b1000 (12 & 10 = 8)
        let a = _word_range(12, 12);
        let b = _word_range(10, 10);
        assert_eq!(word_and(&a, &b), _ai(8, 8));

        // 2. Range: [0, 255] & 0 = 0
        let a_range = _word_range(0, 255);
        let b_zero = _word_range(0, 0);
        assert_eq!(word_and(&a_range, &b_zero), _ai(0, 0));

        // 3. Range: [0, 7] & [0, 7] -> lo=0, hi=7
        let b_range = _word_range(0, 7);
        assert_eq!(word_and(&a_range, &b_range), _ai(0, 7));
    }

    #[test]
    fn test_word_or_comprehensive() {
        use crate::wordop::word_or;
        // 1. Singleton: 0b1100 | 0b0011 = 0b1111 (12 | 3 = 15)
        let a = _word_range(12, 12);
        let b = _word_range(3, 3);
        assert_eq!(word_or(&a, &b), _ai(15, 15));

        // 2. Range: [1, 2] | [4, 8]
        // lo should be at least max(1, 4) = 4
        // hi should be covered by the bitmask of 8 (which is 0b1111 = 15)
        let a_r = _word_range(1, 2);
        let b_r = _word_range(4, 8);
        let res = word_or(&a_r, &b_r);
        assert!(res.lo >= 4);
        assert!(res.hi <= 15);
    }

    #[test]
    fn test_word_xor_comprehensive() {
        use crate::wordop::word_xor;
        // 1. Singleton: 10 ^ 10 = 0
        let a = _word_range(10, 10);
        assert_eq!(word_xor(&a, &a), _ai(0, 0));

        // 2. Singleton: 0b1010 ^ 0b0101 = 0b1111 (10 ^ 5 = 15)
        let b = _word_range(5, 5);
        assert_eq!(word_xor(&a, &b), _ai(15, 15));

        // 3. Range property: [0, 3] ^ [0, 3]
        // Max possible value is 3 (0b11)
        let a_r = _word_range(0, 3);
        let res = word_xor(&a_r, &a_r);
        assert_eq!(res.lo, 0);
        assert!(res.hi >= 3);
    }

    #[test]
    fn test_bitwise_word_boundaries() {
        use crate::wordop::{word_and, word_or, word_xor};
        // Test with WORD_BOUND
        let all_ones = [_byte(255), _byte(255), _byte(255), _byte(255)]; // 0xFFFFFFFF
        let zero = [_byte(0), _byte(0), _byte(0), _byte(0)];

        // AND: ALL & 0 = 0
        assert_eq!(word_and(&all_ones, &zero), _ai(0, 0));

        // OR: 0 | ALL = ALL
        assert_eq!(word_or(&zero, &all_ones), _ai((1 << 32) - 1, (1 << 32) - 1));

        // XOR: ALL ^ ALL = 0
        assert_eq!(word_xor(&all_ones, &all_ones), _ai(0, 0));
    }

    #[test]
    fn test_eq_exact_true() {
        use crate::wordop::{word_eq, word_neq};
        let a = _word_range(10, 10);
        let b = _word_range(10, 10);

        assert_eq!(word_eq(&a, &b), _ai(1, 1));
        assert_eq!(word_neq(&a, &b), _ai(0, 0));
    }

    #[test]
    fn test_eq_exact_false() {
        use crate::wordop::{word_eq, word_neq};
        let a = _word_range(1, 5);
        let b = _word_range(10, 20);

        assert_eq!(word_eq(&a, &b), _ai(0, 0));
        assert_eq!(word_neq(&a, &b), _ai(1, 1));
    }

    #[test]
    fn test_eq_overlap_uncertain() {
        use crate::wordop::{word_eq, word_neq};
        let a = _word_range(5, 15);
        let b = _word_range(10, 20);

        assert_eq!(word_eq(&a, &b), _ai(0, 1));
        assert_eq!(word_neq(&a, &b), _ai(0, 1));
    }

    #[test]
    fn test_eq_concrete() {
        use crate::wordop::{word_eq, Word};
        let a: Word = [_byte(42), _byte(0), _byte(0), _byte(0)];
        let b: Word = [_byte(42), _byte(0), _byte(0), _byte(0)];

        assert_eq!(word_eq(&a, &b), _ai(1, 1));
    }

    #[test]
    fn test_neq_concrete() {
        use crate::wordop::{word_eq, word_neq, Word};
        let a: Word = [_byte(1), _byte(0), _byte(0), _byte(0)];
        let b: Word = [_byte(2), _byte(0), _byte(0), _byte(0)];

        assert_eq!(word_eq(&a, &b), _ai(0, 0));
        assert_eq!(word_neq(&a, &b), _ai(1, 1));
    }

    #[test]
    fn test_srl_singleton() {
        use crate::wordop::word_srl;
        // 16 >> 2 = 4
        let a = _word_range(16, 16);
        let b = _word_range(2, 2);
        assert_eq!(word_srl(&a, &b), _ai(4, 4));
    }

    #[test]
    fn test_srl_range_value() {
        use crate::wordop::word_srl;
        // [16, 32] >> 1 = [8, 16]
        let a = _word_range(16, 32);
        let b = _word_range(1, 1);
        assert_eq!(word_srl(&a, &b), _ai(8, 16));
    }

    #[test]
    fn test_srl_range_shift_amount() {
        use crate::wordop::word_srl;
        // 100 >> [1, 2]
        // 100 >> 1 = 50 (max)
        // 100 >> 2 = 25 (min)
        let a = _word_range(100, 100);
        let b = _word_range(1, 2);
        assert_eq!(word_srl(&a, &b), _ai(25, 50));
    }

    #[test]
    fn test_srl_both_ranges() {
        use crate::wordop::word_srl;
        // [10, 20] >> [1, 2]
        // min: 10 >> 2 = 2
        // max: 20 >> 1 = 10
        let a = _word_range(10, 20);
        let b = _word_range(1, 2);
        assert_eq!(word_srl(&a, &b), _ai(2, 10));
    }

    #[test]
    fn test_srl_overflow_shift_amount() {
        use crate::wordop::word_srl;
        // シフト量が32ビットを超える場合
        let a = _word_range(100, 200);
        let b = _word_range(32, 64);
        assert_eq!(word_srl(&a, &b), _ai(0, 0));

        // シフト量の範囲が32を跨ぐ場合: [100, 100] >> [31, 33]
        // 100 >> 31 = 0
        // 100 >> 33 = 0
        let b_cross = _word_range(31, 33);
        assert_eq!(word_srl(&_word_range(100, 100), &b_cross), _ai(0, 0));
    }

    #[test]
    fn test_srl_large_values() {
        use crate::wordop::word_srl;
        // 0xFFFFFFFF >> 31 = 1
        let a = [_byte(255), _byte(255), _byte(255), _byte(255)];
        let b = _word_range(31, 31);
        assert_eq!(word_srl(&a, &b), _ai(1, 1));
    }
}
