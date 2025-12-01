use std::collections::HashSet;
use std::fmt;
use std::hash::Hash;
use std::ops::{Add, Mul, Neg, Sub};

use serde::Serialize;

use rand::rngs::StdRng;
use rand::seq::{IndexedRandom, SliceRandom};
use rand::Rng;

use crate::interval::{AbstractInterval, MayBeFlag};

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize)]
pub enum LatticeVMSymbolicEntry {
    Main { is_curr: bool },
    Permutation { is_curr: bool },
    Preprocessed { is_curr: bool },
    Public,
}

impl fmt::Display for LatticeVMSymbolicEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LatticeVMSymbolicEntry::Main { is_curr } => {
                write!(f, "{}", if *is_curr { "curr" } else { "next" })
            }
            LatticeVMSymbolicEntry::Permutation { is_curr } => {
                write!(f, "{}", if *is_curr { "curr" } else { "next" })
            }
            LatticeVMSymbolicEntry::Preprocessed { is_curr } => {
                write!(f, "{}", if *is_curr { "curr" } else { "next" })
            }
            LatticeVMSymbolicEntry::Public => write!(f, "{}", "public"),
        }
    }
}

/// Represents a single symbolic variable, like a column in the trace.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize)]
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
#[derive(Clone, Debug, Serialize)]
pub enum LatticeVMSymbolicExpr {
    IsFirstRow,
    IsTransition,
    IsLastRow,
    Constant(AbstractInterval),
    Variable(LatticeVMSymbolicVal),
    Add(Box<Self>, Box<Self>),
    Sub(Box<Self>, Box<Self>),
    Mul(Box<Self>, Box<Self>),
    Neg(Box<Self>),
}

pub fn gather_vars(
    row_index: usize,
    expr: &LatticeVMSymbolicExpr,
    memo: &mut HashSet<(usize, usize)>,
) {
    match expr {
        LatticeVMSymbolicExpr::Variable(lattice_vmsymbolic_val) => {
            memo.insert((row_index, lattice_vmsymbolic_val.index));
        }
        LatticeVMSymbolicExpr::Add(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars(row_index, &lattice_vmsymbolic_expr, memo);
            gather_vars(row_index, &lattice_vmsymbolic_expr1, memo);
        }
        LatticeVMSymbolicExpr::Sub(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars(row_index, &lattice_vmsymbolic_expr, memo);
            gather_vars(row_index, &lattice_vmsymbolic_expr1, memo);
        }
        LatticeVMSymbolicExpr::Mul(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars(row_index, &lattice_vmsymbolic_expr, memo);
            gather_vars(row_index, &lattice_vmsymbolic_expr1, memo);
        }
        LatticeVMSymbolicExpr::Neg(lattice_vmsymbolic_expr) => {
            gather_vars(row_index, &lattice_vmsymbolic_expr, memo);
        }
        _ => {}
    }
}

pub fn get_curr_i(expr: &LatticeVMSymbolicExpr) -> Option<usize> {
    if let LatticeVMSymbolicExpr::Variable(LatticeVMSymbolicVal { entry, index }) = expr {
        if let LatticeVMSymbolicEntry::Main { is_curr } = entry {
            if *is_curr {
                return Some(index.clone());
            }
        }
    }
    None
}

pub fn get_curr_i_sub_cur_j(expr: &LatticeVMSymbolicExpr) -> Option<(usize, usize)> {
    if let LatticeVMSymbolicExpr::Sub(lhs, rhs) = expr {
        if let Some(lhs_i) = get_curr_i(lhs) {
            if let Some(rhs_i) = get_curr_i(rhs) {
                return Some((lhs_i, rhs_i));
            }
        }
    }

    None
}

pub fn preprocess_row(
    air_constraints: &Vec<LatticeVMSymbolicExpr>,
    row: &mut Vec<AbstractInterval>,
    p: u32,
) {
    for c in air_constraints {
        if let LatticeVMSymbolicExpr::Mul(lhs, rhs) = c {
            if let Some(lhs_i) = get_curr_i(lhs) {
                if let Some(rhs_i) = get_curr_i(rhs) {
                    if row[lhs_i].is_non_zero(p) == MayBeFlag::True
                        && row[rhs_i].is_zero(p) != MayBeFlag::False
                    {
                        row[rhs_i] = AbstractInterval::zero();
                    }
                } else if let Some((r_lhs_i, r_rhs_i)) = get_curr_i_sub_cur_j(rhs) {
                    if row[lhs_i].is_non_zero(p) == MayBeFlag::True {
                        if row[r_lhs_i].is_singleton() && !row[r_rhs_i].is_singleton() {
                            row[r_rhs_i] = row[r_lhs_i].clone();
                        }
                        if !row[r_lhs_i].is_singleton() && row[r_rhs_i].is_singleton() {
                            row[r_lhs_i] = row[r_rhs_i].clone();
                        }
                    }
                }
            }
        }
    }
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
        Self::Add(Box::new(self), Box::new(rhs.into()))
    }
}

impl<T: Into<Self>> Sub<T> for LatticeVMSymbolicExpr {
    type Output = Self;
    fn sub(self, rhs: T) -> Self {
        Self::Sub(Box::new(self), Box::new(rhs.into()))
    }
}

impl<T: Into<Self>> Mul<T> for LatticeVMSymbolicExpr {
    type Output = Self;
    fn mul(self, rhs: T) -> Self {
        Self::Mul(Box::new(self), Box::new(rhs.into()))
    }
}

impl Neg for LatticeVMSymbolicExpr {
    type Output = Self;
    fn neg(self) -> Self {
        Self::Neg(Box::new(self))
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
                            None => AbstractInterval::zero(), //panic!("next_row not provided for next-row variable"),
                        }
                    }
                }
                LatticeVMSymbolicEntry::Preprocessed { is_curr } => {
                    if is_curr {
                        curr_row[cell.index].clone()
                    } else {
                        match next_row {
                            Some(nr) => nr[cell.index].clone(),
                            None => AbstractInterval::zero(), //panic!("next_row not provided for next-row variable"),
                        }
                    }
                }
                LatticeVMSymbolicEntry::Permutation { is_curr } => {
                    if is_curr {
                        curr_row[cell.index].clone()
                    } else {
                        match next_row {
                            Some(nr) => nr[cell.index].clone(),
                            None => AbstractInterval::zero(), //panic!("next_row not provided for next-row variable"),
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

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct AbstractTrace {
    pub data: Vec<Vec<AbstractInterval>>,
    pub singleton_positions: Vec<(usize, usize)>,
}

impl fmt::Display for AbstractTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (_i, row) in self.data.iter().enumerate() {
            write!(f, "* ")?;
            for (_j, val) in row.iter().enumerate() {
                if val.is_singleton() {
                    write!(f, "{}, ", val)?;
                } else {
                    write!(f, "{}, ", val)?;
                }
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

impl AbstractTrace {
    pub fn new(raw_trace: Vec<Vec<AbstractInterval>>) -> Self {
        let mut singleton_positions = Vec::new();
        for i in 0..raw_trace.len() {
            for j in 0..raw_trace[0].len() {
                if raw_trace[i][j].is_singleton() {
                    singleton_positions.push((i, j));
                }
            }
        }
        Self {
            data: raw_trace,
            singleton_positions,
        }
    }

    pub fn diff_positions(&self, other: &Self) -> Vec<(usize, usize)> {
        let mut diffs = vec![];

        for (i, (row_self, row_other)) in self.data.iter().zip(&other.data).enumerate() {
            for (j, (cell_self, cell_other)) in row_self.iter().zip(row_other).enumerate() {
                if cell_self != cell_other {
                    diffs.push((i, j));
                }
            }
        }

        diffs
    }
}

pub fn eval_air_constraints(
    trace: &AbstractTrace,
    public_vals: Option<&[AbstractInterval]>,
    constraints: &[LatticeVMSymbolicExpr],
    prime: u32,
) -> (MayBeFlag, i32, HashSet<(usize, usize)>) {
    let num_steps = trace.data.len();
    let mut is_all_true = true;
    let mut potential = 0;
    let mut memo = HashSet::<(usize, usize)>::new();
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
                    return (MayBeFlag::False, 0, memo);
                }
                MayBeFlag::MayBe => {
                    gather_vars(i, tc, &mut memo);
                    is_all_true = false;
                    potential += 1;
                }
            }
        }
    }
    if is_all_true {
        (MayBeFlag::True, 0, memo)
    } else {
        (MayBeFlag::MayBe, potential, memo)
    }
}

#[derive(Clone)]
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
) -> (MayBeFlag, i32, HashSet<(usize, usize)>) {
    let mut is_all_true = true;

    let (air_flag, mut potential, memo) =
        eval_air_constraints(trace, public_vals, &constraints.air_constraints, prime);
    match air_flag {
        MayBeFlag::True => {}
        MayBeFlag::False => return (MayBeFlag::False, 0, memo),
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
                return (MayBeFlag::False, 0, memo);
            }
            MayBeFlag::MayBe => {
                is_all_true = false;
                potential += 1;
            }
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
                return (MayBeFlag::False, 0, memo);
            }
            MayBeFlag::MayBe => {
                is_all_true = false;
                potential += 1;
            }
        }
    }

    if is_all_true {
        (MayBeFlag::True, 0, memo)
    } else {
        (MayBeFlag::MayBe, potential, memo)
    }
}

pub fn refine_trace(
    trace: &AbstractTrace,
    refinment_target_indicies: &Vec<usize>,
    min_row_id: usize,
    max_row_id: usize,
    prime: u32,
    rng: &mut StdRng,
) -> Option<Vec<AbstractTrace>> {
    if trace.singleton_positions.len() == trace.data.len() * trace.data[0].len() {
        return None;
    }
    if refinment_target_indicies.is_empty() {
        return None;
    }
    let mut c_refinment_target_indicies = refinment_target_indicies.clone();
    let i = rng.random_range(min_row_id..(max_row_id + 1)) as usize;
    c_refinment_target_indicies.shuffle(rng);
    let mut j = 0;
    while j < c_refinment_target_indicies.len() - 1
        && trace.data[i][c_refinment_target_indicies[j]].is_singleton()
    {
        j += 1;
    }
    if !trace.data[i][c_refinment_target_indicies[j]].is_singleton() {
        let vs = trace.data[i][c_refinment_target_indicies[j]].split(prime);
        let mut results = vec![];
        for v in vs {
            let mut new_trace = trace.clone();
            if v.is_singleton() {
                new_trace
                    .singleton_positions
                    .push((i, c_refinment_target_indicies[j]));
            }
            new_trace.data[i][c_refinment_target_indicies[j]] = v.clone();
            results.push(new_trace);
        }
        Some(results)
    } else {
        Some(vec![trace.clone(), trace.clone()])
    }
    //}
}

mod tests {
    #[test]
    fn test_eval_complex_constraints() {
        //use std::Box::Rc;

        use crate::{
            interval::AbstractInterval,
            symbolic::{
                AbstractTrace, LatticeVMSymbolicEntry, LatticeVMSymbolicExpr, LatticeVMSymbolicVal,
            },
        };

        let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1; //2_u32.pow(31) - 1;

        let a = LatticeVMSymbolicExpr::Mul(
            Box::new(LatticeVMSymbolicExpr::Constant(AbstractInterval {
                lo: 65536,
                hi: 65536,
            })),
            Box::new(LatticeVMSymbolicExpr::Variable(LatticeVMSymbolicVal {
                entry: LatticeVMSymbolicEntry::Main { is_curr: true },
                index: 2,
            })),
        );
        let b = LatticeVMSymbolicExpr::Mul(
            Box::new(LatticeVMSymbolicExpr::Variable(LatticeVMSymbolicVal {
                entry: LatticeVMSymbolicEntry::Main { is_curr: true },
                index: 1,
            })),
            Box::new(LatticeVMSymbolicExpr::Add(
                Box::new(LatticeVMSymbolicExpr::Add(
                    Box::new(LatticeVMSymbolicExpr::Mul(
                        Box::new(LatticeVMSymbolicExpr::Constant(AbstractInterval {
                            lo: 65536,
                            hi: 65536,
                        })),
                        Box::new(LatticeVMSymbolicExpr::Variable(LatticeVMSymbolicVal {
                            entry: LatticeVMSymbolicEntry::Main { is_curr: true },
                            index: 2,
                        })),
                    )),
                    Box::new(LatticeVMSymbolicExpr::Variable(LatticeVMSymbolicVal {
                        entry: LatticeVMSymbolicEntry::Main { is_curr: true },
                        index: 3,
                    })),
                )),
                Box::new(LatticeVMSymbolicExpr::Constant(AbstractInterval {
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
