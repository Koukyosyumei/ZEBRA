use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};
use std::rc::Rc;

pub trait TwinVMAlgebra:
    Add<Output = Self> + Sub<Output = Self> + Mul<Output = Self> + Neg<Output = Self> + Clone + Eq
{
    fn is_zero(&self) -> bool;
    fn zero() -> Self;
    fn one() -> Self;
}

/// Represents a single symbolic variable, like a column in the trace.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TwinVMSymbolicCell {
    pub is_curr: bool,
    pub col_idx: usize,
}

impl fmt::Debug for TwinVMSymbolicCell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}[{}]",
            if self.is_curr { "curr" } else { "next" },
            self.col_idx
        )
    }
}

/// An enum representing a symbolic expression tree.
pub enum TwinVMSymbolicExpr<F: TwinVMAlgebra> {
    IsFirstRow,
    IsTransition,
    IsLastRow,
    Constant(F),
    Variable(TwinVMSymbolicCell),
    Add(Rc<Self>, Rc<Self>),
    Sub(Rc<Self>, Rc<Self>),
    Mul(Rc<Self>, Rc<Self>),
    Neg(Rc<Self>),
}

impl<F: fmt::Debug + TwinVMAlgebra> fmt::Debug for TwinVMSymbolicExpr<F> {
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

impl<F: TwinVMAlgebra, T: Into<Self>> Add<T> for TwinVMSymbolicExpr<F> {
    type Output = Self;
    fn add(self, rhs: T) -> Self {
        Self::Add(Rc::new(self), Rc::new(rhs.into()))
    }
}

impl<F: TwinVMAlgebra, T: Into<Self>> Sub<T> for TwinVMSymbolicExpr<F> {
    type Output = Self;
    fn sub(self, rhs: T) -> Self {
        Self::Sub(Rc::new(self), Rc::new(rhs.into()))
    }
}

impl<F: TwinVMAlgebra, T: Into<Self>> Mul<T> for TwinVMSymbolicExpr<F> {
    type Output = Self;
    fn mul(self, rhs: T) -> Self {
        Self::Mul(Rc::new(self), Rc::new(rhs.into()))
    }
}

impl<F: TwinVMAlgebra> Neg for TwinVMSymbolicExpr<F> {
    type Output = Self;
    fn neg(self) -> Self {
        Self::Neg(Rc::new(self))
    }
}

impl<F: TwinVMAlgebra> From<TwinVMSymbolicCell> for TwinVMSymbolicExpr<F> {
    fn from(var: TwinVMSymbolicCell) -> Self {
        Self::Variable(var)
    }
}

impl<F: TwinVMAlgebra> TwinVMSymbolicExpr<F> {
    pub fn eval(
        &self,
        curr_row: &[F],
        next_row: Option<&[F]>,
        is_first_row: bool,
        is_transition: bool,
        is_last_row: bool,
    ) -> F {
        match self {
            Self::IsFirstRow => {
                if is_first_row {
                    F::one()
                } else {
                    F::zero()
                }
            }
            Self::IsTransition => {
                if is_transition {
                    F::one()
                } else {
                    F::zero()
                }
            }
            Self::IsLastRow => {
                if is_last_row {
                    F::one()
                } else {
                    F::zero()
                }
            }
            Self::Constant(c) => c.clone(),
            Self::Variable(cell) => {
                if cell.is_curr {
                    curr_row[cell.col_idx].clone()
                } else {
                    match next_row {
                        Some(nr) => nr[cell.col_idx].clone(),
                        None => panic!("next_row not provided for next-row variable"),
                    }
                }
            }
            Self::Add(a, b) => {
                a.eval(curr_row, next_row, is_first_row, is_transition, is_last_row)
                    + b.eval(curr_row, next_row, is_first_row, is_transition, is_last_row)
            }
            Self::Sub(a, b) => {
                a.eval(curr_row, next_row, is_first_row, is_transition, is_last_row)
                    - b.eval(curr_row, next_row, is_first_row, is_transition, is_last_row)
            }
            Self::Mul(a, b) => {
                a.eval(curr_row, next_row, is_first_row, is_transition, is_last_row)
                    * b.eval(curr_row, next_row, is_first_row, is_transition, is_last_row)
            }
            Self::Neg(a) => -a.eval(curr_row, next_row, is_first_row, is_transition, is_last_row),
        }
    }

    pub fn eval_is_zero(
        &self,
        curr_row: &[F],
        next_row: Option<&[F]>,
        is_first_row: bool,
        is_transition: bool,
        is_last_row: bool,
    ) -> bool {
        self.eval(curr_row, next_row, is_first_row, is_transition, is_last_row)
            .is_zero()
    }
}
