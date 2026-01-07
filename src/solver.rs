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

use crate::symbolic::{
    apply_abir_refinement, detect_abir_constraints, detect_conditional_var_sub_const_constraints,
    detect_conditional_var_sub_var_constraints, refine_conditional_constraints_var_sub_const,
    refine_conditional_constraints_var_sub_var, AbirConstraint,
};
use crate::{
    interval::{AbstractInterval, MayBeFlag},
    symbolic::{eval_constraints, refine_trace, AbstractTrace, LatticeVMConstraints},
    ui::UiState,
};

#[derive(Clone, Debug)]
pub enum RangeType {
    Bool,
    U4,
    U8,
    U16,
    U7,
    Top,
    Const(i64),
    PosAny(usize),
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
            RangeType::U16 => AbstractInterval::u16(),
            RangeType::U7 => AbstractInterval { lo: 0, hi: 126 },
            RangeType::U4 => AbstractInterval::u4(),
            RangeType::Top => AbstractInterval::top(prime),
            RangeType::Const(val) => AbstractInterval::from_i64(*val),
            RangeType::PosAny(val) => AbstractInterval {
                lo: 0,
                hi: *val as i64,
            },
        }
    } else {
        AbstractInterval::top(prime)
    }
}

/// Messages sent from workers to the UI thread
pub enum SolverMsg {
    UpdateStats {
        trials: usize,
        unsat: usize,
        queue_len: usize,
    },
    SolutionFound(AbstractTrace, usize),
    Finished,
}

/// The outcome of processing a single node
enum NodeProcessingResult {
    Success(AbstractTrace),
    Pruned, // Unsatisfiable
    Refined(Vec<(SearchNode, Potential)>),
}

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

fn process_single_node(
    head: SearchNode,
    potential: Potential,
    constraints: &LatticeVMConstraints,
    range_types: &HashMap<usize, RangeType>,
    prime: u32,
    rng: &mut StdRng,
    align_pc_to_program: &impl Fn(&mut AbstractTrace, u32),
    refinment_target_indicies_main: &Vec<usize>,
    refinement_plan_pv: &Vec<usize>,
    bool_target_indices: &[usize],
    min_row_id: usize,
    max_row_id: usize,
    conditional_var_sub_const_constraints: &[(usize, usize, i64)],
    eq_constraints: &[(usize, usize, usize)],
    abir_constraints: &[AbirConstraint],
) -> NodeProcessingResult {
    let mut main_trace = head.main_trace;
    let public_trace = head.public_trace;

    // 1. Initial Refinements (ABIR, Conditional, etc.)
    // (Omitted for brevity, but same as your original solve() logic)
    // If any refinement returns MayBeFlag::False -> return NodeProcessingResult::Pruned

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
    if let MayBeFlag::False = apply_abir_refinement(&mut main_trace, &abir_constraints, prime).0 {
        return NodeProcessingResult::Pruned;
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
            )
            .0
        }
    };

    let (refined_public_candidates, _) = refine_trace(
        &public_trace,
        &refinement_plan_pv.to_vec(),
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
            return NodeProcessingResult::Pruned;
        }
    }
    children.shuffle(rng);

    // 3. Evaluate Children
    // Note: Since this is already a worker thread, we evaluate children sequentially
    // or push them all back to the global queue for other workers to grab.
    // Pushing them back is better for balancing high-level parallelism.

    let mut results = Vec::new();
    for mut kid_trace in children {
        align_pc_to_program(&mut kid_trace.0, prime);
        let (res, pot, _) =
            eval_constraints(&kid_trace.0, Some(&kid_trace.1.data[0]), constraints, prime);

        match res {
            MayBeFlag::True => return NodeProcessingResult::Success(kid_trace.0),
            MayBeFlag::False => {}
            MayBeFlag::MayBe => {
                results.push((
                    SearchNode {
                        main_trace: kid_trace.0,
                        public_trace: kid_trace.1,
                        depth: head.depth + 1,
                    },
                    (-pot, (head.depth as i32 + 1)),
                ));
            }
        }
    }

    if results.is_empty() {
        NodeProcessingResult::Pruned
    } else {
        NodeProcessingResult::Refined(results)
    }
}

// Return type: (Did we find a solution?, Did user request global exit?)
pub fn parallel_solve<AlignPcToProgramFn, FinalCheckFn>(
    initial_node: &SearchNode,
    constraints: Arc<LatticeVMConstraints>,
    refinement_plan: Arc<Vec<usize>>, // Specific to this subset
    refinement_plan_pv: Arc<Vec<usize>>,
    range_types: Arc<HashMap<usize, RangeType>>,
    align_pc_to_program: AlignPcToProgramFn,
    min_row_id: usize,
    max_row_id: usize,
    max_expansions: usize,
    num_workers: usize,
    seed: u64,
    prime: u32,
    ui: &mut UiState,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    final_check: FinalCheckFn,
    known_solution: &mut HashSet<String>,
    global_total_trials: Arc<AtomicUsize>,
) -> (bool, bool)
// (Found, Quit)
where
    AlignPcToProgramFn: Fn(&mut AbstractTrace, u32) + Clone + Send + Sync + 'static,
    FinalCheckFn: Fn(&AbstractTrace, usize, u32, &mut HashSet<String>, &mut UiState),
{
    // --- 1. PREP CONSTRAINTS ---
    // (Do this calculation once per subset)
    let bool_target_indices: Vec<usize> = refinement_plan
        .iter()
        .copied()
        .filter(|idx| matches!(range_types.get(idx), Some(RangeType::Bool)))
        .collect();
    let conditional_var_sub_const_constraints =
        detect_conditional_var_sub_const_constraints(&constraints.air_constraints, prime);
    let conditional_var_sub_var_constraints =
        detect_conditional_var_sub_var_constraints(&constraints.air_constraints);
    let abir_constraints = detect_abir_constraints(&constraints.air_constraints, prime);

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
    for wid in 0..num_workers {
        let q = queue.clone();
        let aw = active_workers.clone();
        let l_tr = local_trials.clone();
        let g_tr = global_total_trials.clone();
        let un = unsat.clone();
        let sd = shutdown.clone();
        let tx = tx.clone();

        // Clone Arc data
        let c_cons = constraints.clone();
        let c_rt = range_types.clone();
        let c_rp = refinement_plan.clone();
        let c_rp_pv = refinement_plan_pv.clone();
        let c_bool = bool_target_indices.clone();
        let c_cvsc = conditional_var_sub_const_constraints.clone();
        let c_cvsv = conditional_var_sub_var_constraints.clone();
        let c_abir = abir_constraints.clone();
        let c_align = align_pc_to_program.clone();

        thread::spawn(move || {
            let mut rng = StdRng::seed_from_u64(seed + wid as u64);
            // Time-based throttling to prevent freezing
            let mut last_ui_update = std::time::Instant::now();

            loop {
                if sd.load(Ordering::Relaxed) {
                    break;
                }

                // Check Max Expansions
                if l_tr.load(Ordering::Relaxed) >= max_expansions {
                    sd.store(true, Ordering::Relaxed);
                    let _ = tx.send(SolverMsg::Finished); // Signal main thread
                    break;
                }

                // POP
                let task = {
                    let mut lock = q.lock().unwrap();
                    lock.pop()
                };

                let (node, pot) = match task {
                    Some(x) => x,
                    None => {
                        // Termination Logic: Am I the last one and is queue empty?
                        let is_q_empty = q.lock().unwrap().is_empty();
                        if aw.load(Ordering::Relaxed) == 0 && is_q_empty {
                            sd.store(true, Ordering::Relaxed);
                            let _ = tx.send(SolverMsg::Finished);
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                };

                aw.fetch_add(1, Ordering::SeqCst);
                l_tr.fetch_add(1, Ordering::Relaxed);
                let my_global_id = g_tr.fetch_add(1, Ordering::SeqCst);

                // PROCESS
                let result = process_single_node(
                    node, pot, &c_cons, &c_rt, prime, &mut rng, &c_align, &c_rp, &c_rp_pv, &c_bool,
                    min_row_id, max_row_id, &c_cvsc, &c_cvsv, &c_abir,
                );

                match result {
                    NodeProcessingResult::Success(trace) => {
                        //sd.store(true, Ordering::Relaxed);
                        let _ = tx.send(SolverMsg::SolutionFound(trace, my_global_id));
                    }
                    NodeProcessingResult::Pruned => {
                        un.fetch_add(1, Ordering::Relaxed);
                    }
                    NodeProcessingResult::Refined(nodes) => {
                        // Optimization: Push all at once (BATCH PUSH) to reduce locking
                        let mut lock = q.lock().unwrap();
                        for (n, p) in nodes {
                            lock.push(n, p);
                        }
                    }
                }
                aw.fetch_sub(1, Ordering::SeqCst);

                // UI UPDATE (Time-based, not count-based)
                //if last_ui_update.elapsed().as_millis() > 100 {
                if last_ui_update.elapsed().as_millis() > 100 {
                    let _ = tx.send(SolverMsg::UpdateStats {
                        trials: my_global_id, //tr.load(Ordering::Relaxed),
                        unsat: un.load(Ordering::Relaxed),
                        queue_len: q.lock().unwrap().len(),
                    });
                    last_ui_update = std::time::Instant::now();
                }
            }
        });
    }

    drop(tx);

    // --- 4. MAIN UI LOOP (BLOCKING FOR THIS SUBSET) ---
    let mut solution_found = false;
    let mut user_quit = false;
    // let mut subset_finished = false;

    loop {
        // A. Drain Messages (Non-blocking)
        match rx.recv_timeout(Duration::from_millis(10)) {
            Ok(msg) => {
                match msg {
                    SolverMsg::UpdateStats {
                        trials,
                        unsat,
                        queue_len,
                    } => {
                        ui.status = format!(
                            "Subset: {:?}\n#Trials: {}\n#Unsat: {}\n#Queue: {}",
                            refinement_plan, trials, unsat, queue_len
                        );
                    }
                    SolverMsg::SolutionFound(trace, trials) => {
                        ui.logs = format!("Trial ID: {}\n\n#Main\n{}", trials, trace);

                        final_check(&trace, trials, prime, known_solution, ui);
                        solution_found = true;
                        // shutdown.store(true, Ordering::SeqCst);
                    }
                    SolverMsg::Finished => {
                        // Workers exhausted this subset
                        // subset_finished = true;
                    }
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // メッセージが来ない間は描画更新とキー入力チェック
                terminal
                    .draw(|f| ui.render::<CrosstermBackend<std::io::Stdout>>(f))
                    .unwrap();

                if event::poll(Duration::from_millis(1)).unwrap() {
                    if let Event::Key(key) = event::read().unwrap() {
                        if key.code == KeyCode::Char('q') || key.code == KeyCode::Char('c') {
                            user_quit = true;
                            shutdown.store(true, Ordering::SeqCst);
                            // ユーザー強制終了の場合は即抜ける
                            return (solution_found, user_quit);
                        }
                    }
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                // all threads have been terminated
                break;
            }
        }
    }

    (solution_found, user_quit)
}

pub fn run_parallel_solver<ProgramCounterRefinFn, FinalCheckFn, AlignPcToProgramFn>(
    constraints: &LatticeVMConstraints,
    refinement_plan: &Vec<usize>, // Full plan
    range_types: &HashMap<usize, RangeType>,
    refinement_plan_pv: &Vec<usize>,
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
    FinalCheckFn: Fn(&AbstractTrace, usize, u32, &mut HashSet<String>, &mut UiState) + Clone,
    AlignPcToProgramFn: Fn(&mut AbstractTrace, u32) + Clone + Send + Sync + 'static,
{
    // Arc wrappers for constant data shared across all subsets
    let shared_constraints = Arc::new(constraints.clone());
    let shared_range_types = Arc::new(range_types.clone());
    let shared_pv_plan = Arc::new(refinement_plan_pv.clone());
    let global_count = Arc::new(AtomicUsize::new(0));

    let mut rng = StdRng::seed_from_u64(seed);

    // --- OUTER LOOP: Subset Sizes ---
    'outer: for k in minimum_num_taregt_cols..(refinement_plan.len() + 1) {
        let mut column_subsets: Vec<_> = refinement_plan.iter().combinations(k).collect();
        column_subsets.shuffle(&mut rng);

        // --- MIDDLE LOOP: Specific Subsets ---
        for column_subset in column_subsets {
            // 1. Prepare Initial Trace for this subset
            let mut abs_main_trace_data = base_abs_main_trace_data.clone();
            let subset_indices: Vec<usize> = column_subset.iter().cloned().cloned().collect();

            for i in min_row_id..(max_row_id + 1) {
                for c in &subset_indices {
                    abs_main_trace_data[i][*c] = make_init_val(*c, range_types, prime);
                    program_counter_refine_fn(&mut abs_main_trace_data, program_len, i, *c);
                }
            }

            let initial_node = SearchNode {
                main_trace: AbstractTrace::new(abs_main_trace_data),
                public_trace: AbstractTrace::new(vec![public_vals.clone()]),
                depth: 0,
            };

            // 2. RUN SOLVER for THIS subset
            // We pass ownership of subset_indices wrapped in Arc
            let (_found, quit) = parallel_solve(
                &initial_node,
                shared_constraints.clone(),
                Arc::new(subset_indices), // Subset specific plan
                shared_pv_plan.clone(),
                shared_range_types.clone(),
                align_pc_to_program.clone(),
                min_row_id,
                max_row_id,
                max_expansions,
                4, // Number of workers (adjust as needed)
                seed,
                prime,
                ui,
                terminal,
                final_check.clone(),
                known_solution,
                global_count.clone(),
            );

            // 3. DECIDE NEXT STEP
            if quit {
                // User pressed Q
                break 'outer;
            }
        }
    }
}

pub fn dummy_program_counter_refine_fn(
    _abs_main_trace_data: &mut Vec<Vec<AbstractInterval>>,
    _program_len: usize,
    _i: usize,
    _j: usize,
) {
}

pub fn dummy_adjust_pc_program(_main_trace: &mut AbstractTrace, _prime: u32) {}

pub fn dummy_table_deriver(
    _cpu_main_trace: &Vec<Vec<AbstractInterval>>,
    _range_types: &HashMap<usize, RangeType>,
    _prime: u32,
) -> Vec<Vec<AbstractInterval>> {
    let out = vec![];
    out
}
