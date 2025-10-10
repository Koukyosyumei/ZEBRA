use std::ops::{Add, Mul, Neg, Sub};
use std::rc::Rc;

use p3_field::PrimeCharacteristicRing;
use p3_uni_stark::{
    get_symbolic_constraints, prove, verify, Entry, StarkConfig, SymbolicAirBuilder,
    SymbolicExpression, SymbolicVariable,
};

pub enum MayBeFlag {
    True,
    False,
    MayBe,
}

#[derive(Clone, Hash, Debug)]
pub struct AbstractInterval<F: PrimeCharacteristicRing> {
    pub lo: F,
    pub hi: F,
}

impl<F: PrimeCharacteristicRing> Add<Self> for AbstractInterval<F> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            lo: self.lo + rhs.lo,
            hi: self.hi + rhs.hi,
        }
    }
}

impl<F: PrimeCharacteristicRing> Sub<Self> for AbstractInterval<F> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            lo: self.lo - rhs.lo,
            hi: self.hi - rhs.hi,
        }
    }
}

impl<F: PrimeCharacteristicRing> Mul<Self> for AbstractInterval<F> {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self {
            lo: self.lo * rhs.lo,
            hi: self.hi * rhs.hi,
        }
    }
}

impl<F: PrimeCharacteristicRing> Neg for AbstractInterval<F> {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            lo: -self.lo,
            hi: -self.hi,
        }
    }
}

impl<F: PrimeCharacteristicRing> AbstractInterval<F> {
    pub fn zero() -> Self {
        Self {
            lo: F::ZERO,
            hi: F::ZERO,
        }
    }

    pub fn one() -> Self {
        Self {
            lo: F::ONE,
            hi: F::ONE,
        }
    }

    pub fn from_f(v: &F) -> Self {
        Self {
            lo: v.clone(),
            hi: v.clone(),
        }
    }
}
