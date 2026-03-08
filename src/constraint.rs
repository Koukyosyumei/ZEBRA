use std::collections::HashSet;

use crate::interval::{AbstractInterval, MayBeFlag};
use crate::symbolic::{
    gather_vars, LatticeVMSymbolicEntry, LatticeVMSymbolicExpr, LatticeVMSymbolicVal,
};
use crate::trace::AbstractTrace;

/// Evaluates a set of symbolic constraints over a trace.
///
/// Traverses each row and evaluates constraints in either strict or relaxed mode,
/// recording potential refinements and collecting uncertain variable positions.
///
/// # Parameters
///
/// * `trace` — Abstract trace to evaluate
/// * `public_vals` — Optional array of public input intervals
/// * `constraints` — List of symbolic expressions to evaluate
/// * `prime` — Field modulus for arithmetic operations
/// * `is_strict` — Whether to use strict zero evaluation
/// * `potential` — Counter of non-conclusive evaluations (incremented)
/// * `memo` — Set to record uncertain variable positions
///
/// # Returns
///
/// * `MayBeFlag::True` — All constraints strictly satisfied
/// * `MayBeFlag::False` — At least one constraint violated
/// * `MayBeFlag::MayBe` — Some constraints inconclusive
///
/// # Use Cases
///
/// * Constraint propagation
/// * Symbolic execution
/// * Interval refinement in AIR constraints
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
        let mut _j = 0;
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
                    //println!("({}, {}): {}, {} ^^{}", i, _j, tc, prime, is_strict);
                    return MayBeFlag::False;
                }
                MayBeFlag::MayBe => {
                    // println!("({}, {}): {}, {} ^^{}", i, _j, tc, prime, is_strict);
                    gather_vars(i, tc, memo);
                    is_all_true = false;
                    *potential += 1;
                }
            }
            _j += 1;
        }
    }
    if is_all_true {
        MayBeFlag::True
    } else {
        MayBeFlag::MayBe
    }
}

/// Encapsulates all symbolic constraints for a Lattice VM.
///
/// Divides constraints into categories:
///
/// * `air_constraints` — Core trace constraints (analogous to AIR)
/// * `lookup_constraints` — Table/lookup constraints
/// * `pv_pos_constraints` — Public-positive constraints
/// * `pv_neg_constraints` — Public-negative constraints
/// * `blocking_constraints` — Row-specific blocking constraints
#[derive(Clone)]
pub struct LatticeVMConstraints {
    pub air_constraints: Vec<LatticeVMSymbolicExpr>,
    pub lookup_constraints: Vec<LatticeVMSymbolicExpr>,
    pub pv_pos_constraints: Vec<LatticeVMSymbolicExpr>,
    pub pv_neg_constraints: Vec<LatticeVMSymbolicExpr>,
    pub blocking_constraints: Vec<(usize, LatticeVMSymbolicExpr)>,
}

impl LatticeVMConstraints {
    /// Constructs a `LatticeVMConstraints` object with specified AIR and lookup constraints.
    ///
    /// Other constraint vectors are initialized empty.
    ///
    /// # Parameters
    ///
    /// * `air_constraints` — Vector of AIR constraints
    /// * `lookup_constraints` — Vector of lookup constraints
    ///
    /// # Returns
    ///
    /// Initialized `LatticeVMConstraints` object.
    ///
    /// # Use Cases
    ///
    /// * Constraint collection before evaluation
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

/// Evaluates all Lattice VM constraints over a trace.
///
/// Processes blocking, AIR, lookup, and public constraints, returning an overall
/// `MayBeFlag`, a potential counter, and the memo of uncertain variables.
///
/// # Parameters
///
/// * `trace` — Abstract execution trace
/// * `public_vals` — Optional public input intervals
/// * `constraints` — LatticeVMConstraints object
/// * `prime` — Field modulus
///
/// # Returns
///
/// `(MayBeFlag, i32, HashSet<(usize, usize)>)`
///
/// * `MayBeFlag` — True, False, or MayBe depending on constraint satisfaction
/// * `i32` — Potential counter (number of inconclusive evaluations)
/// * `HashSet<(usize, usize)>` — Positions of variables contributing to MayBe
///
/// # Use Cases
///
/// * Comprehensive constraint evaluation
/// * Solver pruning
/// * Trace analysis and verification
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
