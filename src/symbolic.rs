use std::collections::HashSet;
use std::fmt;
use std::hash::Hash;
use std::ops::{Add, Mul, Neg, Sub};

use serde::Serialize;

use crate::interval::{msb_maybe, AbstractInterval, MayBeFlag};
use crate::wordop::{
    reconstruct_symbolic_word, word_addu, word_and, word_div, word_eq, word_ltu, word_mul,
    word_mulhs, word_mulhu, word_mult, word_multu, word_neq, word_or, word_sdiv, word_sle,
    word_slt, word_srl, word_subu, word_xor,
};

/// Identifies the source and temporal position of a symbolic value within the
/// Lattice VM execution model.
///
/// A symbolic entry specifies both:
///
/// * **Trace segment** — Which logical table the value belongs to
/// * **Row context** — Whether the value refers to the current or next row
///
/// This abstraction allows symbolic expressions to uniformly reference values
/// across multiple trace domains used in constraint systems (e.g., AIR/STARK-like
/// formulations).
///
/// # Variants
///
/// * `Main { is_curr }` — Value from the main execution trace
/// * `Permutation { is_curr }` — Value from the permutation trace
/// * `Preprocessed { is_curr }` — Value from preprocessed auxiliary data
/// * `Public` — Public input (row-independent)
///
/// The `is_curr` flag indicates whether the value refers to:
///
/// * `true`  → the current row
/// * `false` → the next row
///
/// Public values have no row dependence.
///
/// # Display Format
///
/// The `Display` implementation renders entries as:
///
/// * `"curr"` — Current row value
/// * `"next"` — Next row value
/// * `"public"` — Public input
///
/// This representation is designed for compact debugging and constraint printing.
///
/// # Use Cases
///
/// Typically embedded within [`ZEBRASymbolicVal`] to form leaf nodes in
/// symbolic expression trees.
///
/// # See Also
///
/// * [`ZEBRASymbolicVal`] — Symbolic variable representation
/// * [`ZEBRASymbolicExpr`] — Symbolic expression tree
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize)]
pub enum ZEBRASymbolicEntry {
    Main { is_curr: bool },
    Permutation { is_curr: bool },
    Preprocessed { is_curr: bool },
    Public,
}

impl fmt::Display for ZEBRASymbolicEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZEBRASymbolicEntry::Main { is_curr } => {
                write!(f, "{}", if *is_curr { "curr" } else { "next" })
            }
            ZEBRASymbolicEntry::Permutation { is_curr } => {
                write!(f, "{}", if *is_curr { "curr" } else { "next" })
            }
            ZEBRASymbolicEntry::Preprocessed { is_curr } => {
                write!(f, "{}", if *is_curr { "curr" } else { "next" })
            }
            ZEBRASymbolicEntry::Public => write!(f, "{}", "public"),
        }
    }
}

/// Represents a single symbolic variable referencing a column in a trace segment.
///
/// A symbolic value combines:
///
/// * A [`ZEBRASymbolicEntry`] describing the trace domain and row context
/// * A column index within that domain
///
/// Together, these uniquely identify a concrete value in the execution trace
/// during symbolic constraint evaluation.
///
/// # Fields
///
/// * `entry` — Source trace segment and row selector
/// * `index` — Column index within that segment
///
/// # Semantics
///
/// Conceptually corresponds to:
///
/// ```text
/// entry[index]
/// ```
///
/// Examples:
///
/// * `curr[5]` — Column 5 of the current main trace row
/// * `next[2]` — Column 2 of the next row
/// * `public[0]` — First public input value
///
/// # Display Format
///
/// Printed as `<entry>[<index>]`, e.g.:
///
/// ```text
/// curr[3]
/// next[7]
/// public[0]
/// ```
///
/// # Use Cases
///
/// Serves as a leaf node in [`ZEBRASymbolicExpr`] trees representing
/// constraints over traces.
///
/// # See Also
///
/// * [`ZEBRASymbolicEntry`] — Domain and row selector
/// * [`ZEBRASymbolicExpr`] — Expression tree using these variables
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize)]
pub struct ZEBRASymbolicVal {
    pub entry: ZEBRASymbolicEntry,
    pub index: usize,
}

impl fmt::Display for ZEBRASymbolicVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}[{}]", self.entry, self.index)
    }
}

/// Symbolic expression tree used to represent constraints over Lattice VM traces.
///
/// Expressions are constructed from symbolic variables, constants, arithmetic
/// operators, and structural predicates describing the execution context.
/// They are typically evaluated over abstract intervals or concrete field
/// values during constraint solving.
///
/// This enum forms the core intermediate representation (IR) for symbolic
/// reasoning about VM execution.
///
/// # Structural Predicates
///
/// These variants describe properties of the current row position within
/// the trace:
///
/// * `IsFirstRow` — True only for the first row
/// * `IsTransition` — True for rows with a valid successor
/// * `IsLastRow` — True only for the final row
///
/// Such predicates are commonly used in AIR-style constraints to enforce
/// boundary conditions.
///
/// # Evaluation Context
///
/// Expressions are evaluated with access to:
///
/// * Current row values
/// * Next row values (if available)
/// * Public inputs
/// * Field modulus
///
/// # Use Cases
///
/// * Transition constraints
/// * Boundary constraints
/// * Lookup conditions
/// * Symbolic simplification and analysis
///
/// # Notes
///
/// Additional variants (not shown here) typically include arithmetic
/// operations, constants, and variable references.
///
/// # See Also
///
/// * [`ZEBRASymbolicVal`] — Variable leaf nodes
/// * [`ZEBRASymbolicEntry`] — Trace domain selector
#[derive(Clone, Debug, Serialize, PartialEq, Eq, Hash)]
pub enum ZEBRASymbolicExpr {
    IsFirstRow,
    IsTransition,
    IsLastRow,
    WhenNonZero(Box<Self>, Box<Self>),
    WhenZero(Box<Self>, Box<Self>),
    Constant(AbstractInterval),
    Variable(ZEBRASymbolicVal),
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
    SRLCarry(Box<Self>, Box<Self>),
    Lt(Box<Self>, Box<Self>),
    Msb(Box<Self>),
    Neg(Box<Self>),
    Flip(Box<Self>),
    KoalaBearRange(Box<Self>),
    BabyBearRange(Box<Self>),
    WordAddU([Box<Self>; 4], [Box<Self>; 4]),
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
    WordSLe([Box<Self>; 4], [Box<Self>; 4]),
    WordSLt([Box<Self>; 4], [Box<Self>; 4]),
    WordAnd([Box<Self>; 4], [Box<Self>; 4]),
    WordOr([Box<Self>; 4], [Box<Self>; 4]),
    WordXOr([Box<Self>; 4], [Box<Self>; 4]),
    WordEq([Box<Self>; 4], [Box<Self>; 4]),
    WordNEq([Box<Self>; 4], [Box<Self>; 4]),
    WordSrl([Box<Self>; 4], [Box<Self>; 4]),
}

impl Default for ZEBRASymbolicExpr {
    fn default() -> Self {
        Self::Constant(AbstractInterval::zero())
    }
}

impl fmt::Display for ZEBRASymbolicExpr {
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
            Self::SRLCarry(x, y) => write!(f, "({} >>~ {})", x, y),
            Self::Lt(x, y) => write!(f, "({} < {})", x, y),
            Self::Msb(x) => write!(f, "msb({})", x),
            Self::Neg(x) => write!(f, "-{}", x),
            Self::Flip(x) => write!(f, "~{}", x),
            Self::KoalaBearRange(x) => write!(f, "🐨{}🐨", x),
            Self::BabyBearRange(x) => write!(f, "👶{}👶", x),
            Self::WordAddU(b, c) => write!(
                f,
                "[{}, {}, {}, {}] +u [{}, {}, {}, {}]",
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
            Self::WordSLe(b, c) => write!(
                f,
                "[{}, {}, {}, {}] <=_s [{}, {}, {}, {}]",
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

impl<T: Into<Self>> Add<T> for ZEBRASymbolicExpr {
    type Output = Self;
    fn add(self, rhs: T) -> Self {
        Self::Add(Box::new(self), Box::new(rhs.into()))
    }
}

impl<T: Into<Self>> Sub<T> for ZEBRASymbolicExpr {
    type Output = Self;
    fn sub(self, rhs: T) -> Self {
        Self::Sub(Box::new(self), Box::new(rhs.into()))
    }
}

impl<T: Into<Self>> Mul<T> for ZEBRASymbolicExpr {
    type Output = Self;
    fn mul(self, rhs: T) -> Self {
        Self::Mul(Box::new(self), Box::new(rhs.into()))
    }
}

impl Neg for ZEBRASymbolicExpr {
    type Output = Self;
    fn neg(self) -> Self {
        Self::Neg(Box::new(self))
    }
}

impl From<ZEBRASymbolicVal> for ZEBRASymbolicExpr {
    fn from(var: ZEBRASymbolicVal) -> Self {
        Self::Variable(var)
    }
}

impl ZEBRASymbolicExpr {
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
        let rec = |a: &ZEBRASymbolicExpr| {
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
                    let val = rec(b);
                    match val.is_zero(prime) {
                        MayBeFlag::True => val,
                        MayBeFlag::False => {
                            if let MayBeFlag::MayBe = cond.is_zero(prime) {
                                if 0 < val.lo {
                                    AbstractInterval { lo: 0, hi: val.hi }
                                } else {
                                    AbstractInterval { lo: val.lo, hi: 0 }
                                }
                            } else {
                                val
                            }
                        }
                        MayBeFlag::MayBe => val,
                    }
                }
            }
            Self::WhenZero(a, b) => {
                let cond = rec(a);
                if let MayBeFlag::False = cond.is_zero(prime) {
                    AbstractInterval::zero()
                } else {
                    let val = rec(b);
                    match val.is_zero(prime) {
                        MayBeFlag::True => val,
                        MayBeFlag::False => {
                            if let MayBeFlag::MayBe = cond.is_zero(prime) {
                                if 0 < val.lo {
                                    AbstractInterval { lo: 0, hi: val.hi }
                                } else {
                                    AbstractInterval { lo: val.lo, hi: 0 }
                                }
                            } else {
                                val
                            }
                        }
                        MayBeFlag::MayBe => val,
                    }
                }
            }
            Self::Constant(c) => c.clone(),
            Self::Variable(cell) => match cell.entry {
                ZEBRASymbolicEntry::Main { is_curr } => {
                    if is_curr {
                        curr_row[cell.index].clone()
                    } else {
                        match next_row {
                            Some(nr) => nr[cell.index].clone(),
                            None => AbstractInterval::zero(), //panic!("next_row not provided for next-row variable"),
                        }
                    }
                }
                ZEBRASymbolicEntry::Preprocessed { is_curr } => {
                    if is_curr {
                        curr_row[cell.index].clone()
                    } else {
                        match next_row {
                            Some(nr) => nr[cell.index].clone(),
                            None => AbstractInterval::zero(), //panic!("next_row not provided for next-row variable"),
                        }
                    }
                }
                ZEBRASymbolicEntry::Permutation { is_curr } => {
                    if is_curr {
                        curr_row[cell.index].clone()
                    } else {
                        match next_row {
                            Some(nr) => nr[cell.index].clone(),
                            None => AbstractInterval::zero(), //panic!("next_row not provided for next-row variable"),
                        }
                    }
                }
                ZEBRASymbolicEntry::Public => match public_vals {
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
            Self::SRLCarry(a, b) => rec(a).shr_carry(rec(b)).1,
            Self::Lt(a, b) => rec(a).ltu(rec(b)),
            Self::Neg(a) => -rec(a),
            Self::Flip(a) => {
                let is_zero = rec(a).is_zero(prime);
                if let MayBeFlag::True = is_zero {
                    AbstractInterval::one()
                } else {
                    let tmp = ZEBRASymbolicExpr::Sub(
                        Box::new(ZEBRASymbolicExpr::Constant(AbstractInterval::one())),
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
                let e = ZEBRASymbolicExpr::Flip(Box::new(ZEBRASymbolicExpr::Lt(
                    a.clone(),
                    Box::new(ZEBRASymbolicExpr::Constant(
                        AbstractInterval::from_i128(2130706433),
                    )),
                )));
                rec(&e)
            }
            Self::BabyBearRange(a) => {
                let e = ZEBRASymbolicExpr::Flip(Box::new(ZEBRASymbolicExpr::Lt(
                    a.clone(),
                    Box::new(ZEBRASymbolicExpr::Constant(
                        AbstractInterval::from_i128(2013265921),
                    )),
                )));
                rec(&e)
            }
            Self::WordAddU(b, c) => {
                let b_ais: [AbstractInterval; 4] = b.clone().map(|x| rec(&x));
                let c_ais: [AbstractInterval; 4] = c.clone().map(|x| rec(&x));

                word_addu(&b_ais, &c_ais)
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
            Self::WordSLe(b, c) => {
                let b_ais: [AbstractInterval; 4] = b.clone().map(|x| rec(&x));
                let c_ais: [AbstractInterval; 4] = c.clone().map(|x| rec(&x));

                word_sle(&b_ais, &c_ais)
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

                word_eq(&b_ais, &c_ais)
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

pub fn gather_vars_simple(expr: &ZEBRASymbolicExpr, memo: &mut HashSet<usize>) {
    match expr {
        ZEBRASymbolicExpr::Variable(lattice_vmsymbolic_val) => {
            memo.insert(lattice_vmsymbolic_val.index);
        }
        ZEBRASymbolicExpr::Add(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars_simple(&lattice_vmsymbolic_expr, memo);
            gather_vars_simple(&lattice_vmsymbolic_expr1, memo);
        }
        ZEBRASymbolicExpr::Sub(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars_simple(&lattice_vmsymbolic_expr, memo);
            gather_vars_simple(&lattice_vmsymbolic_expr1, memo);
        }
        ZEBRASymbolicExpr::Mul(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars_simple(&lattice_vmsymbolic_expr, memo);
            gather_vars_simple(&lattice_vmsymbolic_expr1, memo);
        }
        ZEBRASymbolicExpr::Neg(lattice_vmsymbolic_expr) => {
            gather_vars_simple(&lattice_vmsymbolic_expr, memo);
        }
        ZEBRASymbolicExpr::And(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars_simple(&lattice_vmsymbolic_expr, memo);
            gather_vars_simple(&lattice_vmsymbolic_expr1, memo);
        }
        ZEBRASymbolicExpr::Or(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars_simple(&lattice_vmsymbolic_expr, memo);
            gather_vars_simple(&lattice_vmsymbolic_expr1, memo);
        }
        ZEBRASymbolicExpr::Xor(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars_simple(&lattice_vmsymbolic_expr, memo);
            gather_vars_simple(&lattice_vmsymbolic_expr1, memo);
        }
        ZEBRASymbolicExpr::Lt(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars_simple(&lattice_vmsymbolic_expr, memo);
            gather_vars_simple(&lattice_vmsymbolic_expr1, memo);
        }
        ZEBRASymbolicExpr::SRL(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars_simple(&lattice_vmsymbolic_expr, memo);
            gather_vars_simple(&lattice_vmsymbolic_expr1, memo);
        }
        ZEBRASymbolicExpr::SRLCarry(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars_simple(&lattice_vmsymbolic_expr, memo);
            gather_vars_simple(&lattice_vmsymbolic_expr1, memo);
        }
        ZEBRASymbolicExpr::Msb(lattice_vmsymbolic_expr) => {
            gather_vars_simple(&lattice_vmsymbolic_expr, memo);
        }
        ZEBRASymbolicExpr::Flip(lattice_vmsymbolic_expr) => {
            gather_vars_simple(&lattice_vmsymbolic_expr, memo);
        }
        _ => {}
    }
}

pub fn gather_vars(
    row_index: usize,
    expr: &ZEBRASymbolicExpr,
    memo: &mut HashSet<(usize, usize)>,
) {
    match expr {
        ZEBRASymbolicExpr::Variable(lattice_vmsymbolic_val) => {
            memo.insert((row_index, lattice_vmsymbolic_val.index));
        }
        ZEBRASymbolicExpr::Add(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars(row_index, &lattice_vmsymbolic_expr, memo);
            gather_vars(row_index, &lattice_vmsymbolic_expr1, memo);
        }
        ZEBRASymbolicExpr::Sub(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars(row_index, &lattice_vmsymbolic_expr, memo);
            gather_vars(row_index, &lattice_vmsymbolic_expr1, memo);
        }
        ZEBRASymbolicExpr::Mul(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars(row_index, &lattice_vmsymbolic_expr, memo);
            gather_vars(row_index, &lattice_vmsymbolic_expr1, memo);
        }
        ZEBRASymbolicExpr::Neg(lattice_vmsymbolic_expr) => {
            gather_vars(row_index, &lattice_vmsymbolic_expr, memo);
        }
        ZEBRASymbolicExpr::And(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars(row_index, &lattice_vmsymbolic_expr, memo);
            gather_vars(row_index, &lattice_vmsymbolic_expr1, memo);
        }
        ZEBRASymbolicExpr::Or(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars(row_index, &lattice_vmsymbolic_expr, memo);
            gather_vars(row_index, &lattice_vmsymbolic_expr1, memo);
        }
        ZEBRASymbolicExpr::Xor(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars(row_index, &lattice_vmsymbolic_expr, memo);
            gather_vars(row_index, &lattice_vmsymbolic_expr1, memo);
        }
        ZEBRASymbolicExpr::Lt(lattice_vmsymbolic_expr, lattice_vmsymbolic_expr1) => {
            gather_vars(row_index, &lattice_vmsymbolic_expr, memo);
            gather_vars(row_index, &lattice_vmsymbolic_expr1, memo);
        }
        _ => {}
    }
}

/// Collects all column indices referenced by `expr` into `cols`.
///
/// Unlike [`gather_vars_simple`], this function handles every variant of
/// [`ZEBRASymbolicExpr`], including `WhenZero`/`WhenNonZero`, `MulLo`,
/// `MulHiSS`/`MulHiUU`, `KoalaBearRange`/`BabyBearRange`, and all `Word*`
/// word-operation variants.  It is therefore suitable for accurate sparsity
/// analysis where missing branches would silently under-count coverage.
pub fn gather_cols(expr: &ZEBRASymbolicExpr, cols: &mut HashSet<usize>) {
    match expr {
        ZEBRASymbolicExpr::Variable(v) => {
            cols.insert(v.index);
        }
        ZEBRASymbolicExpr::Constant(_)
        | ZEBRASymbolicExpr::IsFirstRow
        | ZEBRASymbolicExpr::IsTransition
        | ZEBRASymbolicExpr::IsLastRow => {}
        ZEBRASymbolicExpr::WhenNonZero(cond, body)
        | ZEBRASymbolicExpr::WhenZero(cond, body) => {
            gather_cols(cond, cols);
            gather_cols(body, cols);
        }
        ZEBRASymbolicExpr::Neg(e)
        | ZEBRASymbolicExpr::Msb(e)
        | ZEBRASymbolicExpr::Flip(e)
        | ZEBRASymbolicExpr::KoalaBearRange(e)
        | ZEBRASymbolicExpr::BabyBearRange(e) => {
            gather_cols(e, cols);
        }
        ZEBRASymbolicExpr::Add(a, b)
        | ZEBRASymbolicExpr::Sub(a, b)
        | ZEBRASymbolicExpr::Mul(a, b)
        | ZEBRASymbolicExpr::MulLo(a, b)
        | ZEBRASymbolicExpr::MulHiSS(a, b)
        | ZEBRASymbolicExpr::MulHiUU(a, b)
        | ZEBRASymbolicExpr::And(a, b)
        | ZEBRASymbolicExpr::Or(a, b)
        | ZEBRASymbolicExpr::Xor(a, b)
        | ZEBRASymbolicExpr::SRL(a, b)
        | ZEBRASymbolicExpr::SRLCarry(a, b)
        | ZEBRASymbolicExpr::Lt(a, b) => {
            gather_cols(a, cols);
            gather_cols(b, cols);
        }
        ZEBRASymbolicExpr::WordAddU(a, b)
        | ZEBRASymbolicExpr::WordSubU(a, b)
        | ZEBRASymbolicExpr::WordMul(a, b)
        | ZEBRASymbolicExpr::WordMulhu(a, b)
        | ZEBRASymbolicExpr::WordMulhs(a, b)
        | ZEBRASymbolicExpr::WordMultl(a, b)
        | ZEBRASymbolicExpr::WordMulth(a, b)
        | ZEBRASymbolicExpr::WordMultul(a, b)
        | ZEBRASymbolicExpr::WordMultuh(a, b)
        | ZEBRASymbolicExpr::WordDiv(a, b)
        | ZEBRASymbolicExpr::WordSDiv(a, b)
        | ZEBRASymbolicExpr::WordLt(a, b)
        | ZEBRASymbolicExpr::WordSLe(a, b)
        | ZEBRASymbolicExpr::WordSLt(a, b)
        | ZEBRASymbolicExpr::WordAnd(a, b)
        | ZEBRASymbolicExpr::WordOr(a, b)
        | ZEBRASymbolicExpr::WordXOr(a, b)
        | ZEBRASymbolicExpr::WordEq(a, b)
        | ZEBRASymbolicExpr::WordNEq(a, b)
        | ZEBRASymbolicExpr::WordSrl(a, b) => {
            for e in a.iter().chain(b.iter()) {
                gather_cols(e, cols);
            }
        }
    }
}

/// Count the total number of arithmetic operation nodes in `expr` (T_arith).
/// Every non-leaf node counts as one operation; leaf nodes (Variable, Constant,
/// row predicates) contribute zero. Word* variants recurse into their limbs.
pub fn count_arith_ops(expr: &ZEBRASymbolicExpr) -> usize {
    match expr {
        ZEBRASymbolicExpr::Variable(_)
        | ZEBRASymbolicExpr::Constant(_)
        | ZEBRASymbolicExpr::IsFirstRow
        | ZEBRASymbolicExpr::IsTransition
        | ZEBRASymbolicExpr::IsLastRow => 0,
        ZEBRASymbolicExpr::Neg(e)
        | ZEBRASymbolicExpr::Msb(e)
        | ZEBRASymbolicExpr::Flip(e)
        | ZEBRASymbolicExpr::KoalaBearRange(e)
        | ZEBRASymbolicExpr::BabyBearRange(e) => 1 + count_arith_ops(e),
        ZEBRASymbolicExpr::WhenNonZero(a, b)
        | ZEBRASymbolicExpr::WhenZero(a, b)
        | ZEBRASymbolicExpr::Add(a, b)
        | ZEBRASymbolicExpr::Sub(a, b)
        | ZEBRASymbolicExpr::Mul(a, b)
        | ZEBRASymbolicExpr::MulLo(a, b)
        | ZEBRASymbolicExpr::MulHiSS(a, b)
        | ZEBRASymbolicExpr::MulHiUU(a, b)
        | ZEBRASymbolicExpr::And(a, b)
        | ZEBRASymbolicExpr::Or(a, b)
        | ZEBRASymbolicExpr::Xor(a, b)
        | ZEBRASymbolicExpr::SRL(a, b)
        | ZEBRASymbolicExpr::SRLCarry(a, b)
        | ZEBRASymbolicExpr::Lt(a, b) => 1 + count_arith_ops(a) + count_arith_ops(b),
        ZEBRASymbolicExpr::WordAddU(a, b)
        | ZEBRASymbolicExpr::WordSubU(a, b)
        | ZEBRASymbolicExpr::WordMul(a, b)
        | ZEBRASymbolicExpr::WordMulhu(a, b)
        | ZEBRASymbolicExpr::WordMulhs(a, b)
        | ZEBRASymbolicExpr::WordMultl(a, b)
        | ZEBRASymbolicExpr::WordMulth(a, b)
        | ZEBRASymbolicExpr::WordMultul(a, b)
        | ZEBRASymbolicExpr::WordMultuh(a, b)
        | ZEBRASymbolicExpr::WordDiv(a, b)
        | ZEBRASymbolicExpr::WordSDiv(a, b)
        | ZEBRASymbolicExpr::WordLt(a, b)
        | ZEBRASymbolicExpr::WordSLe(a, b)
        | ZEBRASymbolicExpr::WordSLt(a, b)
        | ZEBRASymbolicExpr::WordAnd(a, b)
        | ZEBRASymbolicExpr::WordOr(a, b)
        | ZEBRASymbolicExpr::WordXOr(a, b)
        | ZEBRASymbolicExpr::WordEq(a, b)
        | ZEBRASymbolicExpr::WordNEq(a, b)
        | ZEBRASymbolicExpr::WordSrl(a, b) => a
            .iter()
            .chain(b.iter())
            .map(|e| count_arith_ops(e))
            .sum(),
    }
}

/// CSE-aware inner helper: count arithmetic ops in `expr`, skipping any
/// subexpression that has already been counted (recorded in `seen`).
fn count_arith_ops_with_cse(
    expr: &ZEBRASymbolicExpr,
    seen: &mut HashSet<ZEBRASymbolicExpr>,
) -> usize {
    // Leaf nodes carry no operation cost and are never deduplicated.
    match expr {
        ZEBRASymbolicExpr::Variable(_)
        | ZEBRASymbolicExpr::Constant(_)
        | ZEBRASymbolicExpr::IsFirstRow
        | ZEBRASymbolicExpr::IsTransition
        | ZEBRASymbolicExpr::IsLastRow => return 0,
        _ => {}
    }
    // insert returns false when already present → already counted, skip.
    if !seen.insert(expr.clone()) {
        return 0;
    }
    match expr {
        ZEBRASymbolicExpr::Neg(e)
        | ZEBRASymbolicExpr::Msb(e)
        | ZEBRASymbolicExpr::Flip(e)
        | ZEBRASymbolicExpr::KoalaBearRange(e)
        | ZEBRASymbolicExpr::BabyBearRange(e) => 1 + count_arith_ops_with_cse(e, seen),
        ZEBRASymbolicExpr::WhenNonZero(a, b)
        | ZEBRASymbolicExpr::WhenZero(a, b)
        | ZEBRASymbolicExpr::Add(a, b)
        | ZEBRASymbolicExpr::Sub(a, b)
        | ZEBRASymbolicExpr::Mul(a, b)
        | ZEBRASymbolicExpr::MulLo(a, b)
        | ZEBRASymbolicExpr::MulHiSS(a, b)
        | ZEBRASymbolicExpr::MulHiUU(a, b)
        | ZEBRASymbolicExpr::And(a, b)
        | ZEBRASymbolicExpr::Or(a, b)
        | ZEBRASymbolicExpr::Xor(a, b)
        | ZEBRASymbolicExpr::SRL(a, b)
        | ZEBRASymbolicExpr::SRLCarry(a, b)
        | ZEBRASymbolicExpr::Lt(a, b) => {
            1 + count_arith_ops_with_cse(a, seen) + count_arith_ops_with_cse(b, seen)
        }
        ZEBRASymbolicExpr::WordAddU(a, b)
        | ZEBRASymbolicExpr::WordSubU(a, b)
        | ZEBRASymbolicExpr::WordMul(a, b)
        | ZEBRASymbolicExpr::WordMulhu(a, b)
        | ZEBRASymbolicExpr::WordMulhs(a, b)
        | ZEBRASymbolicExpr::WordMultl(a, b)
        | ZEBRASymbolicExpr::WordMulth(a, b)
        | ZEBRASymbolicExpr::WordMultul(a, b)
        | ZEBRASymbolicExpr::WordMultuh(a, b)
        | ZEBRASymbolicExpr::WordDiv(a, b)
        | ZEBRASymbolicExpr::WordSDiv(a, b)
        | ZEBRASymbolicExpr::WordLt(a, b)
        | ZEBRASymbolicExpr::WordSLe(a, b)
        | ZEBRASymbolicExpr::WordSLt(a, b)
        | ZEBRASymbolicExpr::WordAnd(a, b)
        | ZEBRASymbolicExpr::WordOr(a, b)
        | ZEBRASymbolicExpr::WordXOr(a, b)
        | ZEBRASymbolicExpr::WordEq(a, b)
        | ZEBRASymbolicExpr::WordNEq(a, b)
        | ZEBRASymbolicExpr::WordSrl(a, b) => a
            .iter()
            .chain(b.iter())
            .map(|e| count_arith_ops_with_cse(e, seen))
            .sum(),
        // leaves already handled above
        ZEBRASymbolicExpr::Variable(_)
        | ZEBRASymbolicExpr::Constant(_)
        | ZEBRASymbolicExpr::IsFirstRow
        | ZEBRASymbolicExpr::IsTransition
        | ZEBRASymbolicExpr::IsLastRow => unreachable!(),
    }
}

/// Count T_arith across all `exprs` with CSE: common subexpressions shared
/// across multiple constraints are counted only once.
pub fn count_arith_ops_cse(exprs: &[&ZEBRASymbolicExpr]) -> usize {
    let mut seen = HashSet::new();
    exprs
        .iter()
        .map(|e| count_arith_ops_with_cse(e, &mut seen))
        .sum()
}

/// Collect all distinct `(entry, column-index)` neighbor pairs referenced by `expr`.
/// This is the AIR neighborhood N: each unique [`ZEBRASymbolicVal`] is one cell.
pub fn gather_neighbors(expr: &ZEBRASymbolicExpr, neighbors: &mut HashSet<ZEBRASymbolicVal>) {
    match expr {
        ZEBRASymbolicExpr::Variable(v) => {
            neighbors.insert(v.clone());
        }
        ZEBRASymbolicExpr::Constant(_)
        | ZEBRASymbolicExpr::IsFirstRow
        | ZEBRASymbolicExpr::IsTransition
        | ZEBRASymbolicExpr::IsLastRow => {}
        ZEBRASymbolicExpr::WhenNonZero(a, b) | ZEBRASymbolicExpr::WhenZero(a, b) => {
            gather_neighbors(a, neighbors);
            gather_neighbors(b, neighbors);
        }
        ZEBRASymbolicExpr::Neg(e)
        | ZEBRASymbolicExpr::Msb(e)
        | ZEBRASymbolicExpr::Flip(e)
        | ZEBRASymbolicExpr::KoalaBearRange(e)
        | ZEBRASymbolicExpr::BabyBearRange(e) => gather_neighbors(e, neighbors),
        ZEBRASymbolicExpr::Add(a, b)
        | ZEBRASymbolicExpr::Sub(a, b)
        | ZEBRASymbolicExpr::Mul(a, b)
        | ZEBRASymbolicExpr::MulLo(a, b)
        | ZEBRASymbolicExpr::MulHiSS(a, b)
        | ZEBRASymbolicExpr::MulHiUU(a, b)
        | ZEBRASymbolicExpr::And(a, b)
        | ZEBRASymbolicExpr::Or(a, b)
        | ZEBRASymbolicExpr::Xor(a, b)
        | ZEBRASymbolicExpr::SRL(a, b)
        | ZEBRASymbolicExpr::SRLCarry(a, b)
        | ZEBRASymbolicExpr::Lt(a, b) => {
            gather_neighbors(a, neighbors);
            gather_neighbors(b, neighbors);
        }
        ZEBRASymbolicExpr::WordAddU(a, b)
        | ZEBRASymbolicExpr::WordSubU(a, b)
        | ZEBRASymbolicExpr::WordMul(a, b)
        | ZEBRASymbolicExpr::WordMulhu(a, b)
        | ZEBRASymbolicExpr::WordMulhs(a, b)
        | ZEBRASymbolicExpr::WordMultl(a, b)
        | ZEBRASymbolicExpr::WordMulth(a, b)
        | ZEBRASymbolicExpr::WordMultul(a, b)
        | ZEBRASymbolicExpr::WordMultuh(a, b)
        | ZEBRASymbolicExpr::WordDiv(a, b)
        | ZEBRASymbolicExpr::WordSDiv(a, b)
        | ZEBRASymbolicExpr::WordLt(a, b)
        | ZEBRASymbolicExpr::WordSLe(a, b)
        | ZEBRASymbolicExpr::WordSLt(a, b)
        | ZEBRASymbolicExpr::WordAnd(a, b)
        | ZEBRASymbolicExpr::WordOr(a, b)
        | ZEBRASymbolicExpr::WordXOr(a, b)
        | ZEBRASymbolicExpr::WordEq(a, b)
        | ZEBRASymbolicExpr::WordNEq(a, b)
        | ZEBRASymbolicExpr::WordSrl(a, b) => {
            for e in a.iter().chain(b.iter()) {
                gather_neighbors(e, neighbors);
            }
        }
    }
}

pub fn get_curr_i(expr: &ZEBRASymbolicExpr) -> Option<usize> {
    if let ZEBRASymbolicExpr::Variable(ZEBRASymbolicVal { entry, index }) = expr {
        if let ZEBRASymbolicEntry::Main { is_curr } = entry {
            if *is_curr {
                return Some(index.clone());
            }
        }
    }
    None
}

pub fn get_curr_i_sub_cur_j(expr: &ZEBRASymbolicExpr) -> Option<(usize, usize)> {
    if let ZEBRASymbolicExpr::Sub(lhs, rhs) = expr {
        if let Some(lhs_i) = get_curr_i(lhs) {
            if let Some(rhs_i) = get_curr_i(rhs) {
                return Some((lhs_i, rhs_i));
            }
        }
    }

    None
}

// collect_add_vars

pub fn get_curr_i_sub_const(expr: &ZEBRASymbolicExpr, _prime: u32) -> Option<(usize, i128)> {
    if let ZEBRASymbolicExpr::Sub(lhs, rhs) = expr {
        // Case: Variable - Constant
        if let Some(v_idx) = get_curr_i(lhs) {
            if let ZEBRASymbolicExpr::Constant(c) = &**rhs {
                if c.is_singleton() {
                    return Some((v_idx, c.lo));
                }
            }
        }
        // Case: Constant - Variable
        if let ZEBRASymbolicExpr::Constant(c) = &**lhs {
            if let Some(v_idx) = get_curr_i(rhs) {
                if c.is_singleton() {
                    return Some((v_idx, c.lo));
                }
            }
        }
    }
    None
}

pub fn get_curr_add_vars_sub_const(
    expr: &ZEBRASymbolicExpr,
    _prime: u32,
) -> Option<(Vec<usize>, i128)> {
    if let ZEBRASymbolicExpr::Sub(lhs, rhs) = expr {
        // Case: Addition of Variables - Constant
        if let Some(vs) = collect_add_vars(lhs) {
            if let ZEBRASymbolicExpr::Constant(c) = &**rhs {
                if c.is_singleton() {
                    return Some((vs.into_iter().collect::<Vec<_>>(), c.lo));
                }
            }
        }
        // Case: Constant - Addition of Variables
        if let ZEBRASymbolicExpr::Constant(c) = &**lhs {
            if let Some(vs) = collect_add_vars(rhs) {
                if c.is_singleton() {
                    return Some((vs.into_iter().collect::<Vec<_>>(), c.lo));
                }
            }
        }
    }
    None
}

/// Parses a selector expression into `(col_idx, is_inverted)`.
///
/// * `Var(s)` → `(s, false)` — condition fires when `curr[s] == 1`
/// * `Sub(Const(1), Var(s))` → `(s, true)` — condition fires when `curr[s] == 0`
pub fn get_curr_i_with_polarity(expr: &ZEBRASymbolicExpr) -> Option<(usize, bool)> {
    if let Some(idx) = get_curr_i(expr) {
        return Some((idx, false));
    }
    if let ZEBRASymbolicExpr::Sub(lhs, rhs) = expr {
        if let ZEBRASymbolicExpr::Constant(c) = lhs.as_ref() {
            if c.is_singleton() && c.lo == 1 {
                if let Some(idx) = get_curr_i(rhs) {
                    return Some((idx, true));
                }
            }
        }
    }
    None
}

/// Like [`get_curr_i_sub_const`] but also matches a bare current-row variable (`target = 0`).
pub fn get_curr_i_or_zero(expr: &ZEBRASymbolicExpr, prime: u32) -> Option<(usize, i128)> {
    if let Some(result) = get_curr_i_sub_const(expr, prime) {
        return Some(result);
    }
    if let Some(idx) = get_curr_i(expr) {
        return Some((idx, 0));
    }
    None
}

/// Like [`get_curr_add_vars_sub_const`] but also matches a bare addition of variables (`target = 0`).
pub fn get_curr_add_vars_or_zero(expr: &ZEBRASymbolicExpr, prime: u32) -> Option<(Vec<usize>, i128)> {
    if let Some(result) = get_curr_add_vars_sub_const(expr, prime) {
        return Some(result);
    }
    if let Some(vs) = collect_add_vars(expr) {
        return Some((vs.into_iter().collect(), 0));
    }
    None
}

pub fn preprocess_row(
    air_constraints: &Vec<ZEBRASymbolicExpr>,
    row: &mut Vec<AbstractInterval>,
    p: u32,
) {
    for c in air_constraints {
        if let ZEBRASymbolicExpr::Mul(lhs, rhs) = c {
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

/// Collects variable indices appearing in a purely additive expression tree.
///
/// This function traverses a symbolic expression and extracts the indices of
/// variables that participate in a sum composed only of additions and variables.
/// The result is returned as a set to eliminate duplicates.
///
/// # Supported Structure
///
/// The expression must be composed exclusively of:
///
/// * `Add(lhs, rhs)` nodes
/// * `Variable` leaf nodes
///
/// Any other construct (e.g., `Sub`, `Mul`, constants, predicates) causes the
/// function to return `None`.
///
/// # Returns
///
/// * `Some(HashSet<usize>)` — Set of variable indices in the additive expression
/// * `None` — Expression is not a pure sum of variables
///
/// # Example
///
/// For an expression equivalent to:
///
/// ```text
/// x_i + x_j + x_k
/// ```
///
/// the function returns:
///
/// ```text
/// {i, j, k}
/// ```
///
/// # Use Cases
///
/// * Detecting packed word constructions
/// * Recognizing linear combinations without coefficients
/// * Constraint pattern matching
///
/// # See Also
///
/// * [`collect_add_vars_vec`] — Same operation preserving order and duplicates
fn collect_add_vars(expr: &ZEBRASymbolicExpr) -> Option<HashSet<usize>> {
    match expr {
        ZEBRASymbolicExpr::Add(lhs, rhs) => {
            let mut left = collect_add_vars(lhs)?;
            let right = collect_add_vars(rhs)?;
            left.extend(right);
            Some(left)
        }
        ZEBRASymbolicExpr::Variable(ZEBRASymbolicVal { index, .. }) => {
            let mut set = HashSet::new();
            set.insert(index.clone());
            Some(set)
        }
        _ => None,
    }
}

/// Collects variable indices from a purely additive expression as an ordered list.
///
/// Similar to [`collect_add_vars`], but returns a vector preserving the traversal
/// order and allowing duplicate indices.
///
/// This is useful when the positional arrangement of variables is significant,
/// such as reconstructing multi-limb words or structured encodings.
///
/// # Supported Structure
///
/// Accepts only expressions composed of:
///
/// * `Add(lhs, rhs)`
/// * `Variable`
///
/// Any other expression form results in `None`.
///
/// # Returns
///
/// * `Some(Vec<usize>)` — Ordered list of variable indices
/// * `None` — Expression is not a pure sum of variables
///
/// # Example
///
/// For:
///
/// ```text
/// x_3 + x_5 + x_7
/// ```
///
/// returns:
///
/// ```text
/// [3, 5, 7]
/// ```
///
/// # Use Cases
///
/// * Word reconstruction from limbs
/// * Range-check pattern detection
/// * Symbolic encoding analysis
///
/// # See Also
///
/// * [`collect_add_vars`] — Set-based variant
fn collect_add_vars_vec(expr: &ZEBRASymbolicExpr) -> Option<Vec<usize>> {
    match expr {
        ZEBRASymbolicExpr::Add(lhs, rhs) => {
            let mut left = collect_add_vars_vec(lhs)?;
            let right = collect_add_vars_vec(rhs)?;
            left.extend(right);
            Some(left)
        }
        ZEBRASymbolicExpr::Variable(ZEBRASymbolicVal { index, .. }) => {
            let mut v = Vec::new();
            v.push(index.clone());
            Some(v)
        }
        _ => None,
    }
}

/// Detects a specialized encoding of an `is_zero` operator.
///
/// Recognizes a constraint pattern that encodes conditional behavior depending
/// on whether an expression evaluates to zero, and rewrites it into explicit
/// conditional symbolic expressions.
///
/// # Returns
///
/// * `Some(Vec<ZEBRASymbolicExpr>)` — Equivalent conditional constraints
/// * `None` — Pattern not recognized
///
/// # Use Cases
///
/// * De-sugaring arithmetic encodings
/// * Conditional reasoning
/// * Constraint normalization
pub fn is_iszero_operator(
    constraint: &ZEBRASymbolicExpr,
    prime: u32,
) -> Option<Vec<ZEBRASymbolicExpr>> {
    if let ZEBRASymbolicExpr::Mul(lhs, rhs) = constraint {
        if let ZEBRASymbolicExpr::Sub(r_lhs, r_rhs) = *rhs.clone() {
            if let ZEBRASymbolicExpr::Constant(c) = *r_rhs {
                if c.as_canonical_u32(prime) == 58079999 {
                    if let ZEBRASymbolicExpr::Add(x, y) = *r_lhs {
                        let cond_when_zero = ZEBRASymbolicExpr::WhenZero(
                            x.clone(),
                            Box::new(ZEBRASymbolicExpr::Sub(
                                y.clone(),
                                Box::new(ZEBRASymbolicExpr::Constant(AbstractInterval::one())),
                            )),
                        );
                        let cond_when_notzero = ZEBRASymbolicExpr::WhenNonZero(
                            x,
                            Box::new(ZEBRASymbolicExpr::Sub(
                                y,
                                Box::new(ZEBRASymbolicExpr::Constant(AbstractInterval::zero())),
                            )),
                        );

                        return Some(vec![
                            ZEBRASymbolicExpr::WhenNonZero(
                                lhs.clone(),
                                Box::new(cond_when_zero),
                            ),
                            ZEBRASymbolicExpr::WhenNonZero(
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

/// Detects KoalaBear word-range check constraints.
///
/// Identifies constraints encoding that a 4-limb word lies within the valid
/// KoalaBear field range and converts them into a semantic range predicate.
///
/// # Returns
///
/// * `Some(expr)` — Rewritten constraint using `KoalaBearRange`
/// * `None` — Pattern not recognized
///
/// # Use Cases
///
/// * Field-specific range validation
/// * Symbolic simplification
/// * Constraint abstraction
pub fn is_koalabear_word_range(
    constraint: &ZEBRASymbolicExpr,
    prime: u32,
) -> Option<ZEBRASymbolicExpr> {
    if let ZEBRASymbolicExpr::Mul(lhs, rhs) = constraint {
        if let ZEBRASymbolicExpr::Sub(r_lhs, r_rhs) = *rhs.clone() {
            if let ZEBRASymbolicExpr::Constant(c) = *r_rhs {
                if c.as_canonical_u32(prime) == 13227174 {
                    if let Some(word) = collect_add_vars_vec(&r_lhs) {
                        if word.len() == 4 {
                            let word_expr: Vec<_> = word
                                .iter()
                                .map(|&index| {
                                    ZEBRASymbolicExpr::Variable(ZEBRASymbolicVal {
                                        entry: ZEBRASymbolicEntry::Main { is_curr: true },
                                        index,
                                    })
                                })
                                .collect();
                            let cond = reconstruct_symbolic_word(&word_expr, 0);
                            return Some(ZEBRASymbolicExpr::WhenNonZero(
                                lhs.clone(),
                                Box::new(ZEBRASymbolicExpr::KoalaBearRange(Box::new(cond))),
                            ));
                        }
                    }
                }
            }
        }
    }

    None
}

/// Detects BabyBear word-range check constraints.
///
/// Similar to [`is_koalabear_word_range`], but for the BabyBear field.
/// Converts low-level arithmetic encodings into a semantic range predicate.
///
/// # Returns
///
/// * `Some(expr)` — Range predicate expression
/// * `None` — Pattern not matched
pub fn is_babybear_word_range(
    constraint: &ZEBRASymbolicExpr,
    prime: u32,
) -> Option<ZEBRASymbolicExpr> {
    if let ZEBRASymbolicExpr::Mul(lhs, rhs) = constraint {
        if let ZEBRASymbolicExpr::Sub(r_lhs, r_rhs) = *rhs.clone() {
            if let ZEBRASymbolicExpr::Constant(c) = *r_rhs {
                if c.as_canonical_u32(prime) == 785099 {
                    if let Some(word) = collect_add_vars_vec(&r_lhs) {
                        if word.len() == 4 {
                            let word_expr: Vec<_> = word
                                .iter()
                                .map(|&index| {
                                    ZEBRASymbolicExpr::Variable(ZEBRASymbolicVal {
                                        entry: ZEBRASymbolicEntry::Main { is_curr: true },
                                        index,
                                    })
                                })
                                .collect();
                            let cond = reconstruct_symbolic_word(&word_expr, 0);
                            return Some(ZEBRASymbolicExpr::WhenNonZero(
                                lhs.clone(),
                                Box::new(ZEBRASymbolicExpr::BabyBearRange(Box::new(cond))),
                            ));
                        }
                    }
                }
            }
        }
    }

    None
}

/// Detects Boolean constraints of the form `x * (x - 1) = 0`.
///
/// This polynomial identity holds exactly when `x ∈ {0, 1}`, enforcing that
/// the variable is Boolean.
///
/// # Returns
///
/// * `Some(index)` — Index of the Boolean variable
/// * `None` — Constraint is not a Boolean check
///
/// # Use Cases
///
/// * Identifying selector variables
/// * Range inference
/// * Domain restriction
pub fn is_boolean_constraint(constraint: &ZEBRASymbolicExpr) -> Option<usize> {
    use ZEBRASymbolicExpr::*;

    // helper: find `x - 1`
    fn is_x_minus_one(expr: &ZEBRASymbolicExpr) -> Option<usize> {
        if let Sub(lhs, rhs) = expr {
            if let (
                Variable(ZEBRASymbolicVal { index, .. }),
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
            (Variable(ZEBRASymbolicVal { index, .. }), rhs)
                if is_x_minus_one(rhs) == Some(index.clone()) =>
            {
                return Some(index.clone());
            }

            // (x - 1) * x
            (lhs, Variable(ZEBRASymbolicVal { index, .. }))
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
    constraints: &[ZEBRASymbolicExpr],
    multiplicities: &HashSet<usize>,
) -> Vec<usize> {
    let mut result = HashSet::new();
    for c in constraints {
        if let ZEBRASymbolicExpr::Mul(lhs, rhs) = c {
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

/// Creates a conditional implementation constraint based on an opcode.
///
/// If `opcode_var` equals the constant `opcode_val`, returns `expr` directly.
/// Otherwise, wraps it in a `WhenZero` condition:
///
/// ```text
/// WhenZero(opcode_var - opcode_val, expr)
/// ```
///
/// # Parameters
///
/// * `opcode_val` — Target opcode value
/// * `opcode_var` — Symbolic expression representing opcode
/// * `expr` — Constraint expression to conditionally apply
/// * `prime` — Field modulus
///
/// # Returns
///
/// * `Some(ZEBRASymbolicExpr)` — Conditional constraint
/// * `None` — Opcode constant mismatch
///
/// # Use Cases
///
/// * Conditional execution constraints
/// * Opcode-specific constraint enforcement
/// * Modular constraint generation in Lattice VM
pub fn make_impl_constraint(
    opcode_val: i128,
    opcode_var: &ZEBRASymbolicExpr,
    expr: ZEBRASymbolicExpr,
    prime: u32,
) -> Option<ZEBRASymbolicExpr> {
    if let ZEBRASymbolicExpr::Constant(c) = opcode_var {
        if c.as_canonical_u32(prime) as i128 == opcode_val {
            Some(expr)
        } else {
            None
        }
    } else {
        Some(ZEBRASymbolicExpr::WhenZero(
            Box::new(ZEBRASymbolicExpr::Sub(
                Box::new(opcode_var.clone()),
                Box::new(ZEBRASymbolicExpr::Constant(
                    AbstractInterval::from_i128(opcode_val),
                )),
            )),
            Box::new(expr),
        ))
    }
}

#[derive(Default, Debug, Clone)]
pub struct GeneralLookupInfo {
    pub op_a: Vec<usize>,
    pub op_b: Vec<usize>,
    pub op_c: Vec<usize>,
    pub is_real: Vec<ZEBRASymbolicExpr>,
}
