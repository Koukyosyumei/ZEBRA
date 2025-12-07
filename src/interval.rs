use std::{
    fmt,
    ops::{Add, BitAnd, BitOr, BitXor, Mul, Neg, Not, Sub},
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
        Self {
            lo: !self.hi,
            hi: !self.lo,
        }
    }
}

impl AbstractInterval {
    pub fn top(prime: u32) -> Self {
        Self {
            lo: -(prime as i64) / 2,
            hi: (prime as i64) / 2,
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
        if self.lo < 0 || rhs.lo < 0 {
            AbstractInterval::bool()
        } else {
            if self.hi < rhs.lo {
                AbstractInterval::one()
            } else if self.lo > rhs.hi {
                AbstractInterval::zero()
            } else {
                AbstractInterval::bool()
            }
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
