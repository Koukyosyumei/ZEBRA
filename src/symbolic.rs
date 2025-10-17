use std::collections::HashSet;
use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};
use std::rc::Rc;

use p3_field::PrimeField32;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::interval::{AbstractInterval, MayBeFlag};

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
pub enum TwinVMSymbolicExpr {
    IsFirstRow,
    IsTransition,
    IsLastRow,
    Constant(AbstractInterval),
    Variable(TwinVMSymbolicVal),
    Add(Rc<Self>, Rc<Self>),
    Sub(Rc<Self>, Rc<Self>),
    Mul(Rc<Self>, Rc<Self>),
    Neg(Rc<Self>),
}

impl fmt::Debug for TwinVMSymbolicExpr {
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

impl<T: Into<Self>> Add<T> for TwinVMSymbolicExpr {
    type Output = Self;
    fn add(self, rhs: T) -> Self {
        Self::Add(Rc::new(self), Rc::new(rhs.into()))
    }
}

impl<T: Into<Self>> Sub<T> for TwinVMSymbolicExpr {
    type Output = Self;
    fn sub(self, rhs: T) -> Self {
        Self::Sub(Rc::new(self), Rc::new(rhs.into()))
    }
}

impl<T: Into<Self>> Mul<T> for TwinVMSymbolicExpr {
    type Output = Self;
    fn mul(self, rhs: T) -> Self {
        Self::Mul(Rc::new(self), Rc::new(rhs.into()))
    }
}

impl Neg for TwinVMSymbolicExpr {
    type Output = Self;
    fn neg(self) -> Self {
        Self::Neg(Rc::new(self))
    }
}

impl From<TwinVMSymbolicVal> for TwinVMSymbolicExpr {
    fn from(var: TwinVMSymbolicVal) -> Self {
        Self::Variable(var)
    }
}

impl TwinVMSymbolicExpr {
    pub fn eval(
        &self,
        curr_row: &[AbstractInterval],
        next_row: Option<&[AbstractInterval]>,
        public_vals: Option<&[AbstractInterval]>,
        is_first_row: bool,
        is_transition: bool,
        is_last_row: bool,
        prime: u32,
    ) -> AbstractInterval {
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
                    prime,
                ) + b.eval(
                    curr_row,
                    next_row,
                    public_vals,
                    is_first_row,
                    is_transition,
                    is_last_row,
                    prime,
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
                    prime,
                ) - b.eval(
                    curr_row,
                    next_row,
                    public_vals,
                    is_first_row,
                    is_transition,
                    is_last_row,
                    prime,
                )
            }
            Self::Mul(a, b) => {
                let va = a.eval(
                    curr_row,
                    next_row,
                    public_vals,
                    is_first_row,
                    is_transition,
                    is_last_row,
                    prime,
                );
                if va.is_zero(prime) == MayBeFlag::True {
                    AbstractInterval::zero()
                } else {
                    b.eval(
                        curr_row,
                        next_row,
                        public_vals,
                        is_first_row,
                        is_transition,
                        is_last_row,
                        prime,
                    )
                }
            }
            Self::Neg(a) => -a.eval(
                curr_row,
                next_row,
                public_vals,
                is_first_row,
                is_transition,
                is_last_row,
                prime,
            ),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AbstractTrace {
    data: Vec<Vec<AbstractInterval>>,
    singleton_positions: HashSet<(usize, usize)>,
}

impl AbstractTrace {
    pub fn new(raw_trace: Vec<Vec<AbstractInterval>>) -> Self {
        Self {
            data: raw_trace,
            singleton_positions: HashSet::new(),
        }
    }
}

pub fn eval_air_constraints(
    trace: &AbstractTrace,
    constraints: &Vec<TwinVMSymbolicExpr>,
    prime: u32,
) -> MayBeFlag {
    let num_steps = trace.data.len();
    let mut is_all_true = true;
    for i in 0..num_steps {
        for tc in constraints {
            let flag = tc
                .eval(
                    &trace.data[i],
                    if i + 1 < num_steps {
                        Some(&trace.data[i + 1])
                    } else {
                        None
                    },
                    None,
                    i == 0,
                    i < num_steps - 1,
                    i == num_steps - 1,
                    prime,
                )
                .is_zero(prime);
            match flag {
                MayBeFlag::True => {}
                MayBeFlag::False => return MayBeFlag::False,
                MayBeFlag::MayBe => is_all_true = false,
            }
        }
    }
    if is_all_true {
        MayBeFlag::True
    } else {
        MayBeFlag::MayBe
    }
}

pub fn refine_trace(
    trace: &AbstractTrace,
    num_refined_points: usize,
    rng: &mut StdRng,
) -> Option<(AbstractTrace, AbstractTrace)> {
    if trace.singleton_positions.len() == trace.data.len() * trace.data[0].len() {
        return None;
    }
    let mut trace_a = trace.clone();
    let mut trace_b = trace.clone();
    for _ in 0..num_refined_points {
        let i = rng.random_range(0..trace.data.len()) as usize;
        let j = rng.random_range(0..trace.data[i].len()) as usize;
        if !trace.data[i][j].is_singleton() {
            let v = trace.data[i][j].split();
            if v.0.is_singleton() {
                trace_a.singleton_positions.insert((i, j));
            }
            if v.1.is_singleton() {
                trace_b.singleton_positions.insert((i, j));
            }
            trace_a.data[i][j] = v.0;
            trace_b.data[i][j] = v.1;
        }
    }
    Some((trace_a, trace_b))
}
