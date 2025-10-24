use std::collections::HashSet;
use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};
use std::rc::Rc;

use rand::rngs::StdRng;
use rand::seq::{IndexedRandom, IteratorRandom, SliceRandom};
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

/// Converts the given expression into SMT-LIB constraints over all rows.
pub fn expr_to_smt_over_trace(
    exprs: &[LatticeVMSymbolicExpr],
    n_rows: usize,
    n_cols: usize,
) -> String {
    fn helper(
        expr: &LatticeVMSymbolicExpr,
        row_id: usize,
        n_rows: usize,
        vars: &mut HashSet<String>,
    ) -> String {
        match expr {
            LatticeVMSymbolicExpr::IsFirstRow => {
                if row_id == 0 {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }
            LatticeVMSymbolicExpr::IsTransition => {
                if row_id < n_rows - 1 {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }
            LatticeVMSymbolicExpr::IsLastRow => {
                if row_id == n_rows - 1 {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }
            LatticeVMSymbolicExpr::Constant(AbstractInterval { lo, hi: _hi }) => format!("{}", lo),
            LatticeVMSymbolicExpr::Variable(v) => {
                // "curr" -> a_row_index, "next" -> a_row+1_index
                let base_row = match v.entry {
                    LatticeVMSymbolicEntry::Main { is_curr } => {
                        if is_curr {
                            row_id
                        } else {
                            row_id + 1
                        }
                    }
                    LatticeVMSymbolicEntry::Public => 0,
                };
                let name = format!("trace_{}_{}", base_row, v.index);
                vars.insert(name.clone());
                name
            }
            LatticeVMSymbolicExpr::Add(a, b) => {
                format!(
                    "(+ {} {})",
                    helper(a, row_id, n_rows, vars),
                    helper(b, row_id, n_rows, vars)
                )
            }
            LatticeVMSymbolicExpr::Sub(a, b) => {
                format!(
                    "(- {} {})",
                    helper(a, row_id, n_rows, vars),
                    helper(b, row_id, n_rows, vars)
                )
            }
            LatticeVMSymbolicExpr::Mul(a, b) => {
                format!(
                    "(* {} {})",
                    helper(a, row_id, n_rows, vars),
                    helper(b, row_id, n_rows, vars)
                )
            }
            LatticeVMSymbolicExpr::Neg(a) => format!("(- {})", helper(a, row_id, n_rows, vars)),
        }
    }

    let mut vars = HashSet::new();
    let mut smt = String::new();
    smt.push_str("(set-logic QF_NIA)\n");

    // Declare all trace variables
    for i in 0..(n_rows + 1) {
        for j in 0..n_cols {
            let name = format!("trace_{}_{}", i, j);
            vars.insert(name.clone());
        }
    }
    for v in &vars {
        smt.push_str(&format!("(declare-fun {} () Int)\n", v));
    }

    // For each row, expand the constraint with concrete row flags
    for expr in exprs {
        for i in 0..n_rows {
            let body = helper(expr, i, n_rows, &mut vars);
            smt.push_str(&format!("(assert (= {} 0))\n", body));
        }
    }

    smt.push_str("(check-sat)\n(get-model)\n");
    smt
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

pub fn gather_boolean_variables(constraints: &[LatticeVMSymbolicExpr]) -> Vec<usize> {
    let mut result = HashSet::new();
    for c in constraints {
        if let LatticeVMSymbolicExpr::Mul(lhs, _) = c {
            if let LatticeVMSymbolicExpr::Variable(LatticeVMSymbolicVal { entry: _, index }) =
                &**lhs
            {
                result.insert(index.clone());
            } else if let LatticeVMSymbolicExpr::Sub(lhs, rhs) = &**lhs {
                if let LatticeVMSymbolicExpr::Variable(LatticeVMSymbolicVal { entry: _, index }) =
                    &**lhs
                {
                    if let LatticeVMSymbolicExpr::Constant(AbstractInterval { lo: 1, hi: 1 }) =
                        &**rhs
                    {
                        result.insert(index.clone());
                    }
                }
                if let LatticeVMSymbolicExpr::Constant(AbstractInterval { lo: 1, hi: 1 }) = &**lhs {
                    if let LatticeVMSymbolicExpr::Variable(LatticeVMSymbolicVal {
                        entry: _,
                        index,
                    }) = &**rhs
                    {
                        result.insert(index.clone());
                    }
                }
            }
        }
    }
    result.iter().cloned().collect()
}

#[derive(Clone, Debug)]
pub struct AbstractTrace {
    pub data: Vec<Vec<AbstractInterval>>,
    pub singleton_positions: HashSet<(usize, usize)>,
}

impl fmt::Display for AbstractTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (_i, row) in self.data.iter().enumerate() {
            for (_j, val) in row.iter().enumerate() {
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
        let mut singleton_positions = HashSet::new();
        for i in 0..raw_trace.len() {
            for j in 0..raw_trace[0].len() {
                if raw_trace[i][j].is_singleton() {
                    singleton_positions.insert((i, j));
                }
            }
        }
        Self {
            data: raw_trace,
            singleton_positions,
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
                MayBeFlag::False => {
                    return MayBeFlag::False;
                }
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

pub struct LatticeVMConstraints {
    pub air_constraints: Vec<LatticeVMSymbolicExpr>,
    pub pv_pos_constraints: Vec<LatticeVMSymbolicExpr>,
    pub pv_neg_constraints: Vec<LatticeVMSymbolicExpr>,
}

pub fn eval_constraints(
    trace: &AbstractTrace,
    public_vals: Option<&[AbstractInterval]>,
    constraints: &LatticeVMConstraints,
    prime: u32,
) -> MayBeFlag {
    let mut is_all_true = true;

    let air_flag = eval_air_constraints(trace, public_vals, &constraints.air_constraints, prime);
    match air_flag {
        MayBeFlag::True => {}
        MayBeFlag::False => return MayBeFlag::False,
        MayBeFlag::MayBe => is_all_true = false,
    }

    for pp in &constraints.pv_pos_constraints {
        let flag = pp
            .eval(
                &trace.data[0],
                None,
                public_vals,
                false,
                false,
                false,
                prime,
            )
            .is_zero(prime);
        match flag {
            MayBeFlag::True => {}
            MayBeFlag::False => {
                return MayBeFlag::False;
            }
            MayBeFlag::MayBe => is_all_true = false,
        }
    }

    for pn in &constraints.pv_neg_constraints {
        let flag = pn
            .eval(
                &trace.data[0],
                None,
                public_vals,
                false,
                false,
                false,
                prime,
            )
            .is_non_zero(prime);
        match flag {
            MayBeFlag::True => {}
            MayBeFlag::False => {
                return MayBeFlag::False;
            }
            MayBeFlag::MayBe => is_all_true = false,
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
    refinment_target_indicies: &Vec<usize>,
    rng: &mut StdRng,
) -> Option<(AbstractTrace, AbstractTrace)> {
    if trace.singleton_positions.len() == trace.data.len() * trace.data[0].len() {
        return None;
    }
    if refinment_target_indicies.is_empty() {
        return None;
    }
    let mut c_refinment_target_indicies = refinment_target_indicies.clone();
    let mut trace_a = trace.clone();
    let mut trace_b = trace.clone();
    for _ in 0..num_refined_points {
        let i = rng.random_range(0..trace.data.len()) as usize;
        c_refinment_target_indicies.shuffle(rng);
        let mut j = 0;
        while trace.data[i][j].is_singleton() {
            j += 1;
        }
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
    #[test]
    fn test_eval_complex_constraints() {
        use std::rc::Rc;

        use crate::{
            interval::AbstractInterval,
            symbolic::{
                AbstractTrace, LatticeVMSymbolicEntry, LatticeVMSymbolicExpr, LatticeVMSymbolicVal,
            },
        };

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
}
