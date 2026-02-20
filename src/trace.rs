use std::fmt;
use std::hash::Hash;

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::Rng;

use crate::interval::AbstractInterval;

/// Represents an abstract execution trace as a table of symbolic intervals.
///
/// Each row corresponds to a timestep, and each column represents a variable or
/// memory cell. Intervals may be singletons (concrete values) or ranges.
///
/// Additionally tracks positions of singleton values for quick access.
///
/// # Fields
///
/// * `data` — `Vec<Vec<AbstractInterval>>` representing the trace table
/// * `singleton_positions` — List of `(row, col)` indices where intervals are singletons
///
/// # Use Cases
///
/// * Constraint evaluation over abstract traces
/// * Interval refinement
/// * Symbolic execution and analysis
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
    /// Constructs a new `AbstractTrace` from raw interval data.
    ///
    /// Automatically identifies all singleton positions in the trace for faster
    /// refinement and constraint evaluation.
    ///
    /// # Parameters
    ///
    /// * `raw_trace` — 2D vector of `AbstractInterval` representing initial trace
    ///
    /// # Returns
    ///
    /// A fully initialized `AbstractTrace` with `singleton_positions` populated.
    ///
    /// # Use Cases
    ///
    /// * Initial trace creation
    /// * Preprocessing before constraint evaluation
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

    /// Compares two abstract traces and returns positions of differing intervals.
    ///
    /// # Parameters
    ///
    /// * `other` — Another `AbstractTrace` to compare with
    ///
    /// # Returns
    ///
    /// A `Vec<(usize, usize)>` listing `(row, column)` positions where intervals differ.
    ///
    /// # Use Cases
    ///
    /// * Detecting updates from refinement
    /// * Debugging changes in interval propagation
    /// * Differential testing of symbolic traces
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

pub fn trace_fmt_with_idxs(trace: &AbstractTrace, i: usize, idxs: &[usize]) -> String {
    idxs.iter()
        .map(|&j| trace.data[i][j].to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Refines an abstract trace by splitting intervals on target indices.
///
/// Randomly selects a row in the specified range, chooses a non-singleton variable
/// from `refinement_target_indices`, and splits it to generate candidate traces.
///
/// # Parameters
///
/// * `trace` — Trace to refine
/// * `refinment_target_indicies` — Indices of columns to refine
/// * `min_row_id` — Minimum row index for refinement
/// * `max_row_id` — Maximum row index for refinement
/// * `prime` — Field modulus
/// * `rng` — Random number generator
///
/// # Returns
///
/// `(Option<Vec<AbstractTrace>>, bool)`
///
/// * `Some(candidates)` — Candidate traces generated from splitting
/// * `None` — Nothing to refine
/// * `bool` — True if a split actually occurred, False otherwise
///
/// # Use Cases
///
/// * Search-based trace refinement
/// * Interval branching in solvers
/// * Randomized symbolic execution
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
