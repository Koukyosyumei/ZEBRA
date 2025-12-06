use std::collections::{HashMap, HashSet};
use std::i32;
use std::io::Stdout;
use std::time::Duration;

use crossterm::event::KeyModifiers;
use crossterm::event::{self, Event, KeyCode};
use itertools::Itertools;
use priority_queue::PriorityQueue;
use rand::seq::SliceRandom;
use rand::{rngs::StdRng, SeedableRng};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::{
    interval::{AbstractInterval, MayBeFlag},
    symbolic::{eval_constraints, refine_trace, AbstractTrace, LatticeVMConstraints},
    ui::UiState,
};

/// Result of constraint checking on a node.
pub type Potential = (i32, i32);

/// Node inside the search queue.
/// depth: number of refinement steps so far.
#[derive(Eq, Hash, PartialEq, Clone, Debug)]
pub struct SearchNode {
    main_trace: AbstractTrace,
    public_trace: AbstractTrace,
    depth: usize,
}

/// Performs a prioritized, iterative refinement search over abstract execution traces.
///
/// This function implements the main refinement loop for symbolic execution or
/// abstract interpretation. It repeatedly selects the most promising trace from
/// the priority queue, applies program semantics constraints via `align_pc_to_program`,
/// and evaluates the trace against user-defined constraints. Based on the evaluation:
/// - If the trace satisfies all constraints (`MayBeFlag::True`), it is returned as a solution.
/// - If the trace violates constraints (`MayBeFlag::False`), it is discarded and counted as an unsatisfied trial.
/// - If the trace is partially satisfying (`MayBeFlag::MayBe`), it is further refined and pushed back into the queue.
///
/// During execution, the function optionally updates a UI showing progress, trial counts,
/// queue status, and potentials.
///
/// # Type Parameters
/// * `AlignPcToProgramFn` - A closure or function that synchronizes the PC column with the program
///   instructions in an abstract trace. Signature: `Fn(&mut AbstractTrace, u32)`.
///
/// # Arguments
/// * `queue` - A priority queue containing candidate search nodes `(main_trace, public_trace, depth)`
///   prioritized by potential and depth.
/// * `num_trial` - Mutable reference to the cumulative number of trials performed.
/// * `constraints` - The `LatticeVMConstraints` against which traces are evaluated.
/// * `_num_refined_points` - Placeholder for number of points to refine (currently unused).
/// * `base_refinment_target_indicies_main` - Indices of main trace columns to target for refinement.
/// * `refinment_target_indicies_pv` - Indices of public-value columns to target for refinement.
/// * `min_row_id` / `max_row_id` - Row bounds within which refinement occurs.
/// * `align_pc_to_program` - Function or closure that updates opcode and operand intervals based on the PC column.
/// * `max_expansions` - Maximum number of iterations / expansions allowed.
/// * `global_expansion_count` - Number of expansions already performed globally (for UI tracking / logging).
/// * `prime` - Field prime for interval arithmetic and canonical conversion.
/// * `rng` - Random number generator for stochastic refinement ordering.
/// * `_solver_context_info` - Optional context string for logging (currently unused).
/// * `ui` - Mutable reference to the UI state to update progress.
/// * `terminal` - Terminal backend used for drawing the UI.
/// * `should_update_ui` - Whether to refresh the UI after each iteration.
///
/// # Returns
/// Tuple `(solution, num_trial, cumulative_potential, exit_flag, final_memo)`:
/// * `solution` - `Some(AbstractTrace)` if a trace satisfying all constraints is found; otherwise `None`.
/// * `num_trial` - Total number of trials performed during this function call.
/// * `cumulative_potential` - Sum of evaluated potentials over all explored nodes.
/// * `exit_flag` - `true` if the user requested an early exit via keyboard input; otherwise `false`.
/// * `final_memo` - A set of `(row_index, col_index)` pairs that were refined during the last evaluation.
pub fn solve<AlignPcToProgramFn>(
    queue: &mut PriorityQueue<SearchNode, Potential>,
    num_trial: &mut usize,
    constraints: &LatticeVMConstraints,
    _num_refined_points: usize,
    base_refinment_target_indicies_main: &Vec<usize>,
    refinment_target_indicies_pv: &Vec<usize>,
    min_row_id: usize,
    max_row_id: usize,
    align_pc_to_program: &AlignPcToProgramFn,
    max_expansions: usize,
    global_expansion_count: usize,
    prime: u32,
    rng: &mut StdRng,
    _solver_context_info: &str,
    ui: &mut UiState,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    should_update_ui: bool,
) -> (
    Option<AbstractTrace>,
    usize,
    i32,
    bool,
    HashSet<(usize, usize)>,
)
where
    AlignPcToProgramFn: Fn(&mut AbstractTrace, u32),
{
    let mut num_unsatisfied_trial = 0;
    let mut cumulative_priority = 0;
    let refinment_target_indicies_main = base_refinment_target_indicies_main.clone();
    let mut final_memo = HashSet::new();

    while !queue.is_empty() && *num_trial < max_expansions {
        *num_trial += &1;

        // -----------------------------
        // POP best candidate from queue
        // -----------------------------
        let (head, potential) = queue.pop().unwrap();
        let main_trace = head.main_trace;
        let public_trace = head.public_trace;

        // Update UI status string if requested
        if should_update_ui {
            ui.status = format!(
                    "Target Columns: {:?}\n #Total Trial: {}\n #Trial {}\n #UNSAT Trial: {}\n #Qued: {}\n Potential: {}\n Sum-Potential: {}",
                    refinment_target_indicies_main,
                    *num_trial + global_expansion_count,
                    num_trial,
                    num_unsatisfied_trial,
                    queue.len(),
                    -potential.0,
                    cumulative_priority,
                );
        }

        // Render the UI (non-fatal unwrap for brevity)
        terminal
            .draw(|f| {
                ui.render::<CrosstermBackend<Stdout>>(f);
            })
            .unwrap();

        // accumulate potential (skip the first trial)
        if *num_trial > 1 {
            cumulative_priority += potential.1;
        }

        // -----------------------------
        // CHILDREN GENERATION
        // Steps:
        //  1) refine main_trace columns
        //  2) refine public_trace columns
        //  3) cartesian combine where needed
        // -----------------------------
        let refined_main_candidates = refine_trace(
            &main_trace,
            &refinment_target_indicies_main,
            min_row_id,
            max_row_id,
            prime,
            rng,
        );

        let refined_public_candidates = refine_trace(
            &public_trace,
            refinment_target_indicies_pv,
            min_row_id,
            max_row_id,
            prime,
            rng,
        );

        // produce children as pairs (main_candidate, public_candidate)
        let mut children = vec![];
        match (refined_main_candidates, refined_public_candidates) {
            (Some(main_cands), Some(pub_cands)) => {
                // combine every pair
                for pub_c in pub_cands {
                    for main_c in &main_cands {
                        children.push((main_c.clone(), pub_c.clone()));
                    }
                }
            }
            (Some(main_cands), None) => {
                for main_c in main_cands {
                    children.push((main_c.clone(), public_trace.clone()));
                }
            }
            (None, Some(pub_cands)) => {
                for pub_c in pub_cands {
                    children.push((main_trace.clone(), pub_c.clone()));
                }
            }
            (None, None) => {
                // No refinements produced → continue to next queue entry
                continue;
            }
        }

        // randomize exploration order
        children.shuffle(rng);

        // -----------------------------
        // Evaluate each child:
        //  - align PC to program semantics
        //  - eval constraints (True/False/MayBe)
        //  - handle each case accordingly
        // -----------------------------
        for kid in &mut children {
            // Align opcode / operands according to PC column before evaluation.
            align_pc_to_program(&mut kid.0, prime);

            // Evaluate constraints on the child trace.
            // The `eval_constraints` function returns:
            //  (MayBeFlag::True)  => full solution
            //  (MayBeFlag::False) => unsatisfiable -> increment unsat counter
            //  (MayBeFlag::MayBe) => push back to queue with priority
            let (constraint_result, potential, memo) =
                eval_constraints(&kid.0, Some(&kid.1.data[0]), constraints, prime);
            final_memo = memo;
            match constraint_result {
                MayBeFlag::True => {
                    // solution found — return immediately
                    return (
                        Some(kid.0.clone()),
                        *num_trial,
                        cumulative_priority,
                        false,
                        final_memo,
                    );
                }
                MayBeFlag::False => {
                    // prune: track unsatisfied counts for diagnostics
                    num_unsatisfied_trial += 1;
                }
                MayBeFlag::MayBe => {
                    // push the ambiguous candidate back to the priority queue
                    queue.push(
                        SearchNode {
                            main_trace: kid.0.clone(),
                            public_trace: kid.1.clone(),
                            depth: head.depth + 1,
                        },
                        (-potential, (head.depth as i32)),
                    );
                }
            }
        }

        // -----------------------------
        // Handle non-blocking keyboard events (quit / scroll)
        // -----------------------------
        if event::poll(Duration::from_millis(1)).unwrap() {
            if let Event::Key(key) = event::read().unwrap() {
                // Ctrl-C or 'q' quits the solver early (return exit flag)
                if key.code == KeyCode::Char('q')
                    || (key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL))
                {
                    return (None, *num_trial, cumulative_priority, true, final_memo);
                }

                // Up / Down modify UI scroll state
                if key.code == KeyCode::Down {
                    ui.scroll_recovered = ui.scroll_logs.saturating_add(1);
                } else if key.code == KeyCode::Up {
                    ui.scroll_recovered = ui.scroll_logs.saturating_sub(1);
                }
            }
        }
    }

    // Exhausted queue or max expansions reached
    (None, *num_trial, cumulative_priority, false, final_memo)
}

#[derive(Clone, Debug)]
pub enum RangeType {
    Bool,
    U4,
    U8,
    Top,
    Const(i64),
}

pub fn make_init_val(
    col_idx: usize,
    range_types: &HashMap<usize, RangeType>,
    prime: u32,
) -> AbstractInterval {
    if range_types.contains_key(&col_idx) {
        match range_types.get(&col_idx).unwrap() {
            RangeType::Bool => AbstractInterval::bool(),
            RangeType::U8 => AbstractInterval::u8(),
            RangeType::U4 => AbstractInterval::u4(),
            RangeType::Top => AbstractInterval::top(prime),
            RangeType::Const(val) => AbstractInterval::from_i64(*val),
        }
    } else {
        AbstractInterval::i4()
    }
}

/// Auxiliary constraint object for multi-phase validation.
#[derive(Clone)]
pub struct AbsConstraintObj {
    pub name: String,
    pub aux_constraints: LatticeVMConstraints,
    pub aux_refinement_plan: Vec<usize>,
    pub aux_range_types: HashMap<usize, RangeType>,
}

/// Orchestrates a full symbolic refinement search over abstract traces, including auxiliary constraints.
///
/// `run_solver` performs a staged, prioritized search for an abstract trace that satisfies a given set
/// of constraints (`constraints`) and optionally a collection of auxiliary constraints (`aux_constraints_objs`).
/// It iterates over combinations of target columns (refinement_plan), initializes abstract traces,
/// applies program counter refinement, and pushes initial candidates into a priority queue.
///
/// Each candidate trace is refined iteratively using `solve`, which performs depth-prioritized refinement
/// and evaluates constraints. If a candidate satisfies all constraints, auxiliary tables are generated
/// and validated against their respective auxiliary constraints. If all checks pass, `final_check`
/// is invoked to finalize the solution (e.g., logging, UI update, or storing known solutions).
///
/// # Type Parameters
/// * `ProgramCounterRefinFn` - Closure to refine the program counter column at a specific row and column index.
/// * `FinalCheckFn` - Closure invoked after a candidate trace satisfies all constraints. Typically used
///   to update UI or store solutions.
/// * `AuxTableGenFn` - Closure generating auxiliary traces from the main trace and a set of boolean variable indices.
/// * `AlignPcToProgramFn` - Closure that enforces program semantics by synchronizing the PC column with opcode/operand intervals.
///
/// # Arguments
/// * `constraints` - Main lattice VM constraints for the primary abstract trace.
/// * `refinement_plan` - List of main trace columns to consider for refinement combinations.
/// * `potential_boolean_vars` - Column indices of boolean variables that may require special refinement.
/// * `aux_constraints_objs` - Auxiliary constraints to check on derived traces.
/// * `aux_table_gen_fns` - Functions to generate auxiliary tables for each auxiliary constraint object.
/// * `refinment_target_indicies_pv` - Column indices for refining public-value traces.
/// * `base_abs_main_trace_data` - Initial abstract trace table for main execution.
/// * `public_vals` - Abstract intervals for public trace values.
/// * `max_expansions` - Maximum number of search expansions allowed per refinement attempt.
/// * `minimum_num_taregt_cols` - Minimum number of columns to include in refinement combinations.
/// * `min_row_id` / `max_row_id` - Row bounds to apply refinement.
/// * `program_len` - Length of the program for program counter constraints.
/// * `program_counter_refine_fn` - Function to refine the PC column in the trace.
/// * `align_pc_to_program` - Function to align PC column to program semantics (opcode/operands).
/// * `final_check` - Function invoked when a candidate trace satisfies all constraints and auxiliary checks.
/// * `prime` - Field prime for interval arithmetic and canonical conversion.
/// * `seed` - Seed for deterministic random number generation used in shuffling refinement combinations.
/// * `ui` - Mutable reference to UI state for displaying search progress.
/// * `terminal` - Terminal backend used to render the UI.
pub fn run_solver<ProgramCounterRefinFn, FinalCheckFn, AuxTableGenFn, AlignPcToProgramFn>(
    constraints: &LatticeVMConstraints,
    refinement_plan: &Vec<usize>,
    range_types: &HashMap<usize, RangeType>,
    aux_constraints_objs: &Vec<AbsConstraintObj>,
    aux_table_gen_fns: &Vec<AuxTableGenFn>,
    refinment_target_indicies_pv: &Vec<usize>,
    base_abs_main_trace_data: &Vec<Vec<AbstractInterval>>,
    public_vals: Vec<AbstractInterval>,
    max_expansions: usize,
    minimum_num_taregt_cols: usize,
    min_row_id: usize,
    max_row_id: usize,
    program_len: usize,
    program_counter_refine_fn: ProgramCounterRefinFn,
    align_pc_to_program: AlignPcToProgramFn,
    final_check: FinalCheckFn,
    prime: u32,
    seed: u64,
    known_solution: &mut HashSet<String>,
    logs_num_solution: &mut Vec<(usize, usize)>,
    ui: &mut UiState,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) where
    ProgramCounterRefinFn: Fn(&mut Vec<Vec<AbstractInterval>>, usize, usize, usize),
    FinalCheckFn: Fn(&AbstractTrace, usize, u32, &mut HashSet<String>, &mut UiState),
    AuxTableGenFn: Fn(
        &Vec<Vec<AbstractInterval>>,
        &HashMap<usize, RangeType>,
        u32,
    ) -> Vec<Vec<AbstractInterval>>,
    AlignPcToProgramFn: Fn(&mut AbstractTrace, u32),
{
    // RNG and bookkeeping
    let mut rng = StdRng::seed_from_u64(seed);
    let mut found_solution_flag = false;
    let mut global_expansion_count = 0;
    let mut exit_flag = false;

    // ################################################################
    // Stage 1: iterate over subset sizes (from minimal to all columns)
    // ################################################################
    for k in minimum_num_taregt_cols..(refinement_plan.len() + 1) {
        // generate all k-sized subsets of refinement_plan and shuffle them
        let mut column_subsets: Vec<_> = refinement_plan.iter().combinations(k).collect();
        column_subsets.shuffle(&mut rng);

        // ###############################################################################
        // Stage 2: for each column subset, build initial abstract main trace & run search
        // ###############################################################################
        for column_subset in column_subsets {
            // clone base trace data to start from
            let mut abs_main_trace_data = base_abs_main_trace_data.clone();

            // convert combination iterator into Vec<usize>
            let refinment_target_indicies_main: Vec<usize> =
                column_subset.clone().into_iter().cloned().collect();
            //refinment_target_indicies_main.push(1);

            // apply coarse domain constraints for the chosen columns across rows
            for i in min_row_id..(max_row_id + 1) {
                for c in &refinment_target_indicies_main {
                    abs_main_trace_data[i][*c] = make_init_val(*c, &range_types, prime);

                    // apply program-counter-specific refinement for this cell
                    program_counter_refine_fn(&mut abs_main_trace_data, program_len, i, *c);
                }
            }

            // Create AbstractTrace from the prepared abstract table
            let abs_main_trace = AbstractTrace::new(abs_main_trace_data.clone());

            // ###################################################
            // Stage 3: initialize the queue with the initial node
            // ###################################################
            let mut queue: PriorityQueue<SearchNode, Potential> = PriorityQueue::new();
            queue.push(
                SearchNode {
                    main_trace: abs_main_trace,
                    public_trace: AbstractTrace::new(vec![public_vals.clone()]),
                    depth: 0,
                },
                (0, i32::MAX),
            );

            // run the main search over this column subset
            let mut num_trial = 0;
            while !queue.is_empty() && num_trial < max_expansions {
                let result = solve(
                    &mut queue,
                    &mut num_trial,
                    &constraints,
                    1,
                    &refinment_target_indicies_main,
                    &refinment_target_indicies_pv,
                    min_row_id,
                    max_row_id,
                    &align_pc_to_program,
                    max_expansions,
                    global_expansion_count,
                    prime,
                    &mut rng,
                    &format!("{:?}", column_subset),
                    ui,
                    terminal,
                    true,
                );

                // If a main trace was found, perform auxiliary validations
                if let (Some(trace), _, _, _, _) = result {
                    let mut output = String::new();
                    output.push_str(&format!(
                        "Trial ID: {}\n\n#Main\n{}",
                        global_expansion_count + result.1,
                        trace
                    ));

                    let mut pass_all_aux = true;

                    // dummy align function used when validating aux tables (no PC alignment needed)
                    fn dummy_align_pc_to_program(_at: &mut AbstractTrace, _prime: u32) {}

                    // #################################################################################
                    // Stage 5: validate auxiliary constraints by generating aux tables and solving them
                    // #################################################################################
                    for (co, gfn) in aux_constraints_objs.iter().zip(aux_table_gen_fns.iter()) {
                        // generate auxiliary table for the candidate main trace
                        let aux_table = gfn(&trace.data, &co.aux_range_types, prime);
                        if !aux_table.is_empty() {
                            let abs_aux_trace = AbstractTrace::new(aux_table.clone());

                            // create new queue & run solve() on aux constraints (depth-limited)
                            let mut aux_queue: PriorityQueue<SearchNode, Potential> =
                                PriorityQueue::new();
                            aux_queue.push(
                                SearchNode {
                                    main_trace: abs_aux_trace,
                                    public_trace: AbstractTrace::new(vec![public_vals.clone()]),
                                    depth: 0,
                                },
                                (0, i32::MAX),
                            );
                            let mut aux_num_trial = 0;
                            let aux_global_expansion_count = 0;

                            let abs_result = solve(
                                &mut aux_queue,
                                &mut aux_num_trial,
                                &co.aux_constraints,
                                1,
                                &co.aux_refinement_plan,
                                &refinment_target_indicies_pv,
                                0,
                                aux_table.len() - 1,
                                &dummy_align_pc_to_program,
                                max_expansions,
                                aux_global_expansion_count,
                                prime,
                                &mut rng,
                                &format!("{:?}", column_subset),
                                ui,
                                terminal,
                                false,
                            );

                            if let Some(abs_trace) = abs_result.0 {
                                // append aux trace output for diagnostics
                                output.push_str(&format!("\n#{}\n{}", co.name, abs_trace));
                            } else {
                                // aux failed — reject this main trace
                                pass_all_aux = false;
                                break;
                            }
                        }
                    }

                    // #############################################################################
                    // Stage 6: if all auxiliary checks passed, execute final_check and show results
                    // #############################################################################
                    if pass_all_aux {
                        ui.logs = output;
                        final_check(
                            &trace,
                            global_expansion_count + result.1,
                            prime,
                            known_solution,
                            ui,
                        );
                        found_solution_flag = true;

                        // re-draw UI to show final result
                        terminal
                            .draw(|f| {
                                ui.render::<CrosstermBackend<Stdout>>(f);
                            })
                            .unwrap();

                        logs_num_solution.push((global_expansion_count, known_solution.len()));
                    }
                }
                // update global expansion counter and check for exit signal
                //global_expansion_count += result.1;
                if result.3 {
                    exit_flag = true;
                    break;
                }
            }

            if exit_flag {
                break;
            }
        }

        if found_solution_flag {
            //break;
        }

        if exit_flag {
            break;
        }
    }
}
