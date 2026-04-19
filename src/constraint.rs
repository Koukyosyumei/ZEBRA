use std::collections::HashSet;

use crate::interval::{AbstractInterval, MayBeFlag};
use crate::symbolic::{
    count_arith_ops_cse, gather_cols, gather_neighbors, gather_vars, ZEBRASymbolicEntry,
    ZEBRASymbolicExpr, ZEBRASymbolicVal,
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
    constraints: &[ZEBRASymbolicExpr],
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
                    //println!("({}, {}): {}, {} ^^{}", i, _j, tc, prime, is_strict);
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
pub struct ZEBRAConstraints {
    pub air_constraints: Vec<ZEBRASymbolicExpr>,
    pub lookup_constraints: Vec<ZEBRASymbolicExpr>,
    pub pv_pos_constraints: Vec<ZEBRASymbolicExpr>,
    pub pv_neg_constraints: Vec<ZEBRASymbolicExpr>,
    pub blocking_constraints: Vec<(usize, ZEBRASymbolicExpr)>,
}

impl ZEBRAConstraints {
    /// Constructs a `ZEBRAConstraints` object with specified AIR and lookup constraints.
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
    /// Initialized `ZEBRAConstraints` object.
    ///
    /// # Use Cases
    ///
    /// * Constraint collection before evaluation
    pub fn new(
        air_constraints: Vec<ZEBRASymbolicExpr>,
        lookup_constraints: Vec<ZEBRASymbolicExpr>,
    ) -> Self {
        ZEBRAConstraints {
            air_constraints,
            lookup_constraints,
            pv_pos_constraints: vec![],
            pv_neg_constraints: vec![],
            blocking_constraints: vec![],
        }
    }
}

/// Sparsity statistics for a [`ZEBRAConstraints`] set relative to a known table width.
///
/// "Sparsity" here means how little of the available column space the constraints
/// actually touch.  A coverage of 0.0 means no column is constrained; 1.0 means
/// every column appears in at least one constraint.
///
/// # Fields
///
/// * `num_total_columns` — Total columns in the table (supplied by the caller).
/// * `air_covered_columns` / `lookup_covered_columns` / `pv_covered_columns`
///   — Distinct columns touched by each category.
/// * `total_covered_columns` — Union across all categories.
/// * `*_column_coverage` — `covered / total` ratio for each category (0.0–1.0).
/// * `num_*_constraints` — Constraint counts per category.
/// * `avg_cols_per_*_constraint` — Mean width (column footprint) per constraint.
/// * `column_hit_counts` — For each column index, how many constraints reference it.
#[derive(Debug)]
pub struct ConstraintSparsityReport {
    pub num_total_columns: usize,
    // ---- per-category coverage ----
    pub air_covered_columns: usize,
    pub lookup_covered_columns: usize,
    pub pv_covered_columns: usize,
    pub total_covered_columns: usize,
    // ---- ratios ----
    pub air_column_coverage: f64,
    pub lookup_column_coverage: f64,
    pub pv_column_coverage: f64,
    pub total_column_coverage: f64,
    // ---- counts ----
    pub num_air_constraints: usize,
    pub num_lookup_constraints: usize,
    pub num_pv_constraints: usize,
    // ---- per-constraint width ----
    pub avg_cols_per_air_constraint: f64,
    pub avg_cols_per_lookup_constraint: f64,
    pub avg_cols_per_pv_constraint: f64,
    /// `column_hit_counts[i]` = number of constraints that reference column `i`.
    /// Length equals `num_total_columns`; unconstrained columns have value 0.
    pub column_hit_counts: Vec<usize>,
    // ---- AIR-sparsity metric (IACR 2018/046) ----
    /// |N|: distinct (entry, row-context, column-index) cells across ALL constraints
    /// (AIR + lookup + PV + blocking), reflecting the full table width.
    pub neighborhood_size: usize,
    /// T_arith: total arithmetic operation nodes across AIR constraints.
    pub t_arith: usize,
    /// T_arith / (s × |N|) — ops per variable slot relative to a fully-dense linear baseline.
    /// Lower means sparser. 0.0 when s or |N| is zero.
    pub air_sparsity: f64,
}

impl std::fmt::Display for ConstraintSparsityReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Constraint Sparsity Report ===")?;
        writeln!(f, "Total columns : {}", self.num_total_columns)?;
        writeln!(
            f,
            "Covered columns: {} / {} ({:.1}%)",
            self.total_covered_columns,
            self.num_total_columns,
            self.total_column_coverage * 100.0,
        )?;
        writeln!(
            f,
            "  AIR    : {:4} cols covered ({:.1}%)  |  {} constraints, {:.2} cols/constraint avg",
            self.air_covered_columns,
            self.air_column_coverage * 100.0,
            self.num_air_constraints,
            self.avg_cols_per_air_constraint,
        )?;
        writeln!(
            f,
            "  Lookup : {:4} cols covered ({:.1}%)  |  {} constraints, {:.2} cols/constraint avg",
            self.lookup_covered_columns,
            self.lookup_column_coverage * 100.0,
            self.num_lookup_constraints,
            self.avg_cols_per_lookup_constraint,
        )?;
        writeln!(
            f,
            "  PV     : {:4} cols covered ({:.1}%)  |  {} constraints, {:.2} cols/constraint avg",
            self.pv_covered_columns,
            self.pv_column_coverage * 100.0,
            self.num_pv_constraints,
            self.avg_cols_per_pv_constraint,
        )?;
        writeln!(
            f,
            "AIR sparsity  : {:.6}  (T_arith={}, |N|={}, s={})",
            self.air_sparsity,
            self.t_arith,
            self.neighborhood_size,
            self.num_air_constraints,
        )?;
        // Show the top-5 hottest columns (most constraints)
        let mut indexed: Vec<(usize, usize)> = self
            .column_hit_counts
            .iter()
            .enumerate()
            .map(|(i, &c)| (i, c))
            .collect();
        indexed.sort_by(|a, b| b.1.cmp(&a.1));
        let top = indexed.iter().take(5).collect::<Vec<_>>();
        write!(f, "Top-5 hottest columns (col_idx: hit_count):")?;
        for (col, hits) in top {
            write!(f, "  [{}]={}", col, hits)?;
        }
        Ok(())
    }
}

impl ZEBRAConstraints {
    /// Compute sparsity metrics for this constraint set.
    ///
    /// # Parameters
    ///
    /// * `num_total_columns` — Number of columns in the table being analysed.
    ///   Pass `ConstraintInfo::num_total_columns` when available.
    ///
    /// # Returns
    ///
    /// A [`ConstraintSparsityReport`] with coverage ratios, per-constraint
    /// widths, and per-column hit counts.
    pub fn sparsity_report(&self, num_total_columns: usize) -> ConstraintSparsityReport {
        // Helper: gather column indices touched by a slice of constraints.
        fn covered(constraints: &[ZEBRASymbolicExpr]) -> HashSet<usize> {
            let mut cols = HashSet::new();
            for c in constraints {
                gather_cols(c, &mut cols);
            }
            cols
        }

        // Helper: average columns per constraint (0.0 when empty).
        fn avg_width(constraints: &[ZEBRASymbolicExpr]) -> f64 {
            if constraints.is_empty() {
                return 0.0;
            }
            let total: usize = constraints
                .iter()
                .map(|c| {
                    let mut cols = HashSet::new();
                    gather_cols(c, &mut cols);
                    cols.len()
                })
                .sum();
            total as f64 / constraints.len() as f64
        }

        let pv: Vec<&ZEBRASymbolicExpr> = self
            .pv_pos_constraints
            .iter()
            .chain(self.pv_neg_constraints.iter())
            .collect();

        let air_cols = covered(&self.air_constraints);
        let lookup_cols = covered(&self.lookup_constraints);
        let pv_cols: HashSet<usize> = {
            let mut s = HashSet::new();
            for c in &pv {
                gather_cols(c, &mut s);
            }
            s
        };

        let mut total_cols = air_cols.clone();
        total_cols.extend(lookup_cols.iter());
        total_cols.extend(pv_cols.iter());

        let ratio = |n: usize| -> f64 {
            if num_total_columns == 0 {
                0.0
            } else {
                n as f64 / num_total_columns as f64
            }
        };

        // Per-column hit counts: count every constraint that mentions each column.
        let mut column_hit_counts = vec![0usize; num_total_columns];
        let all_constraints: Vec<&ZEBRASymbolicExpr> = self
            .air_constraints
            .iter()
            .chain(self.lookup_constraints.iter())
            .chain(self.pv_pos_constraints.iter())
            .chain(self.pv_neg_constraints.iter())
            .chain(self.blocking_constraints.iter().map(|(_, c)| c))
            .collect();
        for c in &all_constraints {
            let mut cols = HashSet::new();
            gather_cols(c, &mut cols);
            for col in cols {
                if col < num_total_columns {
                    column_hit_counts[col] += 1;
                }
            }
        }

        let num_pv = self.pv_pos_constraints.len() + self.pv_neg_constraints.len();
        let avg_pv = if num_pv == 0 {
            0.0
        } else {
            let total: usize = pv
                .iter()
                .map(|c| {
                    let mut cols = HashSet::new();
                    gather_cols(c, &mut cols);
                    cols.len()
                })
                .sum();
            total as f64 / num_pv as f64
        };

        // AIR-sparsity metric: T_arith / (s × |N|)
        // |N| is gathered from ALL constraints so it reflects the true table width
        // (lookup columns are part of the neighborhood even though they don't
        // contribute arithmetic ops).  T_arith and s are AIR-only.
        let air_only: Vec<&ZEBRASymbolicExpr> = self
            .air_constraints
            .iter()
            .chain(self.pv_pos_constraints.iter())
            .chain(self.pv_neg_constraints.iter())
            .chain(self.blocking_constraints.iter().map(|(_, c)| c))
            .collect();
        let all_constraints: Vec<&ZEBRASymbolicExpr> = air_only
            .iter()
            .copied()
            .chain(self.lookup_constraints.iter())
            .collect();
        let mut neighbors = HashSet::new();
        for c in &all_constraints {
            gather_neighbors(c, &mut neighbors);
        }
        let neighborhood_size = neighbors.len();
        // s = number of AIR+PV+blocking constraints (arithmetic constraints only).
        let s = air_only.len();
        let t_arith = count_arith_ops_cse(&air_only);
        let air_sparsity = if s == 0 || neighborhood_size == 0 {
            0.0
        } else {
            t_arith as f64 / (s * neighborhood_size) as f64
        };

        ConstraintSparsityReport {
            num_total_columns,
            air_covered_columns: air_cols.len(),
            lookup_covered_columns: lookup_cols.len(),
            pv_covered_columns: pv_cols.len(),
            total_covered_columns: total_cols.len(),
            air_column_coverage: ratio(air_cols.len()),
            lookup_column_coverage: ratio(lookup_cols.len()),
            pv_column_coverage: ratio(pv_cols.len()),
            total_column_coverage: ratio(total_cols.len()),
            num_air_constraints: self.air_constraints.len(),
            num_lookup_constraints: self.lookup_constraints.len(),
            num_pv_constraints: num_pv,
            avg_cols_per_air_constraint: avg_width(&self.air_constraints),
            avg_cols_per_lookup_constraint: avg_width(&self.lookup_constraints),
            avg_cols_per_pv_constraint: avg_pv,
            column_hit_counts,
            neighborhood_size,
            t_arith,
            air_sparsity,
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
/// * `constraints` — ZEBRAConstraints object
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
    constraints: &ZEBRAConstraints,
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
    constraints: &mut ZEBRAConstraints,
    base_abs_main_trace_data: &Vec<Vec<AbstractInterval>>,
    i: usize,
) {
    for j in output_columns {
        constraints.blocking_constraints.push((
            i,
            ZEBRASymbolicExpr::Sub(
                Box::new(ZEBRASymbolicExpr::Variable(ZEBRASymbolicVal {
                    entry: ZEBRASymbolicEntry::Main { is_curr: true },
                    index: *j,
                })),
                Box::new(ZEBRASymbolicExpr::Constant(
                    base_abs_main_trace_data[i][*j].clone(),
                )),
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interval::AbstractInterval;
    use crate::symbolic::{ZEBRASymbolicEntry, ZEBRASymbolicExpr, ZEBRASymbolicVal};

    fn make_var(col: usize) -> ZEBRASymbolicExpr {
        ZEBRASymbolicExpr::Variable(ZEBRASymbolicVal {
            entry: ZEBRASymbolicEntry::Main { is_curr: true },
            index: col,
        })
    }

    fn make_const(v: i128) -> ZEBRASymbolicExpr {
        ZEBRASymbolicExpr::Constant(AbstractInterval { lo: v, hi: v })
    }

    #[test]
    fn test_sparsity_report_basic() {
        // Three AIR constraints touching columns 0, 1, 2 out of 5 total.
        //   c0: col[0] - const
        //   c1: col[1] + col[2]
        //   c2: WhenZero(col[0], col[1])  -- exercises the previously-missing branch
        let c0 = ZEBRASymbolicExpr::Sub(Box::new(make_var(0)), Box::new(make_const(1)));
        let c1 = ZEBRASymbolicExpr::Add(Box::new(make_var(1)), Box::new(make_var(2)));
        let c2 = ZEBRASymbolicExpr::WhenZero(Box::new(make_var(0)), Box::new(make_var(1)));

        let constraints = ZEBRAConstraints::new(vec![c0, c1, c2], vec![]);
        let report = constraints.sparsity_report(5);

        assert_eq!(report.num_total_columns, 5);
        assert_eq!(report.air_covered_columns, 3); // cols 0, 1, 2
        assert_eq!(report.total_covered_columns, 3);
        assert!((report.air_column_coverage - 0.6).abs() < 1e-9);
        assert!((report.total_column_coverage - 0.6).abs() < 1e-9);

        // column hit counts: col0 in c0+c2=2, col1 in c1+c2=2, col2 in c1=1, rest 0
        assert_eq!(report.column_hit_counts[0], 2);
        assert_eq!(report.column_hit_counts[1], 2);
        assert_eq!(report.column_hit_counts[2], 1);
        assert_eq!(report.column_hit_counts[3], 0);
        assert_eq!(report.column_hit_counts[4], 0);

        // avg cols per AIR constraint: (1 + 2 + 2) / 3 = 5/3
        let expected_avg = 5.0 / 3.0;
        assert!((report.avg_cols_per_air_constraint - expected_avg).abs() < 1e-9);
    }

    #[test]
    fn test_sparsity_report_fully_constrained() {
        let constraints = ZEBRAConstraints::new(
            (0..4).map(|i| make_var(i)).collect(),
            vec![],
        );
        let report = constraints.sparsity_report(4);
        assert_eq!(report.total_covered_columns, 4);
        assert!((report.total_column_coverage - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_sparsity_report_empty() {
        let constraints = ZEBRAConstraints::new(vec![], vec![]);
        let report = constraints.sparsity_report(10);
        assert_eq!(report.total_covered_columns, 0);
        assert!((report.total_column_coverage - 0.0).abs() < 1e-9);
        assert_eq!(report.avg_cols_per_air_constraint, 0.0);
        assert_eq!(report.neighborhood_size, 0);
        assert_eq!(report.t_arith, 0);
        assert!((report.air_sparsity - 0.0).abs() < 1e-9);
    }

    fn make_next_var(col: usize) -> ZEBRASymbolicExpr {
        ZEBRASymbolicExpr::Variable(ZEBRASymbolicVal {
            entry: ZEBRASymbolicEntry::Main { is_curr: false },
            index: col,
        })
    }

    #[test]
    fn test_air_sparsity_basic() {
        // c0 = Sub(var_curr[0], const)            -> 1 op, neighbors: {curr[0]}
        // c1 = Add(var_curr[1], var_curr[2])      -> 1 op, neighbors: {curr[1], curr[2]}
        // c2 = WhenZero(var_curr[0], var_curr[1]) -> 1 op, neighbors: {curr[0], curr[1]}
        // t_arith=3, s=3, |N|=3, air_sparsity = 3 / (3×3) = 1/3
        let c0 = ZEBRASymbolicExpr::Sub(Box::new(make_var(0)), Box::new(make_const(1)));
        let c1 = ZEBRASymbolicExpr::Add(Box::new(make_var(1)), Box::new(make_var(2)));
        let c2 = ZEBRASymbolicExpr::WhenZero(Box::new(make_var(0)), Box::new(make_var(1)));
        let constraints = ZEBRAConstraints::new(vec![c0, c1, c2], vec![]);
        let report = constraints.sparsity_report(5);

        assert_eq!(report.t_arith, 3);
        assert_eq!(report.neighborhood_size, 3);
        assert_eq!(report.num_air_constraints, 3);
        assert!((report.air_sparsity - 3.0 / (3.0 * 3.0)).abs() < 1e-9);
    }

    #[test]
    fn test_air_sparsity_curr_next_distinct() {
        // curr[0] and next[0] are different neighbors even though they share column 0.
        // c0 = Mul(curr[0], next[0]) -> 1 op, neighbors: {curr[0], next[0]}
        // t_arith=1, s=1, |N|=2, air_sparsity = 1 / (1×2) = 0.5
        let c0 = ZEBRASymbolicExpr::Mul(Box::new(make_var(0)), Box::new(make_next_var(0)));
        let constraints = ZEBRAConstraints::new(vec![c0], vec![]);
        let report = constraints.sparsity_report(2);

        assert_eq!(report.neighborhood_size, 2);
        assert_eq!(report.t_arith, 1);
        assert_eq!(report.num_air_constraints, 1);
        assert!((report.air_sparsity - 1.0 / (1.0 * 2.0)).abs() < 1e-9);
    }

    #[test]
    fn test_air_sparsity_cse() {
        // P1 = Mul(Add(var[0], var[1]), var[2])  -> 2 ops: Add + Mul
        // P2 = Mul(Add(var[0], var[1]), var[3])  -> shared Add counted once: 1 new op (Mul)
        // Without CSE: t_arith=4; with CSE: t_arith=3
        // s=2, |N|=4, air_sparsity = 3 / (2×4) = 0.375
        let shared = ZEBRASymbolicExpr::Add(Box::new(make_var(0)), Box::new(make_var(1)));
        let c0 = ZEBRASymbolicExpr::Mul(Box::new(shared.clone()), Box::new(make_var(2)));
        let c1 = ZEBRASymbolicExpr::Mul(Box::new(shared), Box::new(make_var(3)));
        let constraints = ZEBRAConstraints::new(vec![c0, c1], vec![]);
        let report = constraints.sparsity_report(4);

        assert_eq!(report.t_arith, 3);
        assert_eq!(report.neighborhood_size, 4);
        assert_eq!(report.num_air_constraints, 2);
        assert!((report.air_sparsity - 3.0 / (2.0 * 4.0)).abs() < 1e-9);
    }
}
