use std::collections::HashSet;
use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};
use std::rc::Rc;

use rand::rngs::StdRng;
use rand::Rng;

use crate::interval::{AbstractInterval, MayBeFlag};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum LatticeVMSymbolicEntry {
    Main { is_curr: bool },
    Public,
}

impl fmt::Display for LatticeVMSymbolicEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LatticeVMSymbolicEntry::Main { is_curr } => {
                write!(f, "{}", if *is_curr { "curr" } else { "next" })
            }
            LatticeVMSymbolicEntry::Public => write!(f, "{}", "public"),
        }
    }
}

/// Represents a single symbolic variable, like a column in the trace.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct LatticeVMSymbolicVal {
    pub entry: LatticeVMSymbolicEntry,
    pub index: usize,
}

impl fmt::Display for LatticeVMSymbolicVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}[{}]", self.entry, self.index)
    }
}

/// An enum representing a symbolic expression tree.
#[derive(Clone, Debug)]
pub enum LatticeVMSymbolicExpr {
    IsFirstRow,
    IsTransition,
    IsLastRow,
    Constant(AbstractInterval),
    Variable(LatticeVMSymbolicVal),
    Add(Rc<Self>, Rc<Self>),
    Sub(Rc<Self>, Rc<Self>),
    Mul(Rc<Self>, Rc<Self>),
    Neg(Rc<Self>),
}

impl fmt::Display for LatticeVMSymbolicExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IsFirstRow => write!(f, "IsFirstRow"),
            Self::IsTransition => write!(f, "IsTransition"),
            Self::IsLastRow => write!(f, "IsLastRow"),
            Self::Constant(c) => write!(f, "{}", c),
            Self::Variable(v) => write!(f, "{}", v),
            Self::Add(x, y) => write!(f, "({} + {})", x, y),
            Self::Sub(x, y) => write!(f, "({} - {})", x, y),
            Self::Mul(x, y) => write!(f, "({} * {})", x, y),
            Self::Neg(x) => write!(f, "-{}", x),
        }
    }
}

impl<T: Into<Self>> Add<T> for LatticeVMSymbolicExpr {
    type Output = Self;
    fn add(self, rhs: T) -> Self {
        Self::Add(Rc::new(self), Rc::new(rhs.into()))
    }
}

impl<T: Into<Self>> Sub<T> for LatticeVMSymbolicExpr {
    type Output = Self;
    fn sub(self, rhs: T) -> Self {
        Self::Sub(Rc::new(self), Rc::new(rhs.into()))
    }
}

impl<T: Into<Self>> Mul<T> for LatticeVMSymbolicExpr {
    type Output = Self;
    fn mul(self, rhs: T) -> Self {
        Self::Mul(Rc::new(self), Rc::new(rhs.into()))
    }
}

impl Neg for LatticeVMSymbolicExpr {
    type Output = Self;
    fn neg(self) -> Self {
        Self::Neg(Rc::new(self))
    }
}

impl From<LatticeVMSymbolicVal> for LatticeVMSymbolicExpr {
    fn from(var: LatticeVMSymbolicVal) -> Self {
        Self::Variable(var)
    }
}

impl LatticeVMSymbolicExpr {
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
                LatticeVMSymbolicEntry::Main { is_curr } => {
                    if is_curr {
                        curr_row[cell.index].clone()
                    } else {
                        match next_row {
                            Some(nr) => nr[cell.index].clone(),
                            None => panic!("next_row not provided for next-row variable"),
                        }
                    }
                }
                LatticeVMSymbolicEntry::Public => match public_vals {
                    Some(pv) => pv[cell.index].clone(),
                    None => panic!("public_vals not provided"),
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
                    a.eval(
                        curr_row,
                        next_row,
                        public_vals,
                        is_first_row,
                        is_transition,
                        is_last_row,
                        prime,
                    ) * b.eval(
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
    pub data: Vec<Vec<AbstractInterval>>,
    pub singleton_positions: HashSet<(usize, usize)>,
}

impl fmt::Display for AbstractTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, row) in self.data.iter().enumerate() {
            for (j, val) in row.iter().enumerate() {
                if val.is_singleton() {
                    write!(f, "*{}* ", val)?;
                } else {
                    write!(f, "{} ", val)?;
                }
            }
            writeln!(f)?;
        }
        Ok(())
    }
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
    public_vals: Option<&[AbstractInterval]>,
    constraints: &[LatticeVMSymbolicExpr],
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
                    public_vals,
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
        let i = 0; //rng.random_range(0..trace.data.len()) as usize;
        let j = 5; //rng.random_range(0..trace.data[i].len()) as usize;
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

mod tests {
    use std::rc::Rc;

    use crate::{
        interval::AbstractInterval,
        symbolic::{
            AbstractTrace, LatticeVMSymbolicEntry, LatticeVMSymbolicExpr, LatticeVMSymbolicVal,
        },
    };

    #[test]
    fn test_eval_complex_constraints() {
        let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1; //2_u32.pow(31) - 1;

        let a = LatticeVMSymbolicExpr::Mul(
            Rc::new(LatticeVMSymbolicExpr::Constant(AbstractInterval {
                lo: 65536,
                hi: 65536,
            })),
            Rc::new(LatticeVMSymbolicExpr::Variable(LatticeVMSymbolicVal {
                entry: LatticeVMSymbolicEntry::Main { is_curr: true },
                index: 2,
            })),
        );
        let b = LatticeVMSymbolicExpr::Mul(
            Rc::new(LatticeVMSymbolicExpr::Variable(LatticeVMSymbolicVal {
                entry: LatticeVMSymbolicEntry::Main { is_curr: true },
                index: 1,
            })),
            Rc::new(LatticeVMSymbolicExpr::Add(
                Rc::new(LatticeVMSymbolicExpr::Add(
                    Rc::new(LatticeVMSymbolicExpr::Mul(
                        Rc::new(LatticeVMSymbolicExpr::Constant(AbstractInterval {
                            lo: 65536,
                            hi: 65536,
                        })),
                        Rc::new(LatticeVMSymbolicExpr::Variable(LatticeVMSymbolicVal {
                            entry: LatticeVMSymbolicEntry::Main { is_curr: true },
                            index: 2,
                        })),
                    )),
                    Rc::new(LatticeVMSymbolicExpr::Variable(LatticeVMSymbolicVal {
                        entry: LatticeVMSymbolicEntry::Main { is_curr: true },
                        index: 3,
                    })),
                )),
                Rc::new(LatticeVMSymbolicExpr::Constant(AbstractInterval {
                    lo: 2,
                    hi: 2,
                })),
            )),
        );
        let trace_data = vec![vec![
            AbstractInterval::u8(),
            AbstractInterval::u8(),
            AbstractInterval::u8(),
            AbstractInterval::u8(),
        ]];
        let trace = AbstractTrace::new(trace_data);
        let a_eval = a.eval(&trace.data[0], None, None, false, false, false, prime);
        println!("{}", a);
        println!("{}", a_eval);

        let b_eval = b.eval(&trace.data[0], None, None, false, false, false, prime);
        println!("{}", b);
        println!("{}", b_eval);

        assert!(false);
    }

    #[test]
    fn test_eval_fibonacci_air() {
        use crate::p3_to_tv::convert_p3_expr;
        use crate::symbolic::{eval_air_constraints, AbstractInterval, AbstractTrace, MayBeFlag};
        use crate::test_data::FibonacciAir;

        use p3_mersenne_31::Mersenne31;
        use p3_uni_stark::{get_symbolic_constraints, SymbolicExpression};

        let prime = 2_u32.pow(31) - 1;

        let num_steps = 2; // Choose the number of Fibonacci steps
        let final_value = 21; // Choose the final Fibonacci value
        let air = FibonacciAir {
            num_steps,
            final_value,
        };
        let symbolic_constraints: Vec<SymbolicExpression<Mersenne31>> =
            get_symbolic_constraints(&air, 0, 0);

        let mut tv_constraints = vec![];
        for sc in symbolic_constraints {
            tv_constraints.push(convert_p3_expr::<Mersenne31>(&sc));
        }

        let true_trace_data = vec![
            vec![AbstractInterval::zero(), AbstractInterval::one()],
            vec![AbstractInterval::one(), AbstractInterval::one()],
        ];
        let true_trace = AbstractTrace::new(true_trace_data);
        assert_eq!(
            eval_air_constraints(&true_trace, None, &tv_constraints, prime),
            MayBeFlag::True
        );

        let false_trace_data = vec![
            vec![
                AbstractInterval { lo: 0, hi: 7 },
                AbstractInterval { lo: 0, hi: 7 },
            ],
            vec![
                AbstractInterval { lo: 1, hi: 1 },
                AbstractInterval { lo: 0, hi: 15 },
            ],
        ];
        let false_trace = AbstractTrace::new(false_trace_data);
        assert_eq!(
            eval_air_constraints(&false_trace, None, &tv_constraints, prime),
            MayBeFlag::MayBe
        );
    }
}
