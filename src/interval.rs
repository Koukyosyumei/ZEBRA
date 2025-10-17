use std::ops::{Add, Mul, Neg, Sub};

use p3_field::PrimeField32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MayBeFlag {
    True,
    False,
    MayBe,
}

#[derive(Clone, Hash, Debug)]
pub struct AbstractInterval {
    pub lo: i64,
    pub hi: i64,
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
        Self {
            lo: self.lo * rhs.lo,
            hi: self.hi * rhs.hi,
        }
    }
}

impl Neg for AbstractInterval {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            lo: -self.lo,
            hi: -self.hi,
        }
    }
}

impl AbstractInterval {
    pub fn top() -> Self {
        Self {
            lo: 0_i64,
            hi: 0_i64,
        }
    }

    pub fn bool() -> Self {
        Self {
            lo: 0_i64,
            hi: 1_i64,
        }
    }

    pub fn u8() -> Self {
        Self {
            lo: 0_i64,
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

    pub fn from_f<F: PrimeField32>(v: &F) -> Self {
        Self {
            lo: v.to_unique_u32() as i64,
            hi: v.to_unique_u32() as i64,
        }
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
            }
        }
        if self.has_multiple_in_range(p as i64) {
            return MayBeFlag::MayBe;
        } else {
            return MayBeFlag::False;
        }
    }

    pub fn is_singleton(&self) -> bool {
        self.lo == self.hi
    }

    pub fn split(&self) -> (Self, Self) {
        (
            Self {
                lo: self.lo,
                hi: (self.lo + self.hi) / 2,
            },
            Self {
                lo: (self.lo + self.hi) / 2 + 1,
                hi: self.hi,
            },
        )
    }
}
