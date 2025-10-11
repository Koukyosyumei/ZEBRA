use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};
use std::rc::Rc;

use p3_field::PrimeField32;

use crate::interval::AbstractInterval;

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum TwinVMSymbolicEntry {
    Main { is_curr: bool },
    Public,
}

impl fmt::Debug for TwinVMSymbolicEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TwinVMSymbolicEntry::Main { is_curr } => {
                write!(f, "{}", if *is_curr { "curr" } else { "next" })
            }
            TwinVMSymbolicEntry::Public => write!(f, "{}", "public"),
        }
    }
}

/// Represents a single symbolic variable, like a column in the trace.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TwinVMSymbolicVal {
    pub entry: TwinVMSymbolicEntry,
    pub index: usize,
}

impl fmt::Debug for TwinVMSymbolicVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}[{}]", self.entry, self.index)
    }
}

/// An enum representing a symbolic expression tree.
pub enum TwinVMSymbolicExpr<F: PrimeField32> {
    IsFirstRow,
    IsTransition,
    IsLastRow,
    Constant(AbstractInterval<F>),
    Variable(TwinVMSymbolicVal),
    Add(Rc<Self>, Rc<Self>),
    Sub(Rc<Self>, Rc<Self>),
    Mul(Rc<Self>, Rc<Self>),
    Neg(Rc<Self>),
}

impl<F: fmt::Debug + PrimeField32> fmt::Debug for TwinVMSymbolicExpr<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IsFirstRow => write!(f, "IsFirstRow"),
            Self::IsTransition => write!(f, "IsTransition"),
            Self::IsLastRow => write!(f, "IsLastRow"),
            Self::Constant(c) => write!(f, "{:?}", c),
            Self::Variable(v) => write!(f, "{:?}", v),
            Self::Add(x, y) => write!(f, "({:?} + {:?})", x, y),
            Self::Sub(x, y) => write!(f, "({:?} - {:?})", x, y),
            Self::Mul(x, y) => write!(f, "({:?} * {:?})", x, y),
            Self::Neg(x) => write!(f, "-{:?}", x),
        }
    }
}

impl<F: PrimeField32, T: Into<Self>> Add<T> for TwinVMSymbolicExpr<F> {
    type Output = Self;
    fn add(self, rhs: T) -> Self {
        Self::Add(Rc::new(self), Rc::new(rhs.into()))
    }
}

impl<F: PrimeField32, T: Into<Self>> Sub<T> for TwinVMSymbolicExpr<F> {
    type Output = Self;
    fn sub(self, rhs: T) -> Self {
        Self::Sub(Rc::new(self), Rc::new(rhs.into()))
    }
}

impl<F: PrimeField32, T: Into<Self>> Mul<T> for TwinVMSymbolicExpr<F> {
    type Output = Self;
    fn mul(self, rhs: T) -> Self {
        Self::Mul(Rc::new(self), Rc::new(rhs.into()))
    }
}

impl<F: PrimeField32> Neg for TwinVMSymbolicExpr<F> {
    type Output = Self;
    fn neg(self) -> Self {
        Self::Neg(Rc::new(self))
    }
}

impl<F: PrimeField32> From<TwinVMSymbolicVal> for TwinVMSymbolicExpr<F> {
    fn from(var: TwinVMSymbolicVal) -> Self {
        Self::Variable(var)
    }
}

impl<F: PrimeField32> TwinVMSymbolicExpr<F> {
    pub fn eval(
        &self,
        curr_row: &[AbstractInterval<F>],
        next_row: Option<&[AbstractInterval<F>]>,
        public_vals: Option<&[AbstractInterval<F>]>,
        is_first_row: bool,
        is_transition: bool,
        is_last_row: bool,
    ) -> AbstractInterval<F> {
        match self {
            Self::IsFirstRow => {
                if is_first_row {
                    AbstractInterval::one()
                } else {
                    AbstractInterval::zero()
                }
            }
            Self::IsTransition => {
                if is_transition {
                    AbstractInterval::one()
                } else {
                    AbstractInterval::zero()
                }
            }
            Self::IsLastRow => {
                if is_last_row {
                    AbstractInterval::one()
                } else {
                    AbstractInterval::zero()
                }
            }
            Self::Constant(c) => c.clone(),
            Self::Variable(cell) => match cell.entry {
                TwinVMSymbolicEntry::Main { is_curr } => {
                    if is_curr {
                        curr_row[cell.index].clone()
                    } else {
                        match next_row {
                            Some(nr) => nr[cell.index].clone(),
                            None => panic!("next_row not provided for next-row variable"),
                        }
                    }
                }
                TwinVMSymbolicEntry::Public => match public_vals {
                    Some(pv) => pv[cell.index].clone(),
                    None => panic!("next_row not provided for next-row variable"),
                },
            },
            Self::Add(a, b) => {
                a.eval(
                    curr_row,
                    next_row,
                    public_vals,
                    is_first_row,
                    is_transition,
                    is_last_row,
                ) + b.eval(
                    curr_row,
                    next_row,
                    public_vals,
                    is_first_row,
                    is_transition,
                    is_last_row,
                )
            }
            Self::Sub(a, b) => {
                a.eval(
                    curr_row,
                    next_row,
                    public_vals,
                    is_first_row,
                    is_transition,
                    is_last_row,
                ) - b.eval(
                    curr_row,
                    next_row,
                    public_vals,
                    is_first_row,
                    is_transition,
                    is_last_row,
                )
            }
            Self::Mul(a, b) => {
                a.eval(
                    curr_row,
                    next_row,
                    public_vals,
                    is_first_row,
                    is_transition,
                    is_last_row,
                ) * b.eval(
                    curr_row,
                    next_row,
                    public_vals,
                    is_first_row,
                    is_transition,
                    is_last_row,
                )
            }
            Self::Neg(a) => -a.eval(
                curr_row,
                next_row,
                public_vals,
                is_first_row,
                is_transition,
                is_last_row,
            ),
        }
    }
}
