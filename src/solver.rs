use std::collections::{HashMap, HashSet};
use std::i32;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode};
use itertools::Itertools;
use priority_queue::PriorityQueue;
use rand::seq::SliceRandom;
use rand::{rngs::StdRng, SeedableRng};
use ratatui::{backend::CrosstermBackend, Terminal};
use serde::{Deserialize, Serialize};

use crate::shrinker::{
    apply_abir_refinement, detect_abir_constraints,
    detect_conditional_addvars_sub_const_constraints, detect_conditional_var_sub_const_constraints,
    detect_conditional_var_sub_var_constraints, detect_double_sel_addvars_sub_const,
    detect_double_sel_var_sub_const, detect_selector_addu_constraints,
    detect_selector_word_assign_constraints, refine_conditional_constraints_addvars_sub_const,
    refine_conditional_constraints_var_sub_const, refine_conditional_constraints_var_sub_var,
    refine_selector_addu_constraints, refine_selector_word_assign_constraints, AbirConstraint,
    SelectorAddUConstraint, SelectorWordAssignConstraint,
};
use crate::symbolic::{
    gather_boolean_variables, gather_vars, is_babybear_word_range,
    is_boolean_constraint, is_iszero_operator, is_koalabear_word_range, ZEBRASymbolicExpr,
};
use crate::{
    constraint::{eval_constraints, ZEBRAConstraints},
    interval::{AbstractInterval, MayBeFlag},
    trace::{refine_trace, AbstractTrace},
    ui::UiState,
};

/// Describes the allowed value range for a column when initializing an abstract state.
///
/// This enum is used to construct an [`AbstractInterval`] representing the
/// initial domain of a variable (e.g., a column in a trace). Variants correspond
/// to common bounded integer domains, constants, or unconstrained ranges.
///
/// # Variants
///
/// * `Bool`  — Boolean domain `{0, 1}`.
/// * `U4`    — Unsigned 4-bit integer domain `[0, 15]`.
/// * `U7`    — Unsigned 7-bit integer domain `[0, 126]`.
/// * `U8`    — Unsigned 8-bit integer domain `[0, 255]`.
/// * `U16`   — Unsigned 16-bit integer domain `[0, 65535]`.
/// * `Top`   — Unconstrained domain over the full field (bounded by the modulus).
/// * `Const(i128)` — A single constant value.
/// * `Any(i128, i128)` — Explicit inclusive interval `[lo, hi]`.
///
/// # Notes
///
/// * Domains are interpreted as integer intervals.
/// * `Top` typically spans the entire finite field defined by the program modulus.
/// * No validation is performed to ensure bounds are consistent with the field.
#[derive(Clone, Debug)]
pub enum RangeType {
    Bool,
    U4,
    U8,
    U16,
    U7,
    Top,
    Const(i128),
    Any(i128, i128),
}

/// Constructs the initial abstract interval for a given column.
///
/// If a range specification exists for `col_idx`, the corresponding interval
/// is returned. Otherwise, the column is initialized to the top (unconstrained)
/// interval over the field defined by `prime`.
///
/// # Parameters
///
/// * `col_idx` — Column index whose initial value is being created.
/// * `range_types` — Mapping from column indices to their allowed ranges.
/// * `prime` — Field modulus used when constructing top intervals.
///
/// # Returns
///
/// An [`AbstractInterval`] representing the initial domain of the column.
///
/// # Behavior
///
/// * If `range_types` contains an entry for `col_idx`, the interval is derived
///   from the associated [`RangeType`].
/// * If no entry exists, the column is treated as unconstrained (`Top`).
///
/// # Panics
///
/// This function does not panic under normal conditions.
///
/// # Notes
///
/// * The returned interval is purely symbolic and may later be refined
///   by constraint propagation.
/// * Bounds are not automatically reduced modulo `prime`.
pub fn make_init_val(
    col_idx: usize,
    range_types: &HashMap<usize, RangeType>,
    prime: u32,
) -> AbstractInterval {
    if range_types.contains_key(&col_idx) {
        match range_types.get(&col_idx).unwrap() {
            RangeType::Bool => AbstractInterval::bool(),
            RangeType::U8 => AbstractInterval::u8(),
            RangeType::U16 => AbstractInterval::u16(),
            RangeType::U7 => AbstractInterval { lo: 0, hi: 126 },
            RangeType::U4 => AbstractInterval::u4(),
            RangeType::Top => AbstractInterval::top(prime),
            RangeType::Const(val) => AbstractInterval::from_i128(*val),
            RangeType::Any(lo, hi) => AbstractInterval { lo: *lo, hi: *hi },
        }
    } else {
        AbstractInterval::top(prime)
    }
}

/// Messages sent from worker threads to the UI or coordinator thread.
///
/// This enum enables asynchronous progress reporting and result delivery
/// during parallel search or solving.
///
/// # Variants
///
/// * `UpdateStats` — Periodic status update.
///     * `trials` — Number of nodes processed so far.
///     * `unsat` — Number of nodes determined unsatisfiable.
///     * `queue_len` — Current size of the work queue.
/// * `SolutionFound` — A valid solution trace has been discovered.
///     * Contains the solution trace and the number of steps taken.
/// * `Finished` — All work has completed and no further messages will follow.
///
/// # Threading
///
/// Intended for use with channels in a multi-threaded solver architecture.
pub enum SolverMsg {
    UpdateStats {
        trials: usize,
        unsat: usize,
        queue_len: usize,
    },
    SolutionFound(AbstractTrace, usize),
    Finished,
}

/// Result of processing a single search node.
///
/// Returned by worker routines after attempting refinement and constraint
/// evaluation on a node.
///
/// # Variants
///
/// * `Success` — A satisfying trace has been found.
/// * `Pruned` — The node is unsatisfiable and should be discarded.
/// * `Refined` — The node produced child nodes that should be explored.
///
/// This type guides the global search strategy (e.g., queue expansion).
enum NodeProcessingResult {
    Done {
        solutions: Vec<AbstractTrace>,
        refined: Vec<(SearchNode, Potential)>,
    },
    Pruned, // Unsatisfiable
}

/// Heuristic score associated with a node in the search space.
///
/// Represented as `(priority, depth_penalty)` or another solver-specific
/// ordering metric. Lower values typically indicate higher priority,
/// but interpretation depends on the queue implementation.
///
/// Used to guide best-first or heuristic search strategies.
pub type Potential = (i32, i32);

/// A node in the solver's search space.
///
/// Each node represents a partially refined abstract execution trace
/// along with its depth in the refinement tree.
///
/// # Fields
///
/// * `main_trace` — The abstract trace representing current constraints.
/// * `depth` — Number of refinement steps applied from the root.
///
/// # Usage
///
/// Nodes are stored in priority queues or work lists and expanded
/// during the search for satisfying executions.
///
/// # Traits
///
/// Implements `Eq`, `Hash`, and `PartialEq` to allow deduplication
/// and storage in hash-based collections.
#[derive(Eq, Hash, PartialEq, Clone, Debug)]
pub struct SearchNode {
    main_trace: AbstractTrace,
    depth: usize,
}

#[derive(Clone, Debug)]
pub enum VerificationStatus {
    Verified,
    TimedOut,
    ResourceLimitReached,
    Interrupted,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(default)]
pub struct SearchConfig {
    pub num_workers: usize,
    pub time_out_ms: u64,
    pub minimum_num_taregt_cols: usize,
    pub max_expansions: usize,
    pub min_row_id: usize,
    pub max_row_id: usize,
    pub seed: u64,
    pub enable_heuristic: bool,
    pub enable_interval_refinement: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        SearchConfig {
            num_workers: 8,
            time_out_ms: 10000,
            minimum_num_taregt_cols: 0,
            max_expansions: 1000000000,
            min_row_id: 0,
            max_row_id: 0,
            seed: 41,
            enable_heuristic: true,
            enable_interval_refinement: true,
        }
    }
}

#[derive(Clone)]
pub struct ConstraintInfo {
    pub constraints: ZEBRAConstraints,
    pub num_total_columns: usize,
    pub num_pv_columns: usize,
    pub output_columns: Vec<usize>,
    pub refinable_cols: Vec<usize>,
    pub range_types: HashMap<usize, RangeType>,
    pub prime: u32,
}

/// Processes a single search node by applying refinements, generating children,
/// and evaluating constraints.
///
/// This function performs one expansion step in the solver's search procedure.
/// It applies local constraint propagation, attempts domain refinement, and
/// evaluates candidate child traces.
///
/// # Parameters
///
/// * `head` — The node to process.
/// * `public_trace` — Public input trace used for constraint evaluation.
/// * `constraints` — Global VM constraints to be satisfied.
/// * `prime` — Field modulus.
/// * `rng` — Random number generator for stochastic refinement.
/// * `post_process` — Callback that performs post-processes.
/// * `refinment_target_indicies_main` — Preferred indices for general refinement.
/// * `bool_target_indices` — Indices prioritized for boolean refinement.
/// * `min_row_id`, `max_row_id` — Row bounds eligible for refinement.
/// * `conditional_var_sub_const_constraints` — Conditional constraints of the form
///   variable minus constant.
/// * `eq_constraints` — Variable equality constraints.
/// * `abir_constraints` — Additional ABIR constraints.
///
/// # Returns
///
/// A [`NodeProcessingResult`] indicating whether:
///
/// * a solution was found,
/// * the node should be pruned, or
/// * new child nodes were generated.
///
/// # Algorithm Overview
///
/// 1. Apply constraint-driven refinements.
/// 2. Generate candidate refinements of the trace.
/// 3. Evaluate each child against the constraints.
/// 4. Return immediately if a satisfying trace is found.
/// 5. Otherwise return refined children with heuristic priorities.
///
/// # Pruning
///
/// If any refinement proves the node unsatisfiable, the node is discarded
/// without generating children.
///
/// # Parallelism
///
/// Intended to run inside worker threads. Generated children can be pushed
/// back to a global queue for load balancing.
///
/// # Notes
///
/// * Evaluation is sequential within this function.
/// * Heuristic potentials are computed to guide future exploration.
/// * Randomization helps avoid pathological search orderings.
fn process_single_node(
    head: SearchNode,
    public_trace: AbstractTrace,
    constraints: &ZEBRAConstraints,
    prime: u32,
    rng: &mut StdRng,
    post_process: &impl Fn(&mut AbstractTrace, u32) -> MayBeFlag,
    refinment_target_indicies_main: &Vec<usize>,
    bool_target_indices: &[usize],
    min_row_id: usize,
    max_row_id: usize,
    conditional_var_sub_const_constraints: &[(usize, usize, i128)],
    conditional_addvars_sub_const_constraints: &[(usize, Vec<usize>, i128)],
    eq_constraints: &[(usize, usize, usize)],
    abir_constraints: &[AbirConstraint],
    _double_sel_var_sub_const: &[(usize, bool, usize, bool, usize, i128)],
    _double_sel_addvars_sub_const: &[(usize, bool, usize, bool, Vec<usize>, i128)],
    selector_addu_constraints: &[SelectorAddUConstraint],
    selector_word_assign_constraints: &[SelectorWordAssignConstraint],
    is_balanced: bool,
    is_backward_refine_on: bool,
    enable_heuristic: bool,
) -> NodeProcessingResult {
    let mut main_trace = head.main_trace;

    // 1. Initial Refinements (ABIR, Conditional, etc.)
    // (Omitted for brevity, but same as your original solve() logic)
    // If any refinement returns MayBeFlag::False -> return NodeProcessingResult::Pruned

    if is_backward_refine_on {
        if let MayBeFlag::False = refine_conditional_constraints_var_sub_const(
            &mut main_trace,
            &conditional_var_sub_const_constraints,
        ) {
            return NodeProcessingResult::Pruned;
        }
        if let MayBeFlag::False =
            refine_conditional_constraints_var_sub_var(&mut main_trace, &eq_constraints)
        {
            return NodeProcessingResult::Pruned;
        }
        if let MayBeFlag::False = apply_abir_refinement(&mut main_trace, &abir_constraints, prime).0
        {
            return NodeProcessingResult::Pruned;
        }
        if let MayBeFlag::False = refine_conditional_constraints_addvars_sub_const(
            &mut main_trace,
            &conditional_addvars_sub_const_constraints,
        ) {
            return NodeProcessingResult::Pruned;
        }

        /*
        if let MayBeFlag::False =
            refine_double_sel_var_sub_const(&mut main_trace, double_sel_var_sub_const)
        {
            return NodeProcessingResult::Pruned;
        }
        if let MayBeFlag::False =
            refine_double_sel_addvars_sub_const(&mut main_trace, double_sel_addvars_sub_const)
        {
            return NodeProcessingResult::Pruned;
        }*/
        if let MayBeFlag::False =
            refine_selector_addu_constraints(&mut main_trace, selector_addu_constraints, prime)
        {
            return NodeProcessingResult::Pruned;
        }
        if let MayBeFlag::False = refine_selector_word_assign_constraints(
            &mut main_trace,
            selector_word_assign_constraints,
            prime,
        ) {
            return NodeProcessingResult::Pruned;
        }
    }

    // 2. Generate Children
    let refined_main_candidates = {
        let (refined_main_candidates, refined_flag) = refine_trace(
            &main_trace,
            &bool_target_indices.to_vec(),
            min_row_id,
            max_row_id,
            prime,
            rng,
            is_balanced,
        );
        if refined_flag {
            refined_main_candidates
        } else {
            refine_trace(
                &main_trace,
                &refinment_target_indicies_main.to_vec(),
                min_row_id,
                max_row_id,
                prime,
                rng,
                is_balanced,
            )
            .0
        }
    };

    // produce children as pairs (main_candidate, public_candidate)
    let mut children = vec![];
    match refined_main_candidates {
        Some(main_cands) => {
            for main_c in main_cands {
                children.push(main_c.clone());
            }
        }
        None => {
            // No refinements produced → continue to next queue entry
            return NodeProcessingResult::Pruned;
        }
    }
    children.shuffle(rng);

    // 3. Evaluate Children
    // Note: Since this is already a worker thread, we evaluate children sequentially
    // or push them all back to the global queue for other workers to grab.
    // Pushing them back is better for balancing high-level parallelism.

    let mut solutions = Vec::new();
    let mut refined = Vec::new();

    //let mut results = Vec::new();
    for mut kid_trace in children {
        let post_res = post_process(&mut kid_trace, prime);
        let (res, pot, _) =
            eval_constraints(&kid_trace, Some(&public_trace.data[0]), constraints, prime);

        match (res, post_res) {
            (MayBeFlag::True, MayBeFlag::True) => solutions.push(kid_trace),
            (MayBeFlag::False, _) | (_, MayBeFlag::False) => {}
            (MayBeFlag::MayBe, MayBeFlag::MayBe)
            | (MayBeFlag::MayBe, MayBeFlag::True)
            | (MayBeFlag::True, MayBeFlag::MayBe) => {
                let priority = if enable_heuristic {
                    (-pot, head.depth as i32 + 1)
                } else {
                    (0, head.depth as i32 + 1)
                };
                refined.push((
                    SearchNode {
                        main_trace: kid_trace,
                        depth: head.depth + 1,
                    },
                    priority,
                ));
            }
        }
    }

    if solutions.is_empty() && refined.is_empty() {
        NodeProcessingResult::Pruned
    } else {
        NodeProcessingResult::Done { solutions, refined }
    }
}

/// Runs a parallel best-first search to solve a constraint system over abstract traces,
/// with live UI updates and cooperative termination.
///
/// This function explores the refinement search space starting from `initial_node`,
/// using multiple worker threads and a shared priority queue. Workers repeatedly
/// expand nodes via [`process_single_node`], evaluate constraints, and push refined
/// candidates back into the queue until a solution is found, the search space is
/// exhausted, a maximum expansion limit is reached, or the user requests exit.
///
/// The solver operates on a *subset* of refinable columns (`refinable_cols`),
/// allowing callers to partition the search space across multiple invocations.
///
/// # Parameters
///
/// * `initial_node` — Root node of the search (will be cloned internally).
/// * `public_trace` — Public input trace used during constraint evaluation.
/// * `constraints` — Shared VM constraint system.
/// * `refinable_cols` — Column indices allowed to be refined in this subset.
/// * `range_types` — Domain specifications for columns.
/// * `post_process` — Callback that adjusts traces to valid program counters.
/// * `search_config` - Search config
/// * `prime` — Field modulus.
/// * `ui` — Mutable UI state for rendering progress and logs.
/// * `terminal` — Terminal backend used for drawing the UI.
/// * `final_check` — Callback invoked when a candidate solution is found.
/// * `known_solution` — Set used to deduplicate previously discovered solutions.
/// * `global_total_trials` — Global counter shared across subsets.
/// * `sleep_time` — Accumulates idle sleep time to throttle CPU usage.
///
/// # Returns
///
/// A pair `(found, quit)` where:
///
/// * `found` — `true` if at least one satisfying trace was discovered.
/// * `quit` — `true` if the user requested global termination (e.g., pressed `q`).
///
/// # Algorithm Overview
///
/// 1. **Preprocessing**
///    * Detects conditional boolean constraints and adjusts target indices.
///    * Extracts auxiliary constraint forms (conditional, equality, ABIR).
///
/// 2. **Shared State Initialization**
///    * Creates a thread-safe priority queue of search nodes.
///    * Initializes counters and shutdown flags.
///    * Enqueues the initial node.
///
/// 3. **Worker Execution**
///    Each worker:
///    * Pops nodes from the queue
///    * Applies refinements and evaluates constraints
///    * Reports results via a message channel
///    * Pushes refined children back into the queue
///    * Periodically sends progress updates
///
/// 4. **UI Event Loop**
///    The main thread:
///    * Receives worker messages
///    * Updates status and logs
///    * Invokes `final_check` on candidate solutions
///    * Handles user input (e.g., quit requests)
///    * Terminates when the queue is empty and all workers are idle
///
/// # Concurrency Model
///
/// * Workers share a priority queue protected by a mutex.
/// * Progress and results are communicated through an MPSC channel.
/// * A shutdown flag enables cooperative termination.
/// * Each worker uses an independent RNG stream.
///
/// # Termination Conditions
///
/// The search stops when any of the following occurs:
///
/// * A solution is found (search continues unless externally stopped)
/// * The priority queue becomes empty and all workers are idle
/// * `max_expansions` is reached
/// * The user presses `q` or `c`
///
/// # UI Behavior
///
/// * Progress statistics are updated periodically (time-based).
/// * Rendering occurs at a fixed tick rate.
/// * Logs include discovered solution traces.
///
/// # Notes
///
/// * The search is heuristic and not guaranteed to find a solution.
/// * Multiple solutions may be discovered during a single run.
/// * `initial_node` is not consumed; a clone is used internally.
/// * This function blocks until completion for the given subset.
///
/// # Thread Safety
///
/// All shared structures use `Arc` and atomic primitives to ensure
/// safe concurrent access.
///
/// # Panics
///
/// May panic if terminal rendering fails or if mutexes are poisoned.
///
/// # See Also
///
/// * [`process_single_node`] — Core node expansion routine.
/// * [`SolverMsg`] — Messages exchanged between workers and UI thread.
/// * [`SearchNode`] — Representation of search states.
pub fn parallel_solve<PostProcessFn, FinalCheckFn>(
    initial_node: &mut SearchNode,
    public_trace: AbstractTrace,
    constraints: Arc<ZEBRAConstraints>,
    refinable_cols: Arc<Vec<usize>>, // Specific to this subset
    range_types: Arc<HashMap<usize, RangeType>>,
    post_process: PostProcessFn,
    search_config: SearchConfig,
    prime: u32,
    ui: &mut UiState,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    final_check: FinalCheckFn,
    known_solution: &mut HashSet<String>,
    known_solution_area: &mut i128,
    global_total_trials: Arc<AtomicUsize>,
    sleep_time: &mut Duration,
    start_time: &std::time::Instant,
    is_balanced: bool,
    is_backward_refine_on: bool,
) -> (VerificationStatus, bool, bool)
// (Found, Quit)
where
    PostProcessFn: Fn(&mut AbstractTrace, u32) -> MayBeFlag + Clone + Send + Sync + 'static,
    FinalCheckFn: Fn(&AbstractTrace, usize, u32, &mut HashSet<String>, &mut UiState, &mut i128),
{
    let time_out = Duration::from_millis(search_config.time_out_ms);
    let mut conditional_bool_target_indices = Vec::<(usize, usize)>::new();

    for t in &constraints.air_constraints {
        if let ZEBRASymbolicExpr::Mul(lhs, rhs) = t {
            for i in search_config.min_row_id..(search_config.max_row_id + 1) {
                let lv = lhs.eval(
                    &initial_node.main_trace.data[i],
                    if i >= initial_node.main_trace.data.len() - 1 {
                        None
                    } else {
                        Some(&initial_node.main_trace.data[i + 1])
                    },
                    Some(&public_trace.data[0]),
                    i == 0,
                    i <= initial_node.main_trace.data.len() - 2,
                    i == initial_node.main_trace.data.len() - 1,
                    prime,
                );
                if let MayBeFlag::True = lv.is_non_zero(prime) {
                    if let Some(j) = is_boolean_constraint(rhs) {
                        conditional_bool_target_indices.push((i, j));
                    }
                }
            }
        }
    }

    // --- 1. PREP CONSTRAINTS ---
    // (Do this calculation once per subset)
    let mut bool_target_indices: Vec<usize> = refinable_cols
        .iter()
        .copied()
        .filter(|idx| matches!(range_types.get(idx), Some(RangeType::Bool)))
        .collect();
    for c in conditional_bool_target_indices {
        if !bool_target_indices.contains(&c.1) {
            bool_target_indices.push(c.1.clone());
            for i in search_config.min_row_id..(search_config.max_row_id + 1) {
                if !initial_node.main_trace.data[i][c.1].is_singleton() {
                    initial_node.main_trace.data[i][c.1] = AbstractInterval::bool();
                }
            }
        }
    }

    let conditional_var_sub_const_constraints =
        detect_conditional_var_sub_const_constraints(&constraints.air_constraints, prime);
    let conditional_addvars_sub_const_constraints =
        detect_conditional_addvars_sub_const_constraints(&constraints.air_constraints, prime);
    let conditional_var_sub_var_constraints =
        detect_conditional_var_sub_var_constraints(&constraints.air_constraints);
    let abir_constraints = detect_abir_constraints(&constraints.air_constraints, prime);
    let double_sel_var_sub_const =
        detect_double_sel_var_sub_const(&constraints.air_constraints, prime);
    let double_sel_addvars_sub_const =
        detect_double_sel_addvars_sub_const(&constraints.air_constraints, prime);
    let selector_addu_constraints =
        detect_selector_addu_constraints(&constraints.lookup_constraints);
    let selector_word_assign_constraints =
        detect_selector_word_assign_constraints(&constraints.air_constraints, prime);

    // --- 2. ISOLATED SHARED STATE ---
    // These belong ONLY to this function call. They are dropped when function returns.
    let queue = Arc::new(Mutex::new(PriorityQueue::<SearchNode, Potential>::new()));
    let active_workers = Arc::new(AtomicUsize::new(0));
    let local_trials = Arc::new(AtomicUsize::new(0));
    let unsat = Arc::new(AtomicUsize::new(0));
    let shutdown = Arc::new(AtomicBool::new(false)); // Stops workers for THIS subset
    let (tx, rx) = mpsc::channel();

    // Push Start Node
    queue
        .lock()
        .unwrap()
        .push(initial_node.clone(), (0, i32::MAX));

    // --- 3. SPAWN WORKERS ---
    for wid in 0..search_config.num_workers {
        let q = queue.clone();
        let aw = active_workers.clone();
        let l_tr = local_trials.clone();
        let g_tr = global_total_trials.clone();
        let un = unsat.clone();
        let sd = shutdown.clone();
        let tx = tx.clone();
        let pv = public_trace.clone();

        // Clone Arc data
        let c_cons = constraints.clone();
        let c_rp = refinable_cols.clone();
        let c_bool = bool_target_indices.clone();
        let c_cvsc = conditional_var_sub_const_constraints.clone();
        let c_casc = conditional_addvars_sub_const_constraints.clone();
        let c_cvsv = conditional_var_sub_var_constraints.clone();
        let c_abir = abir_constraints.clone();
        let c_dsvsc = double_sel_var_sub_const.clone();
        let c_dsasc = double_sel_addvars_sub_const.clone();
        let c_saddu = selector_addu_constraints.clone();
        let c_swa = selector_word_assign_constraints.clone();
        let c_align = post_process.clone();

        thread::spawn(move || {
            let mut rng = StdRng::seed_from_u64(search_config.seed + wid as u64);
            // Time-based throttling to prevent freezing
            let mut last_ui_update = std::time::Instant::now();

            loop {
                aw.fetch_add(1, Ordering::SeqCst);

                if sd.load(Ordering::Relaxed) {
                    aw.fetch_sub(1, Ordering::SeqCst);
                    break;
                }

                // Check Max Expansions
                if l_tr.load(Ordering::Relaxed) >= search_config.max_expansions {
                    sd.store(true, Ordering::Relaxed);
                    let _ = tx.send(SolverMsg::Finished); // Signal main thread
                    aw.fetch_sub(1, Ordering::SeqCst);
                    break;
                }

                // POP
                let task = {
                    let mut lock = q.lock().unwrap();
                    lock.pop()
                };

                let (node, _pot) = match task {
                    Some(x) => x,
                    None => {
                        // Termination Logic: Am I the last one and is queue empty?
                        /*
                        let is_q_empty = q.lock().unwrap().is_empty();
                        if aw.load(Ordering::Relaxed) == 0 && is_q_empty {
                            sd.store(true, Ordering::Relaxed);
                            let _ = tx.send(SolverMsg::Finished);
                            break;
                        }*/
                        aw.fetch_sub(1, Ordering::SeqCst);
                        std::thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                };

                l_tr.fetch_add(1, Ordering::Relaxed);
                let my_global_id = g_tr.fetch_add(1, Ordering::SeqCst);

                // PROCESS
                let result = process_single_node(
                    node,
                    pv.clone(),
                    &c_cons,
                    prime,
                    &mut rng,
                    &c_align,
                    &c_rp,
                    &c_bool,
                    search_config.min_row_id,
                    search_config.max_row_id,
                    &c_cvsc,
                    &c_casc,
                    &c_cvsv,
                    &c_abir,
                    &c_dsvsc,
                    &c_dsasc,
                    &c_saddu,
                    &c_swa,
                    is_balanced,
                    is_backward_refine_on,
                    search_config.enable_heuristic,
                );

                match result {
                    NodeProcessingResult::Pruned => {
                        un.fetch_add(1, Ordering::Relaxed);
                    }
                    NodeProcessingResult::Done { solutions, refined } => {
                        for trace in solutions {
                            let _ = tx.send(SolverMsg::SolutionFound(trace, my_global_id));
                        }
                        if !refined.is_empty() {
                            let mut lock = q.lock().unwrap();
                            for (n, p) in refined {
                                lock.push(n, p);
                            }
                        }
                    }
                }

                // UI UPDATE (Time-based, not count-based)
                if last_ui_update.elapsed().as_millis() > 100 {
                    let _ = tx.send(SolverMsg::UpdateStats {
                        trials: my_global_id, //tr.load(Ordering::Relaxed),
                        unsat: un.load(Ordering::Relaxed),
                        queue_len: q.lock().unwrap().len(),
                    });
                    last_ui_update = std::time::Instant::now();
                }

                aw.fetch_sub(1, Ordering::SeqCst);
            }
        });
    }

    drop(tx);

    // --- 4. MAIN UI LOOP (BLOCKING FOR THIS SUBSET) ---
    let mut solution_found = false;
    let mut user_quit = false;
    // let mut subset_finished = false;

    ui.status = format!(
        "Subset: {:?}\n#Trials: {}\n#Unsat: {}\n#Queue: {}",
        refinable_cols, 0, 0, 1
    );
    terminal
        .draw(|f| ui.render::<CrosstermBackend<std::io::Stdout>>(f))
        .unwrap();
    // 30 FPR = ~33ms
    let tick_rate = Duration::from_millis(50);
    let mut last_tick = std::time::Instant::now();

    loop {
        let mut _got_msg = false;

        for msg in rx.try_iter() {
            _got_msg = true;

            match msg {
                SolverMsg::UpdateStats {
                    trials,
                    unsat,
                    queue_len,
                } => {
                    ui.status = format!(
                        "Subset: {:?}\n#Trials: {}\n#Unsat: {}\n#Queue: {}",
                        refinable_cols, trials, unsat, queue_len
                    );
                }
                SolverMsg::SolutionFound(trace, trials) => {
                    ui.logs = format!(
                        "Trial ID: {}\n\n#Main\n{}\n#PV\n{}",
                        trials, trace, public_trace
                    );
                    final_check(
                        &trace,
                        trials,
                        prime,
                        known_solution,
                        ui,
                        known_solution_area,
                    );
                    solution_found = true;
                    if last_tick.elapsed() >= tick_rate {
                        terminal
                            .draw(|f| ui.render::<CrosstermBackend<std::io::Stdout>>(f))
                            .unwrap();
                        last_tick = std::time::Instant::now();
                    }
                }
                SolverMsg::Finished => {
                    // ui.status = format!("Subset: {:?}\nFinished", refinable_cols);
                    // Workers exhausted this subset
                    // subset_finished = true;
                }
            }

            if local_trials.load(Ordering::SeqCst) >= search_config.max_expansions {
                shutdown.store(true, Ordering::SeqCst);
                return (
                    VerificationStatus::ResourceLimitReached,
                    solution_found,
                    user_quit,
                );
            }

            if start_time.elapsed() >= time_out {
                shutdown.store(true, Ordering::SeqCst);
                return (VerificationStatus::TimedOut, solution_found, user_quit);
            }
        }

        // --------------------------------------------------------
        // 2. Check Key Inputs
        // --------------------------------------------------------
        if event::poll(Duration::from_millis(0)).unwrap() {
            if let Event::Key(key) = event::read().unwrap() {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('c') => {
                        user_quit = true;
                        shutdown.store(true, Ordering::SeqCst);
                        return (VerificationStatus::Interrupted, solution_found, user_quit);
                    }
                    _ => {}
                }
            }
        }

        // --------------------------------------------------------
        // 3. Update UI
        // --------------------------------------------------------
        if last_tick.elapsed() >= tick_rate {
            terminal
                .draw(|f| ui.render::<CrosstermBackend<std::io::Stdout>>(f))
                .unwrap();
            last_tick = std::time::Instant::now();
        }

        // --------------------------------------------------------
        // 4. Check whether all workers finished
        // --------------------------------------------------------
        if local_trials.load(Ordering::SeqCst) >= search_config.max_expansions {
            return (
                VerificationStatus::ResourceLimitReached,
                solution_found,
                user_quit,
            );
        }

        if start_time.elapsed() >= time_out {
            return (VerificationStatus::TimedOut, solution_found, user_quit);
        }

        let queue_empty = queue.lock().unwrap().is_empty();
        let workers_idle = active_workers.load(Ordering::SeqCst) == 0;
        if queue_empty && workers_idle {
            for msg in rx.try_iter() {
                if let SolverMsg::SolutionFound(trace, trials) = msg {
                    final_check(
                        &trace,
                        trials,
                        prime,
                        known_solution,
                        ui,
                        known_solution_area,
                    );
                    solution_found = true;
                }
            }
            break;
        }

        // tiny sleep to avoid the over-usage of CPU
        *sleep_time += Duration::from_millis(10);
        thread::sleep(Duration::from_millis(10));
    }

    for msg in rx.try_iter() {
        if let SolverMsg::SolutionFound(trace, trials) = msg {
            final_check(
                &trace,
                trials,
                prime,
                known_solution,
                ui,
                known_solution_area,
            );
            solution_found = true;
        }
    }

    (VerificationStatus::Verified, solution_found, user_quit)
}

/// Returns `true` if `tc` evaluates to definite zero on every row of `trace`.
///
/// Used to detect constraints that are trivially satisfied under the initial
/// (widest) abstract intervals and can be dropped before the search begins.
pub fn is_constraint_trivially_true(
    tc: &ZEBRASymbolicExpr,
    trace: &AbstractTrace,
    public_vals: &[AbstractInterval],
    prime: u32,
) -> bool {
    let num_steps = trace.data.len();
    (0..num_steps).all(|i| {
        matches!(
            tc.eval(
                &trace.data[i],
                if i + 1 < num_steps {
                    Some(&trace.data[i + 1])
                } else {
                    None
                },
                Some(public_vals),
                i == 0,
                i < num_steps - 1,
                i == num_steps - 1,
                prime,
            )
            .is_zero(prime),
            MayBeFlag::True
        )
    })
}

/// Orchestrates a parallel search over progressively larger subsets of refinable
/// columns, invoking [`parallel_solve`] for each subset until termination.
///
/// This function implements a hierarchical search strategy:
///
/// 1. Iterate over subset sizes `k`, starting from `minimum_num_taregt_cols`
///    up to the full set of refinable columns.
/// 2. For each size, enumerate combinations of columns (shuffled randomly).
/// 3. For each subset, construct a fresh initial abstract trace restricted
///    to those columns.
/// 4. Invoke the parallel solver on that subset.
/// 5. Stop early if the user requests termination.
///
/// The approach allows the solver to prioritize smaller refinement sets first,
/// which often yields solutions faster and reduces combinatorial explosion.
///
/// # Parameters
///
/// * `constraints_info` — VM constraint system to satisfy.
/// * `base_abs_main_trace_data` — Baseline abstract trace data used as a template.
/// * `public_vals` — Public input values (single-row trace).
/// * `search_config` - Search config
/// * `post_process` — Callback to align traces to valid program counters.
/// * `final_check` — Callback invoked when candidate solutions are found.
/// * `known_solution` — Set used to deduplicate discovered solutions.
/// * `ui` — Mutable UI state for progress reporting.
/// * `terminal` — Terminal backend for rendering.
/// * `sleep_time` — Accumulates solver idle time for throttling.
///
/// # Search Strategy
///
/// For each subset:
///
/// * The base trace is cloned.
/// * Selected columns are reinitialized using [`make_init_val`].
/// * Program counter domains may be further refined.
/// * A fresh root [`SearchNode`] is created.
/// * The subset is solved independently using multiple worker threads.
///
/// Subsets are processed in randomized order to avoid worst-case patterns.
///
/// # Termination
///
/// The outer loop stops when:
///
/// * All subsets have been explored, or
/// * The user requests exit (e.g., presses `q`)
///
/// Solutions discovered in earlier subsets do not prevent exploration of later
/// subsets unless externally terminated.
///
/// # Concurrency
///
/// Each subset search runs independently but shares immutable data via `Arc`.
/// A global trial counter tracks progress across subsets.
///
/// # Notes
///
/// * The function blocks until completion or user exit.
/// * The base trace template is never modified directly.
/// * Smaller subsets typically correspond to simpler hypotheses.
///
/// # Panics
///
/// May panic if terminal rendering fails.
///
/// # See Also
///
/// * [`parallel_solve`] — Performs the per-subset parallel search.
/// * [`make_init_val`] — Initializes column domains.
/// * [`SearchNode`] — Root node representation.
pub fn run_parallel_solver<FinalCheckFn, PostProcessFn>(
    constraint_info: &ConstraintInfo,
    base_abs_main_trace_data: &Vec<Vec<AbstractInterval>>,
    public_vals: Vec<AbstractInterval>,
    search_config: &SearchConfig,
    post_process: PostProcessFn,
    final_check: FinalCheckFn,
    known_solution: &mut HashSet<String>,
    known_solution_area: &mut i128,
    ui: &mut UiState,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    sleep_time: &mut Duration,
) -> (VerificationStatus, usize)
where
    FinalCheckFn:
        Fn(&AbstractTrace, usize, u32, &mut HashSet<String>, &mut UiState, &mut i128) + Clone,
    PostProcessFn: Fn(&mut AbstractTrace, u32) -> MayBeFlag + Clone + Send + Sync + 'static,
{
    // Arc wrappers for constant data shared across all subsets
    let shared_constraints = Arc::new(constraint_info.constraints.clone());
    let shared_range_types = Arc::new(constraint_info.range_types.clone());
    let global_count = Arc::new(AtomicUsize::new(0));
    let start_time = std::time::Instant::now();

    let mut rng = StdRng::seed_from_u64(search_config.seed);
    let mut last_verification_status = VerificationStatus::Interrupted;

    // --- OUTER LOOP: Subset Sizes ---
    'outer: for k in
        search_config.minimum_num_taregt_cols..(constraint_info.refinable_cols.len() + 1)
    {
        let mut column_subsets: Vec<_> = constraint_info
            .refinable_cols
            .iter()
            .combinations(k)
            .collect();
        column_subsets.shuffle(&mut rng);

        // --- MIDDLE LOOP: Specific Subsets ---
        for column_subset in column_subsets {
            // 1. Prepare Initial Trace for this subset
            let mut abs_main_trace_data = base_abs_main_trace_data.clone();
            let subset_indices: Vec<usize> = column_subset.iter().cloned().cloned().collect();

            for i in search_config.min_row_id..(search_config.max_row_id + 1) {
                for c in &subset_indices {
                    abs_main_trace_data[i][*c] =
                        make_init_val(*c, &constraint_info.range_types, constraint_info.prime);
                }
            }

            let mut initial_node = SearchNode {
                main_trace: AbstractTrace::new(abs_main_trace_data),
                depth: 0,
            };
            let public_trace = AbstractTrace::new(vec![public_vals.clone()]);

            // 2. RUN SOLVER for THIS subset
            // We pass ownership of subset_indices wrapped in Arc
            let (status, _found, quit) = parallel_solve(
                &mut initial_node,
                public_trace,
                shared_constraints.clone(),
                Arc::new(subset_indices), // Subset specific plan
                shared_range_types.clone(),
                post_process.clone(),
                search_config.clone(),
                constraint_info.prime,
                ui,
                terminal,
                final_check.clone(),
                known_solution,
                known_solution_area,
                global_count.clone(),
                sleep_time,
                &start_time,
                column_subset.len() == constraint_info.refinable_cols.len(),
                search_config.enable_interval_refinement
                    && column_subset.len() == constraint_info.refinable_cols.len(),
            );
            last_verification_status = status;

            if let VerificationStatus::TimedOut = last_verification_status {
                break 'outer;
            }

            // 3. DECIDE NEXT STEP
            if quit {
                // User pressed Q
                break 'outer;
            }
        }
    }

    (
        last_verification_status,
        global_count.load(Ordering::SeqCst),
    )
}

/// Preprocesses symbolic constraints to determine refinable columns and their
/// initial value ranges.
///
/// This function analyzes transition and lookup constraints to:
///
/// * Remove columns that should not be refined
/// * Normalize composite constraint patterns (e.g., `iszero`, word range checks)
/// * Identify variables actually used in constraints
/// * Infer domain types (Boolean, U8, U16, etc.)
///
/// The resulting outputs guide initialization and refinement during solving.
///
/// # Parameters
///
/// * `num_cols` — Total number of columns in the trace.
/// * `u8_cols` — Columns known to hold 8-bit unsigned values.
/// * `u16_cols` — Columns known to hold 16-bit unsigned values.
/// * `multiplicities` — Columns representing multiplicity counters (excluded).
/// * `received_vars_from_cpu` — Columns externally controlled by the CPU (excluded).
/// * `tv_constraints` — Transition constraints (modified in place).
/// * `lookup_symbolic_constraints` — Additional lookup constraints.
/// * `prime` — Field modulus used for symbolic analysis.
///
/// # Returns
///
/// A pair `(refinable_cols, range_types)` where:
///
/// * `refinable_cols` — Columns eligible for refinement.
/// * `range_types` — Mapping from column index to inferred [`RangeType`].
///
/// # Processing Steps
///
/// 1. **Initial Filtering**
///    Remove columns that must remain fixed (multiplicities, CPU inputs).
///
/// 2. **Constraint Normalization**
///    Detect and rewrite special constructs such as:
///
///    * `iszero` operators
///    * Word range checks (e.g., KoalaBear, BabyBear)
///
///    These patterns often expand into multiple simpler constraints.
///
/// 3. **Variable Usage Analysis**
///    Collect all variable indices appearing in constraints and retain only
///    those columns that are actually used.
///
/// 4. **Boolean Variable Detection**
///    Identify variables constrained to `{0,1}` and mark them as `RangeType::Bool`.
///
/// 5. **Explicit Range Assignment**
///    Override inferred ranges for known byte-width columns (`U8`, `U16`).
///
/// # Notes
///
/// * `tv_constraints` is modified in place to contain the normalized set.
/// * Lookup constraints contribute to variable usage but are not rewritten.
/// * Columns without explicit range types default to unconstrained (`Top`)
///   during initialization.
///
/// # Use Cases
///
/// Typically invoked once before launching the solver to construct the
/// refinement plan and domain information.
///
/// # Panics
///
/// This function does not panic under normal conditions.
///
/// # See Also
///
/// * [`RangeType`] — Domain specification enum.
/// * [`make_init_val`] — Uses the resulting range map for initialization.
pub fn prepare_constraints_and_range_type(
    num_cols: usize,
    u8_cols: &Vec<usize>,
    u16_cols: &Vec<usize>,
    multiplicities: &HashSet<usize>,
    received_vars_from_cpu: &HashSet<usize>,
    tv_constraints: &mut Vec<ZEBRASymbolicExpr>,
    lookup_symbolic_constraints: &Vec<ZEBRASymbolicExpr>,
    prime: u32,
    simplify_constraints: bool,
) -> (Vec<usize>, HashMap<usize, RangeType>) {
    let mut refinable_cols: Vec<usize> = (0..num_cols).collect();
    refinable_cols.retain(|c| !multiplicities.contains(c));
    refinable_cols.retain(|c| !received_vars_from_cpu.contains(c));

    if simplify_constraints {
        let mut new_tv_constraints = Vec::new();
        let mut is_in_koalabear_word_range_check = false;
        let mut is_in_babybear_word_range_check = false;
        let mut is_in_iszero_operator = false;
        for t in tv_constraints.iter() {
            if let Some(exprs) = is_iszero_operator(t, prime) {
                if is_in_iszero_operator {
                    is_in_iszero_operator = false;
                } else {
                    new_tv_constraints.push(exprs[0].clone());
                    new_tv_constraints.push(exprs[1].clone());
                    is_in_iszero_operator = true;
                }
            } else {
                if let Some(expr) = is_koalabear_word_range(t, prime) {
                    if is_in_koalabear_word_range_check {
                        is_in_koalabear_word_range_check = false;
                    } else {
                        new_tv_constraints.push(expr);
                        is_in_koalabear_word_range_check = true;
                    }
                } else if let Some(expr) = is_babybear_word_range(t, prime) {
                    if is_in_babybear_word_range_check {
                        is_in_babybear_word_range_check = false;
                    } else {
                        new_tv_constraints.push(expr);
                        is_in_babybear_word_range_check = true;
                    }
                } else {
                    if (!is_in_koalabear_word_range_check)
                        && (!is_in_iszero_operator)
                        && (!is_in_babybear_word_range_check)
                    {
                        new_tv_constraints.push(t.clone());
                    }
                }
            }
        }
        *tv_constraints = new_tv_constraints;
    }
    //tv_constraints.extend(lookup_symbolic_constraints);

    let mut used_vars = HashSet::new();
    for t in tv_constraints.iter() {
        gather_vars(0, t, &mut used_vars);
    }
    for t in lookup_symbolic_constraints.iter() {
        gather_vars(0, t, &mut used_vars);
    }
    let used_var_ids: HashSet<usize> = used_vars.iter().map(|x| x.1).collect();
    refinable_cols.retain(|c| used_var_ids.contains(c));

    let potential_boolean_vars = gather_boolean_variables(&tv_constraints, &multiplicities);
    let mut range_types: HashMap<usize, RangeType> = potential_boolean_vars
        .iter()
        .map(|k| (*k, RangeType::Bool))
        .collect();
    for c in u8_cols {
        range_types.insert(*c, RangeType::U8);
    }
    for c in u16_cols {
        range_types.insert(*c, RangeType::U16);
    }

    (refinable_cols, range_types)
}

pub fn dummy_program_counter_refine_fn(
    _abs_main_trace_data: &mut Vec<Vec<AbstractInterval>>,
    _program_len: usize,
    _i: usize,
    _j: usize,
) {
}

pub fn nop_post_process(_main_trace: &mut AbstractTrace, _prime: u32) -> MayBeFlag {
    MayBeFlag::True
}

pub fn dummy_table_deriver(
    _cpu_main_trace: &Vec<Vec<AbstractInterval>>,
    _range_types: &HashMap<usize, RangeType>,
    _prime: u32,
) -> Vec<Vec<AbstractInterval>> {
    let out = vec![];
    out
}
