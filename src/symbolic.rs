use std::collections::HashSet;
use std::fmt;
use std::hash::Hash;
use std::ops::{Add, Mul, Neg, Sub};

use serde::Serialize;

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::Rng;

use crate::alu::{
    reconstruct_symbolic_word, word_add, word_and, word_div, word_ltu, word_mul, word_mulhs,
    word_mulhu, word_mult, word_multu, word_neq, word_or, word_sdiv, word_slt, word_srl, word_sub,
    word_subu, word_xor,
};
use crate::interval::{msb_maybe, AbstractInterval, MayBeFlag};

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
    WhenNonZero(Box<Self>, Box<Self>),
    WhenZero(Box<Self>, Box<Self>),
    Constant(AbstractInterval),
    Variable(LatticeVMSymbolicVal),
    Add(Box<Self>, Box<Self>),
    Sub(Box<Self>, Box<Self>),
    Mul(Box<Self>, Box<Self>),
    MulLo(Box<Self>, Box<Self>),
    MulHiSS(Box<Self>, Box<Self>),
    MulHiUU(Box<Self>, Box<Self>),
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
    Xor(Box<Self>, Box<Self>),
    SRL(Box<Self>, Box<Self>),
    Lt(Box<Self>, Box<Self>),
    Msb(Box<Self>),
    Neg(Box<Self>),
    Flip(Box<Self>),
    KoalaBearRange(Box<Self>),
    WordAdd([Box<Self>; 4], [Box<Self>; 4]),
    WordSub([Box<Self>; 4], [Box<Self>; 4]),
    WordSubU([Box<Self>; 4], [Box<Self>; 4]),
    WordMul([Box<Self>; 4], [Box<Self>; 4]),
    WordMulhu([Box<Self>; 4], [Box<Self>; 4]),
    WordMulhs([Box<Self>; 4], [Box<Self>; 4]),
    WordMultl([Box<Self>; 4], [Box<Self>; 4]),
    WordMulth([Box<Self>; 4], [Box<Self>; 4]),
    WordMultul([Box<Self>; 4], [Box<Self>; 4]),
    WordMultuh([Box<Self>; 4], [Box<Self>; 4]),
    WordDiv([Box<Self>; 4], [Box<Self>; 4]),
    WordSDiv([Box<Self>; 4], [Box<Self>; 4]),
    WordLt([Box<Self>; 4], [Box<Self>; 4]),
    WordSLt([Box<Self>; 4], [Box<Self>; 4]),
    WordAnd([Box<Self>; 4], [Box<Self>; 4]),
    WordOr([Box<Self>; 4], [Box<Self>; 4]),
    WordXOr([Box<Self>; 4], [Box<Self>; 4]),
    WordEq([Box<Self>; 4], [Box<Self>; 4]),
    WordNEq([Box<Self>; 4], [Box<Self>; 4]),
    WordSrl([Box<Self>; 4], [Box<Self>; 4]),
}

impl Default for LatticeVMSymbolicExpr {
    fn default() -> Self {
        Self::Constant(AbstractInterval::zero())
    }
}

pub fn gather_vars_simple(expr: &LatticeVMSymbolicExpr, memo: &mut HashSet<usize>) {
    match expr {
        LatticeVMSymbolicExpr::Variable(lattice_vmsymbolic_val) => {
            memo.insert(lattice_vmsymbolic_val.index);
        }
        LatticeVMSymbolicExpr::Add(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars_simple(&lattice_vmsymbolic_expr, memo);
            gather_vars_simple(&lattice_vmsymbolic_expr1, memo);
        }
        LatticeVMSymbolicExpr::Sub(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars_simple(&lattice_vmsymbolic_expr, memo);
            gather_vars_simple(&lattice_vmsymbolic_expr1, memo);
        }
        LatticeVMSymbolicExpr::Mul(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars_simple(&lattice_vmsymbolic_expr, memo);
            gather_vars_simple(&lattice_vmsymbolic_expr1, memo);
        }
        LatticeVMSymbolicExpr::Neg(lattice_vmsymbolic_expr) => {
            gather_vars_simple(&lattice_vmsymbolic_expr, memo);
        }
        LatticeVMSymbolicExpr::And(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars_simple(&lattice_vmsymbolic_expr, memo);
            gather_vars_simple(&lattice_vmsymbolic_expr1, memo);
        }
        LatticeVMSymbolicExpr::Or(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars_simple(&lattice_vmsymbolic_expr, memo);
            gather_vars_simple(&lattice_vmsymbolic_expr1, memo);
        }
        LatticeVMSymbolicExpr::Xor(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars_simple(&lattice_vmsymbolic_expr, memo);
            gather_vars_simple(&lattice_vmsymbolic_expr1, memo);
        }
        LatticeVMSymbolicExpr::Lt(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars_simple(&lattice_vmsymbolic_expr, memo);
            gather_vars_simple(&lattice_vmsymbolic_expr1, memo);
        }
        LatticeVMSymbolicExpr::SRL(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars_simple(&lattice_vmsymbolic_expr, memo);
            gather_vars_simple(&lattice_vmsymbolic_expr1, memo);
        }
        LatticeVMSymbolicExpr::Msb(lattice_vmsymbolic_expr) => {
            gather_vars_simple(&lattice_vmsymbolic_expr, memo);
        }
        LatticeVMSymbolicExpr::Flip(lattice_vmsymbolic_expr) => {
            gather_vars_simple(&lattice_vmsymbolic_expr, memo);
        }
        _ => {}
    }
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
        LatticeVMSymbolicExpr::And(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars(row_index, &lattice_vmsymbolic_expr, memo);
            gather_vars(row_index, &lattice_vmsymbolic_expr1, memo);
        }
        LatticeVMSymbolicExpr::Or(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars(row_index, &lattice_vmsymbolic_expr, memo);
            gather_vars(row_index, &lattice_vmsymbolic_expr1, memo);
        }
        LatticeVMSymbolicExpr::Xor(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars(row_index, &lattice_vmsymbolic_expr, memo);
            gather_vars(row_index, &lattice_vmsymbolic_expr1, memo);
        }
        LatticeVMSymbolicExpr::Lt(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars(row_index, &lattice_vmsymbolic_expr, memo);
            gather_vars(row_index, &lattice_vmsymbolic_expr1, memo);
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

pub fn get_curr_i_sub_const(expr: &LatticeVMSymbolicExpr, _prime: u32) -> Option<(usize, i64)> {
    if let LatticeVMSymbolicExpr::Sub(lhs, rhs) = expr {
        // Case: Variable - Constant
        if let Some(v_idx) = get_curr_i(lhs) {
            if let LatticeVMSymbolicExpr::Constant(c) = &**rhs {
                if c.is_singleton() {
                    return Some((v_idx, c.lo));
                }
            }
        }
        // Case: Constant - Variable
        if let LatticeVMSymbolicExpr::Constant(c) = &**lhs {
            if let Some(v_idx) = get_curr_i(rhs) {
                if c.is_singleton() {
                    return Some((v_idx, c.lo));
                }
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
            Self::WhenNonZero(x, y) => write!(f, "([{} /= 0] => {})", x, y),
            Self::WhenZero(x, y) => write!(f, "([{} = 0] => {})", x, y),
            Self::Constant(c) => write!(f, "{}", c),
            Self::Variable(v) => write!(f, "{}", v),
            Self::Add(x, y) => write!(f, "({} + {})", x, y),
            Self::Sub(x, y) => write!(f, "({} - {})", x, y),
            Self::Mul(x, y) => write!(f, "({} * {})", x, y),
            Self::MulLo(x, y) => write!(f, "({} *_lo {})", x, y),
            Self::MulHiSS(x, y) => write!(f, "({} *_hiss {})", x, y),
            Self::MulHiUU(x, y) => write!(f, "({} *_hiuu {})", x, y),
            Self::And(x, y) => write!(f, "({} && {})", x, y),
            Self::Or(x, y) => write!(f, "({} || {})", x, y),
            Self::Xor(x, y) => write!(f, "({} ^ {})", x, y),
            Self::SRL(x, y) => write!(f, "({} >> {})", x, y),
            Self::Lt(x, y) => write!(f, "({} < {})", x, y),
            Self::Msb(x) => write!(f, "msb({})", x),
            Self::Neg(x) => write!(f, "-{}", x),
            Self::Flip(x) => write!(f, "~{}", x),
            Self::KoalaBearRange(x) => write!(f, "🐨{}🐨", x),
            Self::WordAdd(b, c) => write!(
                f,
                "[{}, {}, {}, {}] + [{}, {}, {}, {}]",
                b[0], b[1], b[2], b[3], c[0], c[1], c[2], c[3]
            ),
            Self::WordSub(b, c) => write!(
                f,
                "[{}, {}, {}, {}] - [{}, {}, {}, {}]",
                b[0], b[1], b[2], b[3], c[0], c[1], c[2], c[3]
            ),
            Self::WordSubU(b, c) => write!(
                f,
                "[{}, {}, {}, {}] -u [{}, {}, {}, {}]",
                b[0], b[1], b[2], b[3], c[0], c[1], c[2], c[3]
            ),
            Self::WordMul(b, c) => write!(
                f,
                "[{}, {}, {}, {}] * [{}, {}, {}, {}]",
                b[0], b[1], b[2], b[3], c[0], c[1], c[2], c[3]
            ),
            Self::WordMulhu(b, c) => write!(
                f,
                "[{}, {}, {}, {}] *_hu [{}, {}, {}, {}]",
                b[0], b[1], b[2], b[3], c[0], c[1], c[2], c[3]
            ),
            Self::WordMulhs(b, c) => write!(
                f,
                "[{}, {}, {}, {}] *_hs [{}, {}, {}, {}]",
                b[0], b[1], b[2], b[3], c[0], c[1], c[2], c[3]
            ),
            Self::WordMultl(b, c) => write!(
                f,
                "[{}, {}, {}, {}] *_tl [{}, {}, {}, {}]",
                b[0], b[1], b[2], b[3], c[0], c[1], c[2], c[3]
            ),
            Self::WordMulth(b, c) => write!(
                f,
                "[{}, {}, {}, {}] *_th [{}, {}, {}, {}]",
                b[0], b[1], b[2], b[3], c[0], c[1], c[2], c[3]
            ),
            Self::WordMultul(b, c) => write!(
                f,
                "[{}, {}, {}, {}] *_tul [{}, {}, {}, {}]",
                b[0], b[1], b[2], b[3], c[0], c[1], c[2], c[3]
            ),
            Self::WordMultuh(b, c) => write!(
                f,
                "[{}, {}, {}, {}] *_tuh [{}, {}, {}, {}]",
                b[0], b[1], b[2], b[3], c[0], c[1], c[2], c[3]
            ),
            Self::WordDiv(b, c) => write!(
                f,
                "[{}, {}, {}, {}] / [{}, {}, {}, {}]",
                b[0], b[1], b[2], b[3], c[0], c[1], c[2], c[3]
            ),
            Self::WordSDiv(b, c) => write!(
                f,
                "[{}, {}, {}, {}] /_s [{}, {}, {}, {}]",
                b[0], b[1], b[2], b[3], c[0], c[1], c[2], c[3]
            ),
            Self::WordLt(b, c) => write!(
                f,
                "[{}, {}, {}, {}] < [{}, {}, {}, {}]",
                b[0], b[1], b[2], b[3], c[0], c[1], c[2], c[3]
            ),
            Self::WordSLt(b, c) => write!(
                f,
                "[{}, {}, {}, {}] <_s [{}, {}, {}, {}]",
                b[0], b[1], b[2], b[3], c[0], c[1], c[2], c[3]
            ),
            Self::WordAnd(b, c) => write!(
                f,
                "[{}, {}, {}, {}] & [{}, {}, {}, {}]",
                b[0], b[1], b[2], b[3], c[0], c[1], c[2], c[3]
            ),
            Self::WordOr(b, c) => write!(
                f,
                "[{}, {}, {}, {}] | [{}, {}, {}, {}]",
                b[0], b[1], b[2], b[3], c[0], c[1], c[2], c[3]
            ),
            Self::WordXOr(b, c) => write!(
                f,
                "[{}, {}, {}, {}] ^ [{}, {}, {}, {}]",
                b[0], b[1], b[2], b[3], c[0], c[1], c[2], c[3]
            ),
            Self::WordEq(b, c) => write!(
                f,
                "[{}, {}, {}, {}] == [{}, {}, {}, {}]",
                b[0], b[1], b[2], b[3], c[0], c[1], c[2], c[3]
            ),
            Self::WordNEq(b, c) => write!(
                f,
                "[{}, {}, {}, {}] != [{}, {}, {}, {}]",
                b[0], b[1], b[2], b[3], c[0], c[1], c[2], c[3]
            ),
            Self::WordSrl(b, c) => write!(
                f,
                "[{}, {}, {}, {}] >> [{}, {}, {}, {}]",
                b[0], b[1], b[2], b[3], c[0], c[1], c[2], c[3]
            ),
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
        let rec = |a: &LatticeVMSymbolicExpr| {
            a.eval(
                curr_row,
                next_row,
                public_vals,
                is_first_row,
                is_transition,
                is_last_row,
                prime,
            )
        };

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
            Self::WhenNonZero(a, b) => {
                let cond = rec(a);
                if let MayBeFlag::True = cond.is_zero(prime) {
                    AbstractInterval::zero()
                } else {
                    rec(b)
                }
            }
            Self::WhenZero(a, b) => {
                let cond = rec(a);
                if let MayBeFlag::False = cond.is_zero(prime) {
                    AbstractInterval::zero()
                } else {
                    rec(b)
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
            Self::Add(a, b) => rec(a) + rec(b),
            Self::Sub(a, b) => rec(a) - rec(b),
            Self::Mul(a, b) => {
                let va = rec(a);
                if va.is_zero(prime) == MayBeFlag::True {
                    AbstractInterval::zero()
                } else {
                    rec(a) * rec(b)
                }
            }
            Self::MulLo(a, b) => {
                let full = rec(a) * rec(b);
                full.modulo(4294967296)
            }
            Self::MulHiSS(a, b) => {
                let sa = rec(a).to_signed(32);
                let sb = rec(b).to_signed(32);
                (sa * sb).div_floor(2 ^ 32)
            }
            Self::MulHiUU(a, b) => {
                let ua = rec(a);
                let ub = rec(b);
                (ua * ub).div_floor(2 ^ 32)
            }
            Self::And(a, b) => rec(a) & rec(b),
            Self::Or(a, b) => rec(a) | rec(b),
            Self::Xor(a, b) => rec(a) ^ rec(b),
            Self::SRL(a, b) => rec(a) >> rec(b),
            Self::Lt(a, b) => rec(a).ltu(rec(b)),
            Self::Neg(a) => -rec(a),
            Self::Flip(a) => {
                let is_zero = rec(a).is_zero(prime);
                if let MayBeFlag::True = is_zero {
                    AbstractInterval::one()
                } else {
                    let tmp = LatticeVMSymbolicExpr::Sub(
                        Box::new(LatticeVMSymbolicExpr::Constant(AbstractInterval::one())),
                        a.clone(),
                    );
                    let is_one = rec(&tmp).is_zero(prime);
                    if let MayBeFlag::True = is_one {
                        AbstractInterval::zero()
                    } else {
                        AbstractInterval::bool()
                    }
                }
            }
            Self::Msb(a) => msb_maybe(&rec(a)),
            Self::KoalaBearRange(a) => {
                let e = LatticeVMSymbolicExpr::Lt(
                    a.clone(),
                    Box::new(LatticeVMSymbolicExpr::Constant(AbstractInterval::from_i64(
                        2130706433,
                    ))),
                );
                rec(&e)
            }
            Self::WordAdd(b, c) => {
                let b_ais: [AbstractInterval; 4] = b.clone().map(|x| rec(&x));
                let c_ais: [AbstractInterval; 4] = c.clone().map(|x| rec(&x));

                word_add(&b_ais, &c_ais)
            }
            Self::WordSub(b, c) => {
                let b_ais: [AbstractInterval; 4] = b.clone().map(|x| rec(&x));
                let c_ais: [AbstractInterval; 4] = c.clone().map(|x| rec(&x));

                word_sub(&b_ais, &c_ais)
            }
            Self::WordSubU(b, c) => {
                let b_ais: [AbstractInterval; 4] = b.clone().map(|x| rec(&x));
                let c_ais: [AbstractInterval; 4] = c.clone().map(|x| rec(&x));

                word_subu(&b_ais, &c_ais)
            }
            Self::WordMul(b, c) => {
                let b_ais: [AbstractInterval; 4] = b.clone().map(|x| rec(&x));
                let c_ais: [AbstractInterval; 4] = c.clone().map(|x| rec(&x));

                word_mul(&b_ais, &c_ais)
            }
            Self::WordMulhs(b, c) => {
                let b_ais: [AbstractInterval; 4] = b.clone().map(|x| rec(&x));
                let c_ais: [AbstractInterval; 4] = c.clone().map(|x| rec(&x));

                word_mulhs(&b_ais, &c_ais)
            }
            Self::WordMulhu(b, c) => {
                let b_ais: [AbstractInterval; 4] = b.clone().map(|x| rec(&x));
                let c_ais: [AbstractInterval; 4] = c.clone().map(|x| rec(&x));

                word_mulhu(&b_ais, &c_ais)
            }
            Self::WordMultl(b, c) => {
                let b_ais: [AbstractInterval; 4] = b.clone().map(|x| rec(&x));
                let c_ais: [AbstractInterval; 4] = c.clone().map(|x| rec(&x));

                word_mult(&b_ais, &c_ais).0
            }
            Self::WordMulth(b, c) => {
                let b_ais: [AbstractInterval; 4] = b.clone().map(|x| rec(&x));
                let c_ais: [AbstractInterval; 4] = c.clone().map(|x| rec(&x));

                word_mult(&b_ais, &c_ais).1
            }

            Self::WordMultul(b, c) => {
                let b_ais: [AbstractInterval; 4] = b.clone().map(|x| rec(&x));
                let c_ais: [AbstractInterval; 4] = c.clone().map(|x| rec(&x));

                word_multu(&b_ais, &c_ais).0
            }
            Self::WordMultuh(b, c) => {
                let b_ais: [AbstractInterval; 4] = b.clone().map(|x| rec(&x));
                let c_ais: [AbstractInterval; 4] = c.clone().map(|x| rec(&x));

                word_multu(&b_ais, &c_ais).1
            }
            Self::WordDiv(b, c) => {
                let b_ais: [AbstractInterval; 4] = b.clone().map(|x| rec(&x));
                let c_ais: [AbstractInterval; 4] = c.clone().map(|x| rec(&x));

                word_div(&b_ais, &c_ais)
            }
            Self::WordSDiv(b, c) => {
                let b_ais: [AbstractInterval; 4] = b.clone().map(|x| rec(&x));
                let c_ais: [AbstractInterval; 4] = c.clone().map(|x| rec(&x));

                word_sdiv(&b_ais, &c_ais)
            }
            Self::WordLt(b, c) => {
                let b_ais: [AbstractInterval; 4] = b.clone().map(|x| rec(&x));
                let c_ais: [AbstractInterval; 4] = c.clone().map(|x| rec(&x));

                word_ltu(&b_ais, &c_ais)
            }
            Self::WordSLt(b, c) => {
                let b_ais: [AbstractInterval; 4] = b.clone().map(|x| rec(&x));
                let c_ais: [AbstractInterval; 4] = c.clone().map(|x| rec(&x));

                word_slt(&b_ais, &c_ais)
            }
            Self::WordAnd(b, c) => {
                let b_ais: [AbstractInterval; 4] = b.clone().map(|x| rec(&x));
                let c_ais: [AbstractInterval; 4] = c.clone().map(|x| rec(&x));

                word_and(&b_ais, &c_ais)
            }
            Self::WordOr(b, c) => {
                let b_ais: [AbstractInterval; 4] = b.clone().map(|x| rec(&x));
                let c_ais: [AbstractInterval; 4] = c.clone().map(|x| rec(&x));

                word_or(&b_ais, &c_ais)
            }
            Self::WordXOr(b, c) => {
                let b_ais: [AbstractInterval; 4] = b.clone().map(|x| rec(&x));
                let c_ais: [AbstractInterval; 4] = c.clone().map(|x| rec(&x));

                word_xor(&b_ais, &c_ais)
            }
            Self::WordEq(b, c) => {
                let b_ais: [AbstractInterval; 4] = b.clone().map(|x| rec(&x));
                let c_ais: [AbstractInterval; 4] = c.clone().map(|x| rec(&x));

                word_or(&b_ais, &c_ais)
            }
            Self::WordNEq(b, c) => {
                let b_ais: [AbstractInterval; 4] = b.clone().map(|x| rec(&x));
                let c_ais: [AbstractInterval; 4] = c.clone().map(|x| rec(&x));

                word_neq(&b_ais, &c_ais)
            }
            Self::WordSrl(b, c) => {
                let b_ais: [AbstractInterval; 4] = b.clone().map(|x| rec(&x));
                let c_ais: [AbstractInterval; 4] = c.clone().map(|x| rec(&x));

                word_srl(&b_ais, &c_ais)
            }
        }
    }
}

fn collect_add_vars(expr: &LatticeVMSymbolicExpr) -> Option<HashSet<usize>> {
    match expr {
        LatticeVMSymbolicExpr::Add(lhs, rhs) => {
            let mut left = collect_add_vars(lhs)?;
            let right = collect_add_vars(rhs)?;
            left.extend(right);
            Some(left)
        }
        LatticeVMSymbolicExpr::Variable(LatticeVMSymbolicVal { index, .. }) => {
            let mut set = HashSet::new();
            set.insert(index.clone());
            Some(set)
        }
        _ => None,
    }
}

fn collect_add_vars_vec(expr: &LatticeVMSymbolicExpr) -> Option<Vec<usize>> {
    match expr {
        LatticeVMSymbolicExpr::Add(lhs, rhs) => {
            let mut left = collect_add_vars_vec(lhs)?;
            let right = collect_add_vars_vec(rhs)?;
            left.extend(right);
            Some(left)
        }
        LatticeVMSymbolicExpr::Variable(LatticeVMSymbolicVal { index, .. }) => {
            let mut v = Vec::new();
            v.push(index.clone());
            Some(v)
        }
        _ => None,
    }
}

pub fn detect_conditional_var_sub_const_constraints(
    constraints: &[LatticeVMSymbolicExpr],
    prime: u32,
) -> Vec<(usize, usize, i64)> {
    let mut const_constraints = Vec::new();

    for c in constraints {
        // Mul(selector, Sub(lhs, rhs))
        if let LatticeVMSymbolicExpr::Mul(lhs_expr, rhs_expr) = c {
            // 1. the left is selector, and the right is Sub expr
            if let Some(s_idx) = get_curr_i(lhs_expr) {
                if let Some((v_idx, target)) = get_curr_i_sub_const(rhs_expr, prime) {
                    if s_idx != v_idx {
                        const_constraints.push((s_idx, v_idx, target));
                        continue;
                    }
                }
            }

            // 2. the left is Sub expr, and the right is the selector
            if let Some(s_idx) = get_curr_i(rhs_expr) {
                if let Some((v_idx, target)) = get_curr_i_sub_const(lhs_expr, prime) {
                    if s_idx != v_idx {
                        const_constraints.push((s_idx, v_idx, target));
                        continue;
                    }
                }
            }
        }
    }

    const_constraints
}

/// 1. selector * (curr[val_idx] - target_val) = 0
pub fn refine_conditional_constraints_var_sub_const(
    trace: &mut AbstractTrace,
    const_constraints: &[(usize, usize, i64)], // (selector_idx, value_idx, target_constant)
) -> MayBeFlag {
    let num_rows = trace.data.len();

    for r in 0..num_rows {
        for &(sel_idx, val_idx, target) in const_constraints {
            let selector = &trace.data[r][sel_idx];
            let target_interval = AbstractInterval::from_i64(target);

            // selector is one
            if selector.is_singleton() && selector.lo == 1 {
                let current_val = &trace.data[r][val_idx];
                if let Some(refined) = current_val.intersect(&target_interval) {
                    trace.data[r][val_idx] = refined;
                } else {
                    return MayBeFlag::False;
                }
            }

            // TODO: if the interval does not contain the target, the selector must be zero.
        }
    }

    MayBeFlag::MayBe
}

pub fn detect_conditional_var_sub_var_constraints(
    constraints: &[LatticeVMSymbolicExpr],
) -> Vec<(usize, usize, usize)> {
    let mut eq_constraints = Vec::new();

    for c in constraints {
        // Mul(selector, Sub(lhs, rhs))
        if let LatticeVMSymbolicExpr::Mul(lhs_expr, rhs_expr) = c {
            // 1. the left is selector, and the right is Sub expr
            if let Some(s_idx) = get_curr_i(lhs_expr) {
                if let Some((a_idx, b_idx)) = get_curr_i_sub_cur_j(rhs_expr) {
                    eq_constraints.push((s_idx, a_idx, b_idx));
                    continue;
                }
            }

            // 2. the left is Sub expr, and the right is the selector
            if let Some(s_idx) = get_curr_i(rhs_expr) {
                if let Some((a_idx, b_idx)) = get_curr_i_sub_cur_j(lhs_expr) {
                    eq_constraints.push((s_idx, a_idx, b_idx));
                    continue;
                }
            }
        }
    }

    eq_constraints
}

/// selector * (curr[a_idx] - curr[b_idx]) = 0
pub fn refine_conditional_constraints_var_sub_var(
    trace: &mut AbstractTrace,
    eq_constraints: &[(usize, usize, usize)], // (selector_idx, a_idx, b_idx)
) -> MayBeFlag {
    let num_rows = trace.data.len();

    for r in 0..num_rows {
        for &(sel_idx, a_idx, b_idx) in eq_constraints {
            let selector = &trace.data[r][sel_idx];

            if selector.is_singleton() && selector.lo == 1 {
                let a_val = trace.data[r][a_idx].clone();
                let b_val = trace.data[r][b_idx].clone();

                if let Some(intersected) = a_val.intersect(&b_val) {
                    trace.data[r][a_idx] = intersected.clone();
                    trace.data[r][b_idx] = intersected;
                } else {
                    return MayBeFlag::False;
                }
            }
            // TODO: if the intervals of a and b are disjoint, the selector must be zero.
        }
    }

    MayBeFlag::MayBe
}

pub fn move_sub_expr_to_right(expr: &LatticeVMSymbolicExpr) -> LatticeVMSymbolicExpr {
    // (a - c) + b ==> (a + b) - c
    if let LatticeVMSymbolicExpr::Add(lhs_1, rhs_1) = expr {
        if let LatticeVMSymbolicExpr::Sub(lhs_2, rhs_2) = *lhs_1.clone() {
            return LatticeVMSymbolicExpr::Sub(
                Box::new(LatticeVMSymbolicExpr::Add(lhs_2, rhs_1.clone())),
                rhs_2,
            );
        }
    }

    expr.clone()
}

//   a = L - de
//   d \in [d, d]
//   a \in [0, d - 1]
//   L \in [l, h]
//   a, b \notin FV(L)
// ----------------------------------------
//   e \in [[(l - (d - 1)) / d], [h / d]]
//
// *Proof*
//   0 <= L - de <= d - 1
//   L - (d - 1) <= de <= L
//   (L - (d - 1) / d) <= e <= L / d
//   (l - (d - 1)) / d <= e <= h / d

//   **Generalized Version**
// Affine Backward Interval Refinement (ABIR)
//
// a = L - d e
// d ∈ ℕ, d > 0
// a ∈ [amin, amax]
// L ∈ [ℓ, h]
// a, e ∉ FV(L)
// ----------------------------------------
// e :=
// e ∧ [
//   ⌊(ℓ - amax) / d ,
//   ⌊(h - amin) / d
// ]

#[derive(Debug, Clone)]
pub struct AbirConstraint {
    pub lhs_var: usize,                         // a
    pub affine_rhs: Box<LatticeVMSymbolicExpr>, // L
    pub quotient_var: usize,                    // e
    pub stride: u32,                            // d > 0
}

pub fn detect_abir_constraints(
    constraints: &[LatticeVMSymbolicExpr],
    prime: u32,
) -> Vec<AbirConstraint> {
    let mut result = Vec::new();

    for constraint in constraints {
        // Expect: a - RHS
        let (lhs, rhs) = match constraint {
            LatticeVMSymbolicExpr::Sub(lhs, rhs) => (lhs, rhs),
            _ => continue,
        };

        let lhs_var = match &**lhs {
            LatticeVMSymbolicExpr::Variable(v) => v.index,
            _ => continue,
        };

        // Normalize RHS: (L - d*e)
        let normalized_rhs = move_sub_expr_to_right(rhs);

        let (affine_rhs, de_term) = match normalized_rhs {
            LatticeVMSymbolicExpr::Sub(lhs, rhs) => (lhs, rhs),
            _ => continue,
        };

        // Match d * e
        let (quotient_var, stride) = match &*de_term {
            LatticeVMSymbolicExpr::Mul(x, y) => match (&**x, &**y) {
                (LatticeVMSymbolicExpr::Variable(v), LatticeVMSymbolicExpr::Constant(k))
                | (LatticeVMSymbolicExpr::Constant(k), LatticeVMSymbolicExpr::Variable(v)) => {
                    (v.index, k.as_canonical_u32(prime))
                }
                _ => continue,
            },
            LatticeVMSymbolicExpr::Variable(v) => (v.index, 1),
            _ => continue,
        };

        if stride == 0 {
            continue;
        }

        // Side condition: a, e ∉ FV(L)
        let mut free_vars = HashSet::new();
        gather_vars_simple(&affine_rhs, &mut free_vars);

        if free_vars.contains(&lhs_var) || free_vars.contains(&quotient_var) {
            continue;
        }

        result.push(AbirConstraint {
            lhs_var,
            affine_rhs: affine_rhs.clone(),
            quotient_var,
            stride,
        });
    }

    result
}

pub fn apply_abir_refinement(
    trace: &mut AbstractTrace,
    constraints: &[AbirConstraint],
    prime: u32,
) -> (MayBeFlag, Vec<(AbstractInterval, AbstractInterval)>) {
    let mut logs = Vec::new();

    for constraint in constraints {
        let AbirConstraint {
            lhs_var,
            affine_rhs,
            quotient_var,
            stride,
        } = constraint;

        let num_steps = trace.data.len();

        for step in 0..num_steps {
            let (past, future) = trace.data.split_at_mut(step + 1);
            let row = &mut past[past.len() - 1];
            let next_row = future.first().map(|r| &r[..]);

            // Evaluate L
            let rhs_interval = affine_rhs.eval(
                row,
                next_row,
                None,
                step == 0,
                step + 1 < num_steps,
                step + 1 == num_steps,
                prime,
            );

            // Weak ABIR:
            //   e ∈ floor((L - a) / d)
            let inferred_e = (rhs_interval - row[*lhs_var].clone()).div_floor(*stride as i64);

            if let Some(refined) = row[*quotient_var].intersect(&inferred_e) {
                logs.push((row[*quotient_var].clone(), refined.clone()));
                row[*quotient_var] = refined;
            } else {
                return (MayBeFlag::False, logs);
            }
        }
    }

    (MayBeFlag::MayBe, logs)
}

pub fn is_iszero_operator(
    constraint: &LatticeVMSymbolicExpr,
    prime: u32,
) -> Option<Vec<LatticeVMSymbolicExpr>> {
    if let LatticeVMSymbolicExpr::Mul(lhs, rhs) = constraint {
        if let LatticeVMSymbolicExpr::Sub(r_lhs, r_rhs) = *rhs.clone() {
            if let LatticeVMSymbolicExpr::Constant(c) = *r_rhs {
                if c.as_canonical_u32(prime) == 58079999 {
                    if let LatticeVMSymbolicExpr::Add(x, y) = *r_lhs {
                        let cond_when_zero = LatticeVMSymbolicExpr::WhenZero(
                            x.clone(),
                            Box::new(LatticeVMSymbolicExpr::Sub(
                                y.clone(),
                                Box::new(LatticeVMSymbolicExpr::Constant(AbstractInterval::one())),
                            )),
                        );
                        let cond_when_notzero = LatticeVMSymbolicExpr::WhenNonZero(
                            x,
                            Box::new(LatticeVMSymbolicExpr::Sub(
                                y,
                                Box::new(LatticeVMSymbolicExpr::Constant(AbstractInterval::zero())),
                            )),
                        );

                        return Some(vec![
                            LatticeVMSymbolicExpr::WhenNonZero(
                                lhs.clone(),
                                Box::new(cond_when_zero),
                            ),
                            LatticeVMSymbolicExpr::WhenNonZero(
                                lhs.clone(),
                                Box::new(cond_when_notzero),
                            ),
                        ]);
                    }
                }
            }
        }
    }

    None
}

pub fn is_koalabear_word_range(
    constraint: &LatticeVMSymbolicExpr,
    prime: u32,
) -> Option<LatticeVMSymbolicExpr> {
    if let LatticeVMSymbolicExpr::Mul(lhs, rhs) = constraint {
        if let LatticeVMSymbolicExpr::Sub(r_lhs, r_rhs) = *rhs.clone() {
            if let LatticeVMSymbolicExpr::Constant(c) = *r_rhs {
                if c.as_canonical_u32(prime) == 13227174 {
                    if let Some(word) = collect_add_vars_vec(&r_lhs) {
                        if word.len() == 4 {
                            let word_expr: Vec<_> = word
                                .iter()
                                .map(|&index| {
                                    LatticeVMSymbolicExpr::Variable(LatticeVMSymbolicVal {
                                        entry: LatticeVMSymbolicEntry::Main { is_curr: true },
                                        index,
                                    })
                                })
                                .collect();
                            let cond = reconstruct_symbolic_word(&word_expr, 0);
                            return Some(LatticeVMSymbolicExpr::WhenNonZero(
                                lhs.clone(),
                                Box::new(LatticeVMSymbolicExpr::Lt(
                                    Box::new(cond),
                                    Box::new(LatticeVMSymbolicExpr::Constant(
                                        AbstractInterval::from_i64(2130706433),
                                    )),
                                )),
                            ));
                        }
                    }
                }
            }
        }
    }

    None
}

pub fn is_boolean_constraint(constraint: &LatticeVMSymbolicExpr) -> Option<usize> {
    use LatticeVMSymbolicExpr::*;

    // helper: find `x - 1`
    fn is_x_minus_one(expr: &LatticeVMSymbolicExpr) -> Option<usize> {
        if let Sub(lhs, rhs) = expr {
            if let (
                Variable(LatticeVMSymbolicVal { index, .. }),
                Constant(AbstractInterval { lo: 1, hi: 1 }),
            ) = (&**lhs, &**rhs)
            {
                return Some(index.clone());
            }
        }
        None
    }

    if let Mul(a, b) = constraint {
        match (&**a, &**b) {
            // x * (x - 1)
            (Variable(LatticeVMSymbolicVal { index, .. }), rhs)
                if is_x_minus_one(rhs) == Some(index.clone()) =>
            {
                return Some(index.clone());
            }

            // (x - 1) * x
            (lhs, Variable(LatticeVMSymbolicVal { index, .. }))
                if is_x_minus_one(lhs) == Some(index.clone()) =>
            {
                return Some(index.clone());
            }

            _ => {}
        }
    }

    None
}

pub fn gather_boolean_variables(
    constraints: &[LatticeVMSymbolicExpr],
    multiplicities: &HashSet<usize>,
) -> Vec<usize> {
    let mut result = HashSet::new();
    for c in constraints {
        if let LatticeVMSymbolicExpr::Mul(lhs, rhs) = c {
            let add_vars = collect_add_vars(lhs);
            if let Some(add_vars) = add_vars {
                if add_vars.is_subset(multiplicities) {
                    if let Some(idx) = is_boolean_constraint(rhs) {
                        result.insert(idx);
                    }
                }
            }
        }
        if let Some(idx) = is_boolean_constraint(c) {
            result.insert(idx);
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

pub fn eval_base_constraints(
    trace: &AbstractTrace,
    public_vals: Option<&[AbstractInterval]>,
    constraints: &[LatticeVMSymbolicExpr],
    prime: u32,
    is_strict: bool,
    potential: &mut i32,
    memo: &mut HashSet<(usize, usize)>,
) -> MayBeFlag {
    let num_steps = trace.data.len();
    let mut is_all_true = true;
    for i in 0..num_steps {
        let mut j = 0;
        for tc in constraints {
            let flag = if is_strict {
                tc.eval(
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
                .is_strict_zero()
            } else {
                tc.eval(
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
                .is_zero(prime)
            };
            match flag {
                MayBeFlag::True => {}
                MayBeFlag::False => {
                    println!("{}", j);
                    return MayBeFlag::False;
                }
                MayBeFlag::MayBe => {
                    gather_vars(i, tc, memo);
                    is_all_true = false;
                    *potential += 1;
                }
            }
            j += 1;
        }
    }
    if is_all_true {
        MayBeFlag::True
    } else {
        MayBeFlag::MayBe
    }
}

#[derive(Clone)]
pub struct LatticeVMConstraints {
    pub air_constraints: Vec<LatticeVMSymbolicExpr>,
    pub lookup_constraints: Vec<LatticeVMSymbolicExpr>,
    pub pv_pos_constraints: Vec<LatticeVMSymbolicExpr>,
    pub pv_neg_constraints: Vec<LatticeVMSymbolicExpr>,
    pub blocking_constraints: Vec<(usize, LatticeVMSymbolicExpr)>,
}

impl LatticeVMConstraints {
    pub fn new(
        air_constraints: Vec<LatticeVMSymbolicExpr>,
        lookup_constraints: Vec<LatticeVMSymbolicExpr>,
    ) -> Self {
        LatticeVMConstraints {
            air_constraints,
            lookup_constraints,
            pv_pos_constraints: vec![],
            pv_neg_constraints: vec![],
            blocking_constraints: vec![],
        }
    }
}

pub fn eval_constraints(
    trace: &AbstractTrace,
    public_vals: Option<&[AbstractInterval]>,
    constraints: &LatticeVMConstraints,
    prime: u32,
) -> (MayBeFlag, i32, HashSet<(usize, usize)>) {
    let mut is_all_true = true;
    let mut potential = 0;
    let mut memo = HashSet::<(usize, usize)>::new();

    // ########## Check Blocking Constraints ###########
    let num_steps = trace.data.len();
    let mut blocking_is_all = true;
    for (i, bc) in &constraints.blocking_constraints {
        let flag = bc
            .eval(
                &trace.data[*i],
                if *i + 1 < num_steps {
                    Some(&trace.data[i + 1])
                } else {
                    None
                },
                public_vals,
                *i == 0,
                *i < num_steps - 1,
                *i == num_steps - 1,
                prime,
            )
            .is_zero(prime);

        match flag {
            MayBeFlag::True => {}
            MayBeFlag::False | MayBeFlag::MayBe => {
                blocking_is_all = false;
            }
        }
    }
    if !constraints.blocking_constraints.is_empty() && blocking_is_all {
        return (MayBeFlag::False, 0, memo);
    }

    // ########## Check AIR Constraints ###########
    let air_flag = eval_base_constraints(
        trace,
        public_vals,
        &constraints.air_constraints,
        prime,
        false,
        &mut potential,
        &mut memo,
    );
    match air_flag {
        MayBeFlag::True => {}
        MayBeFlag::False => return (MayBeFlag::False, 0, memo),
        MayBeFlag::MayBe => is_all_true = false,
    }

    // ########## Check Lookup Constraints ###########
    let air_flag = eval_base_constraints(
        trace,
        public_vals,
        &constraints.lookup_constraints,
        prime,
        true,
        &mut potential,
        &mut memo,
    );
    match air_flag {
        MayBeFlag::True => {}
        MayBeFlag::False => return (MayBeFlag::False, 0, memo),
        MayBeFlag::MayBe => is_all_true = false,
    }

    // ########## Check Public-Positive Constraints ###########
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

    // ########## Check Public-Negative Constraints ###########
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
) -> (Option<Vec<AbstractTrace>>, bool) {
    if trace.singleton_positions.len() == trace.data.len() * trace.data[0].len() {
        return (None, false);
    }
    if refinment_target_indicies.is_empty() {
        return (None, false);
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
        (Some(results), true)
    } else {
        (Some(vec![trace.clone(), trace.clone()]), false)
    }
    //}
}

pub fn make_impl_constraint(
    opcode_val: i64,
    opcode_var: &LatticeVMSymbolicExpr,
    expr: LatticeVMSymbolicExpr,
    prime: u32,
) -> Option<LatticeVMSymbolicExpr> {
    if let LatticeVMSymbolicExpr::Constant(c) = opcode_var {
        if c.as_canonical_u32(prime) as i64 == opcode_val {
            Some(expr)
        } else {
            None
        }
    } else {
        Some(LatticeVMSymbolicExpr::WhenZero(
            Box::new(LatticeVMSymbolicExpr::Sub(
                Box::new(opcode_var.clone()),
                Box::new(LatticeVMSymbolicExpr::Constant(AbstractInterval::from_i64(
                    opcode_val,
                ))),
            )),
            Box::new(expr),
        ))
    }
}

pub fn add_blocking_constraint(
    output_columns: &[usize],
    constraints: &mut LatticeVMConstraints,
    base_abs_main_trace_data: &Vec<Vec<AbstractInterval>>,
    i: usize,
) {
    for j in output_columns {
        constraints.blocking_constraints.push((
            i,
            LatticeVMSymbolicExpr::Sub(
                Box::new(LatticeVMSymbolicExpr::Variable(LatticeVMSymbolicVal {
                    entry: LatticeVMSymbolicEntry::Main { is_curr: true },
                    index: *j,
                })),
                Box::new(LatticeVMSymbolicExpr::Constant(
                    base_abs_main_trace_data[i][*j].clone(),
                )),
            ),
        ));
    }
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

        let b_eval = b.eval(&trace.data[0], None, None, false, false, false, prime);

        //assert!(false);
    }
}
