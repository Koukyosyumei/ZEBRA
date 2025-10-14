use std::ops::{Add, Mul, Neg, Sub};

use p3_field::PrimeField32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MayBeFlag {
    True,
    False,
    MayBe,
}

#[derive(Clone, Hash, Debug)]
pub struct AbstractInterval<F: PrimeField32> {
    pub lo: F,
    pub hi: F,
}

impl<F: PrimeField32> Add<Self> for AbstractInterval<F> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            lo: self.lo + rhs.lo,
            hi: self.hi + rhs.hi,
        }
    }
}

impl<F: PrimeField32> Sub<Self> for AbstractInterval<F> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            lo: self.lo - rhs.lo,
            hi: self.hi - rhs.hi,
        }
    }
}

impl<F: PrimeField32> Mul<Self> for AbstractInterval<F> {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self {
            lo: self.lo * rhs.lo,
            hi: self.hi * rhs.hi,
        }
    }
}

impl<F: PrimeField32> Neg for AbstractInterval<F> {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            lo: -self.lo,
            hi: -self.hi,
        }
    }
}

impl<F: PrimeField32> AbstractInterval<F> {
    pub fn top() -> Self {
        Self {
            lo: F::ZERO,
            hi: -F::ONE,
        }
    }

    pub fn zero() -> Self {
        Self {
            lo: F::ZERO,
            hi: F::ZERO,
        }
    }

    pub fn is_zero(&self) -> MayBeFlag {
        if self.lo.to_unique_u32() == 0 {
            if self.hi.to_unique_u32() == 0 {
                return MayBeFlag::True;
            } else {
                return MayBeFlag::MayBe;
            }
        }
        return MayBeFlag::False;
    }

    pub fn one() -> Self {
        Self {
            lo: F::ONE,
            hi: F::ONE,
        }
    }

    pub fn split(&self) -> (Self, Self) {
        (
            Self {
                lo: self.lo,
                hi: self.hi / F::from_i16(2),
            },
            Self {
                lo: self.hi / F::from_i16(2),
                hi: self.hi,
            },
        )
    }

    pub fn from_f(v: &F) -> Self {
        Self {
            lo: v.clone(),
            hi: v.clone(),
        }
    }
}
