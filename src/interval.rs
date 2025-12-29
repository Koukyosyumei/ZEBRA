use std::{
    fmt,
    ops::{Add, BitAnd, BitOr, BitXor, Mul, Neg, Not, Shl, Shr, Sub},
};

use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MayBeFlag {
    True,
    False,
    MayBe,
}

#[derive(Clone, Hash, Debug, PartialEq, Eq, Serialize)]
pub struct AbstractInterval {
    pub lo: i64,
    pub hi: i64,
}

impl fmt::Display for AbstractInterval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_singleton() {
            write!(f, "{}", self.lo)
        } else {
            write!(f, "[{}, {}]", self.lo, self.hi)
        }
    }
}

impl Add<Self> for AbstractInterval {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            lo: self.lo + rhs.lo,
            hi: self.hi + rhs.hi,
        }
    }
}

impl Sub<Self> for AbstractInterval {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            lo: self.lo - rhs.hi,
            hi: self.hi - rhs.lo,
        }
    }
}

impl Mul<Self> for AbstractInterval {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        let products = [
            self.lo * rhs.lo,
            self.lo * rhs.hi,
            self.hi * rhs.lo,
            self.hi * rhs.hi,
        ];
        Self {
            lo: *products.iter().min().unwrap(),
            hi: *products.iter().max().unwrap(),
        }
    }
}

impl Neg for AbstractInterval {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            lo: -self.hi,
            hi: -self.lo,
        }
    }
}

// Helper: Find the mask of bits that vary within the interval [lo, hi]
pub fn varying_bits(lo: i64, hi: i64) -> u64 {
    if lo == hi {
        return 0;
    }
    let diff = (lo as u64) ^ (hi as u64);
    if diff == 0 {
        return 0;
    }
    let msb = 63 - diff.leading_zeros();
    (1u64 << (msb + 1)) - 1
}

impl BitAnd<Self> for AbstractInterval {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        if self.lo < 0 || rhs.lo < 0 {
            panic!("BitAnd for negative region is not supported.");
        }

        let var1 = varying_bits(self.lo, self.hi);
        let var2 = varying_bits(rhs.lo, rhs.hi);

        let min_val = (self.lo as u64) & (rhs.lo as u64);
        let var_result = var1 | var2;

        let res_lo = min_val & (!var_result);
        let res_hi = min_val | var_result;

        let tight_hi = if self.lo >= 0 && rhs.lo >= 0 {
            std::cmp::min(res_hi, std::cmp::min(self.hi as u64, rhs.hi as u64))
        } else {
            res_hi
        };

        Self {
            lo: res_lo as i64,
            hi: tight_hi as i64,
        }
    }
}

impl BitOr<Self> for AbstractInterval {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        if self.lo < 0 || rhs.lo < 0 {
            panic!("BitOr for negative region is not supported.");
        }

        let var1 = varying_bits(self.lo, self.hi);
        let var2 = varying_bits(rhs.lo, rhs.hi);

        let max_val = (self.hi as u64) | (rhs.hi as u64);
        let var_result = var1 | var2;

        let res_lo = max_val & (!var_result);
        let res_hi = max_val | var_result;

        Self {
            lo: res_lo as i64,
            hi: res_hi as i64,
        }
    }
}

impl BitXor<Self> for AbstractInterval {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self {
        if self.lo < 0 || rhs.lo < 0 {
            panic!("BitXor for negative region is not supported.");
        }

        let var1 = varying_bits(self.lo, self.hi);
        let var2 = varying_bits(rhs.lo, rhs.hi);

        let base = (self.lo as u64) ^ (rhs.lo as u64);
        let var_result = var1 | var2;

        let res_lo = base & (!var_result);
        let res_hi = base | var_result;

        Self {
            lo: res_lo as i64,
            hi: res_hi as i64,
        }
    }
}

impl Not for AbstractInterval {
    type Output = Self;
    fn not(self) -> Self {
        if self.lo < 0 {
            panic!("Not for negative region is not supported.");
        }

        Self {
            lo: !self.hi,
            hi: !self.lo,
        }
    }
}

impl Shl<AbstractInterval> for AbstractInterval {
    type Output = Self;

    fn shl(self, rhs: AbstractInterval) -> Self {
        if self.lo < 0 || rhs.lo < 0 {
            if self.is_singleton() && rhs.is_singleton() {
                let val = self.lo << rhs.lo;
                return Self { lo: val, hi: val };
            } else {
                return Self::top(2130706433);
            }
        }

        let min_shift = rhs.lo as u32;
        let max_shift = rhs.hi as u32;

        let lo = self.lo << min_shift;
        let hi = self.hi << max_shift;

        Self { lo, hi }
    }
}

impl Shr<AbstractInterval> for AbstractInterval {
    type Output = Self;

    fn shr(self, rhs: AbstractInterval) -> Self {
        if self.lo < 0 || rhs.lo < 0 {
            if self.is_singleton() && rhs.is_singleton() {
                let val = self.lo >> rhs.lo;
                return Self { lo: val, hi: val };
            } else {
                return Self::top(2130706433);
            }
        }

        let min_shift = rhs.lo as u32;
        let max_shift = rhs.hi as u32;

        let lo = self.lo >> max_shift;
        let hi = self.hi >> min_shift;

        Self { lo, hi }
    }
}

fn div_floor_i64(x: i64, k: i64) -> i64 {
    debug_assert!(k > 0);
    if x >= 0 {
        x / k
    } else {
        -((-x + k - 1) / k)
    }
}

impl AbstractInterval {
    pub fn top(prime: u32) -> Self {
        Self {
            lo: 0,                //-(prime as i64) / 2,
            hi: prime as i64 - 1, //(prime as i64) / 2,
        }
    }

    pub fn bool() -> Self {
        Self {
            lo: 0_i64,
            hi: 1_i64,
        }
    }

    pub fn u4() -> Self {
        Self {
            lo: 0_i64,
            hi: 15_i64,
        }
    }

    pub fn u8() -> Self {
        Self {
            lo: 0_i64,
            hi: 255_i64,
        }
    }

    pub fn u16() -> Self {
        Self {
            lo: 0_i64,
            hi: 65535_i64,
        }
    }

    pub fn i4() -> Self {
        Self {
            lo: -8_i64,
            hi: 8_i64,
        }
    }

    pub fn i8() -> Self {
        Self {
            lo: -255_i64,
            hi: 255_i64,
        }
    }

    pub fn zero() -> Self {
        Self {
            lo: 0_i64,
            hi: 0_i64,
        }
    }

    pub fn one() -> Self {
        Self {
            lo: 1_i64,
            hi: 1_i64,
        }
    }

    pub fn from_i64(v: i64) -> Self {
        Self { lo: v, hi: v }
    }

    pub fn as_canonical_u32(&self, prime: u32) -> u32 {
        let prime_i64 = prime as i64;
        (((self.lo % prime_i64) + prime_i64) % prime_i64) as u32
    }

    pub fn has_multiple_in_range(&self, k: i64) -> bool {
        if self.lo <= 0 && 0 <= self.hi {
            true
        } else {
            (self.hi / k) - ((self.lo - 1) / k) >= 1
        }
    }

    pub fn is_zero(&self, p: u32) -> MayBeFlag {
        if self.is_singleton() {
            if self.lo % (p as i64) == 0 {
                return MayBeFlag::True;
            } else {
                return MayBeFlag::False;
            }
        }
        if self.has_multiple_in_range(p as i64) {
            return MayBeFlag::MayBe;
        } else {
            return MayBeFlag::False;
        }
    }

    pub fn is_non_zero(&self, p: u32) -> MayBeFlag {
        if self.is_singleton() {
            if self.lo % (p as i64) != 0 {
                return MayBeFlag::True;
            } else {
                return MayBeFlag::False;
            }
        }
        if self.has_multiple_in_range(p as i64) {
            return MayBeFlag::MayBe;
        } else {
            return MayBeFlag::True;
        }
    }

    pub fn ltu(&self, rhs: Self) -> AbstractInterval {
        if self.lo < 0 {
            //panic!("LTU for negative region is not supported.");
            return AbstractInterval::from_i64(123456);
        }

        // returns zero when self < rhs
        if self.hi < rhs.lo {
            AbstractInterval::zero()
        } else if self.lo >= rhs.hi {
            AbstractInterval::one()
        } else {
            AbstractInterval::bool()
        }
    }

    pub fn div_floor(&self, k: i64) -> Self {
        debug_assert!(k > 0);

        let lo = div_floor_i64(self.lo, k);
        let hi = div_floor_i64(self.hi, k);

        AbstractInterval { lo, hi }
    }

    pub fn intersect(&self, other: &Self) -> Option<Self> {
        let lo = std::cmp::max(self.lo, other.lo);
        let hi = std::cmp::min(self.hi, other.hi);

        if lo <= hi {
            Some(Self { lo, hi })
        } else {
            None
        }
    }

    pub fn is_singleton(&self) -> bool {
        self.lo == self.hi
    }

    pub fn split(&self, _p: u32) -> Vec<Self> {
        if self.hi == self.lo || self.hi == self.lo + 1 {
            vec![
                Self {
                    lo: self.lo,
                    hi: self.lo,
                },
                Self {
                    lo: self.hi,
                    hi: self.hi,
                },
            ]
        } else {
            vec![
                Self {
                    lo: self.lo,
                    hi: (self.lo + self.hi) / 2,
                },
                Self {
                    lo: (self.lo + self.hi) / 2 + 1,
                    hi: self.hi,
                },
            ]
        }
    }
}

pub fn msb_u8(b: u8) -> bool {
    (b & 0b1000_0000) != 0
}

pub fn msb_maybe(interval: &AbstractInterval) -> AbstractInterval {
    if interval.lo < 0 {
        return AbstractInterval::bool();
    }

    let lo_msb = msb_u8(interval.lo as u8);
    let hi_msb = msb_u8(interval.hi as u8);

    if lo_msb == hi_msb {
        if lo_msb {
            AbstractInterval::one()
        } else {
            AbstractInterval::zero()
        }
    } else {
        AbstractInterval::bool()
    }
}

#[cfg(test)]
mod tests {
    use std::panic;

    use super::*;

    struct SimpleRng {
        state: u64,
    }

    impl SimpleRng {
        fn new(seed: u64) -> Self {
            Self { state: seed }
        }

        fn next_u64(&mut self) -> u64 {
            let mut x = self.state;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.state = x;
            x
        }

        // min以上 max以下の値を返す
        fn range(&mut self, min: i64, max: i64) -> i64 {
            let width = (max - min + 1) as u64;
            (min as u64 + (self.next_u64() % width)) as i64
        }

        fn gen_positive_interval(&mut self, max_val: i64, max_width: i64) -> AbstractInterval {
            let lo = self.range(0, max_val);
            let width = self.range(0, max_width);
            AbstractInterval { lo, hi: lo + width }
        }
    }

    fn verify_binary_op<FAbs, FConc>(
        op_name: &str,
        a: AbstractInterval,
        b: AbstractInterval,
        op_abs: FAbs,
        op_conc: FConc,
    ) where
        FAbs: Fn(AbstractInterval, AbstractInterval) -> AbstractInterval,
        FConc: Fn(i64, i64) -> i64,
    {
        let res = op_abs(a.clone(), b.clone());

        // brute-force verification
        for x in a.lo..=a.hi {
            for y in b.lo..=b.hi {
                let concrete_res = op_conc(x, y);
                assert!(
                    concrete_res >= res.lo && concrete_res <= res.hi,
                    "Soundness check failed for {}: \n\
                     Input A: {:?}, Input B: {:?}\n\
                     Values: {} op {} = {}\n\
                     Result Interval: {:?} (Concrete value not in range!)",
                    op_name,
                    a,
                    b,
                    x,
                    y,
                    concrete_res,
                    res
                );
            }
        }
    }

    fn verify_unary_op<FAbs, FConc>(
        op_name: &str,
        a: AbstractInterval,
        op_abs: FAbs,
        op_conc: FConc,
    ) where
        FAbs: Fn(AbstractInterval) -> AbstractInterval,
        FConc: Fn(i64) -> i64,
    {
        let res = op_abs(a.clone());
        for x in a.lo..=a.hi {
            let concrete_res = op_conc(x);
            assert!(
                concrete_res >= res.lo && concrete_res <= res.hi,
                "Soundness check failed for {}: \n\
                 Input: {:?}, Value: {}\n\
                 Result: {} \n\
                 Result Interval: {:?}",
                op_name,
                a,
                x,
                concrete_res,
                res
            );
        }
    }

    #[test]
    fn test_bitwise_and_soundness() {
        let mut rng = SimpleRng::new(12345);
        for _ in 0..100 {
            let a = rng.gen_positive_interval(1000, 50); // 値は0~1000, 幅は最大50
            let b = rng.gen_positive_interval(1000, 50);

            verify_binary_op("BitAnd", a, b, |x, y| x & y, |x, y| x & y);
        }

        let zero = AbstractInterval { lo: 0, hi: 0 };
        let any = AbstractInterval { lo: 0, hi: 100 };
        verify_binary_op("BitAnd Zero", zero, any, |x, y| x & y, |x, y| x & y);
    }

    #[test]
    fn test_bitwise_or_soundness() {
        let mut rng = SimpleRng::new(67890);
        for _ in 0..100 {
            let a = rng.gen_positive_interval(2000, 60);
            let b = rng.gen_positive_interval(2000, 60);

            verify_binary_op("BitOr", a, b, |x, y| x | y, |x, y| x | y);
        }
    }

    #[test]
    fn test_bitwise_xor_soundness() {
        let mut rng = SimpleRng::new(112233);
        for _ in 0..100 {
            let a = rng.gen_positive_interval(500, 100);
            let b = rng.gen_positive_interval(500, 100);

            verify_binary_op("BitXor", a, b, |x, y| x ^ y, |x, y| x ^ y);
        }
    }

    #[test]
    fn test_not_soundness() {
        // 注: 正の数のNOTは負の数になりますが、結果が範囲に含まれているか検証します
        let mut rng = SimpleRng::new(445566);
        for _ in 0..100 {
            let a = rng.gen_positive_interval(1000, 100);
            verify_unary_op("Not", a, |x| !x, |x| !x);
        }
    }

    #[test]
    fn test_negative_input_panics() {
        // 負の数が入力されたときにパニックすることを確認
        let pos = AbstractInterval { lo: 0, hi: 10 };
        let neg = AbstractInterval { lo: -5, hi: -1 };
        let cross = AbstractInterval { lo: -2, hi: 2 }; // 負の領域を含む

        // BitAnd
        let result = panic::catch_unwind(|| pos.clone() & neg.clone());
        assert!(result.is_err(), "BitAnd should panic with negative operand");

        let result = panic::catch_unwind(|| cross.clone() & pos.clone());
        assert!(
            result.is_err(),
            "BitAnd should panic if interval crosses zero"
        );

        // BitOr
        let result = panic::catch_unwind(|| neg.clone() | pos.clone());
        assert!(result.is_err(), "BitOr should panic with negative operand");

        // BitXor
        let result = panic::catch_unwind(|| pos ^ cross);
        assert!(result.is_err(), "BitXor should panic with negative operand");

        // Not
        let result = panic::catch_unwind(|| !neg);
        assert!(result.is_err(), "Not should panic with negative operand");
    }

    #[test]
    fn test_specific_edge_cases() {
        // 手動で設定する特定のコーナーケース

        // ケース1: シングルトン同士 (2 & 3 = 2)
        let a = AbstractInterval::from_i64(2);
        let b = AbstractInterval::from_i64(3);
        assert_eq!((a.clone() & b.clone()).lo, 2);
        assert_eq!((a & b).hi, 2);

        // ケース2: 包含関係 ( [4,7] & [4,5] )
        // [4,7] -> 100, 101, 110, 111 (上位 1xx)
        // [4,5] -> 100, 101 (上位 10x)
        // AND結果は 100, 101 -> [4, 5] になるはず
        let c = AbstractInterval { lo: 4, hi: 7 };
        let d = AbstractInterval { lo: 4, hi: 5 };
        let res = c & d;
        assert!(res.lo <= 4);
        assert!(res.hi >= 5);

        // ケース3: 大きな飛び地
        // [0, 1] | [16, 17]
        // 00000, 00001 | 10000, 10001
        // OR結果は 10000(16) ~ 10001(17)
        let e = AbstractInterval { lo: 0, hi: 1 };
        let f = AbstractInterval { lo: 16, hi: 17 };
        let or_res = e | f;
        assert_eq!(or_res.lo, 16);
        assert_eq!(or_res.hi, 17);
    }
}
