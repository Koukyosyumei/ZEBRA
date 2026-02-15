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
    detect_conditional_var_sub_var_constraints, gather_boolean_variables, gather_vars,
    is_babybear_word_range, is_boolean_constraint, is_iszero_operator, is_koalabear_word_range,
    refine_conditional_constraints_var_sub_const, refine_conditional_constraints_var_sub_var,
    AbirConstraint, LatticeVMSymbolicExpr,
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
    Any(i64, i64),
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
            RangeType::Any(lo, hi) => AbstractInterval { lo: *lo, hi: *hi },
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
    depth: usize,
}

fn process_single_node(
    head: SearchNode,
    public_trace: AbstractTrace,
    constraints: &LatticeVMConstraints,
    prime: u32,
    rng: &mut StdRng,
    align_pc_to_program: &impl Fn(&mut AbstractTrace, u32),
    refinment_target_indicies_main: &Vec<usize>,
    bool_target_indices: &[usize],
    min_row_id: usize,
    max_row_id: usize,
    conditional_var_sub_const_constraints: &[(usize, usize, i64)],
    eq_constraints: &[(usize, usize, usize)],
    abir_constraints: &[AbirConstraint],
) -> NodeProcessingResult {
    let mut main_trace = head.main_trace;

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

    let mut results = Vec::new();
    for mut kid_trace in children {
        align_pc_to_program(&mut kid_trace, prime);
        let (res, pot, _) =
            eval_constraints(&kid_trace, Some(&public_trace.data[0]), constraints, prime);

        match res {
            MayBeFlag::True => return NodeProcessingResult::Success(kid_trace),
            MayBeFlag::False => {}
            MayBeFlag::MayBe => {
                results.push((
                    SearchNode {
                        main_trace: kid_trace,
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
    initial_node: &mut SearchNode,
    public_trace: AbstractTrace,
    constraints: Arc<LatticeVMConstraints>,
    refinable_cols: Arc<Vec<usize>>, // Specific to this subset
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
    sleep_time: &mut Duration,
) -> (bool, bool)
// (Found, Quit)
where
    AlignPcToProgramFn: Fn(&mut AbstractTrace, u32) + Clone + Send + Sync + 'static,
    FinalCheckFn: Fn(&AbstractTrace, usize, u32, &mut HashSet<String>, &mut UiState),
{
    let mut conditional_bool_target_indices = Vec::<(usize, usize)>::new();

    for t in &constraints.air_constraints {
        if let LatticeVMSymbolicExpr::Mul(lhs, rhs) = t {
            for i in min_row_id..(max_row_id + 1) {
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
            for i in min_row_id..(max_row_id + 1) {
                if !initial_node.main_trace.data[i][c.1].is_singleton() {
                    initial_node.main_trace.data[i][c.1] = AbstractInterval::bool();
                }
            }
        }
    }

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
        let pv = public_trace.clone();

        // Clone Arc data
        let c_cons = constraints.clone();
        let c_rp = refinable_cols.clone();
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
                aw.fetch_add(1, Ordering::SeqCst);

                if sd.load(Ordering::Relaxed) {
                    aw.fetch_sub(1, Ordering::SeqCst);
                    break;
                }

                // Check Max Expansions
                if l_tr.load(Ordering::Relaxed) >= max_expansions {
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
                    min_row_id,
                    max_row_id,
                    &c_cvsc,
                    &c_cvsv,
                    &c_abir,
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

    // 描画更新の頻度を制御（例: 30 FPS = 約33ms, ここでは少し余裕を見て 50ms）
    let tick_rate = Duration::from_millis(50);
    let mut last_tick = std::time::Instant::now();

    loop {
        let mut got_msg = false;

        for msg in rx.try_iter() {
            got_msg = true;

            match msg {
                SolverMsg::UpdateStats {
                    trials,
                    unsat,
                    queue_len,
                } => {
                    // 書式生成はコストがかかるので、本当に描画が必要な時だけやる手もありますが、
                    // ここでは最新状態で上書きし続ける
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
                    final_check(&trace, trials, prime, known_solution, ui);
                    solution_found = true;
                }
                SolverMsg::Finished => {
                    // ui.status = format!("Subset: {:?}\nFinished", refinable_cols);
                    // Workers exhausted this subset
                    // subset_finished = true;
                }
            }
        }

        // 全スレッドが終了し、かつメッセージもない場合のチェック
        // (rx.try_iter()が終わった時点でDisconnectなら終了)
        // ただし、mpscは送信側がすべてドロップされると RecvError を返しますが、
        // try_iter は単にループを抜けるので、ここで明示的な終了判定を入れるか、
        // active_workers 等を見るのが確実です。
        // 簡単のため、shutdownフラグと solution_found で判定します。

        // --------------------------------------------------------
        // 2. キー入力のチェック (最優先)
        // --------------------------------------------------------
        // timeout ブロックの中ではなく、ループ毎に必ずチェックします。
        // poll(Duration::ZERO) はノンブロッキングです。
        if event::poll(Duration::from_millis(0)).unwrap() {
            if let Event::Key(key) = event::read().unwrap() {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('c') => {
                        user_quit = true;
                        shutdown.store(true, Ordering::SeqCst);
                        return (solution_found, user_quit);
                    }
                    _ => {}
                }
            }
        }

        // --------------------------------------------------------
        // 3. 描画更新 (一定時間経過時のみ)
        // --------------------------------------------------------
        if last_tick.elapsed() >= tick_rate {
            terminal
                .draw(|f| ui.render::<CrosstermBackend<std::io::Stdout>>(f))
                .unwrap();
            last_tick = std::time::Instant::now();
        }

        // 全ワーカーが終了したかの判定
        // (厳密には active_workers == 0 && queue empty ですが、
        //  SolverMsg::Finished をカウントする等の方法もあります。
        //  ここではシンプルに channel が切断されているかを確認する手段として
        //  rx.try_recv()のエラーを見る方法もありますが、
        //  上の try_iter ループを抜けたということは空なので、
        //  active_workers が 0 なら終了とみなせます)

        /*
        let workers_active = Arc::strong_count(&queue) > 1; // メインスレッドも持っているので > 1
        if !workers_active {
            break;
        }*/

        //if !got_msg {
        let queue_empty = queue.lock().unwrap().is_empty();
        let workers_idle = active_workers.load(Ordering::SeqCst) == 0;
        if queue_empty && workers_idle {
            break;
        }
        //}

        // ワーカーが全員死んでチャネルも空ならループを抜ける
        // (簡略化のため、Disconnect検知は recv() で行うのが一般的ですが、
        //  ここでは active_workers を見るか、単に少し sleep してループさせる)

        // 短いスリープを入れてCPU使用率100%を防ぐ
        *sleep_time += Duration::from_millis(10);
        thread::sleep(Duration::from_millis(10));
    }

    (solution_found, user_quit)
}

pub fn run_parallel_solver<ProgramCounterRefinFn, FinalCheckFn, AlignPcToProgramFn>(
    constraints: &LatticeVMConstraints,
    refinable_cols: &Vec<usize>, // Full plan
    range_types: &HashMap<usize, RangeType>,
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
    ui: &mut UiState,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    sleep_time: &mut Duration,
) where
    ProgramCounterRefinFn: Fn(&mut Vec<Vec<AbstractInterval>>, usize, usize, usize),
    FinalCheckFn: Fn(&AbstractTrace, usize, u32, &mut HashSet<String>, &mut UiState) + Clone,
    AlignPcToProgramFn: Fn(&mut AbstractTrace, u32) + Clone + Send + Sync + 'static,
{
    // Arc wrappers for constant data shared across all subsets
    let shared_constraints = Arc::new(constraints.clone());
    let shared_range_types = Arc::new(range_types.clone());
    let global_count = Arc::new(AtomicUsize::new(0));

    let mut rng = StdRng::seed_from_u64(seed);

    // --- OUTER LOOP: Subset Sizes ---
    'outer: for k in minimum_num_taregt_cols..(refinable_cols.len() + 1) {
        let mut column_subsets: Vec<_> = refinable_cols.iter().combinations(k).collect();
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

            let mut initial_node = SearchNode {
                main_trace: AbstractTrace::new(abs_main_trace_data),
                depth: 0,
            };
            let public_trace = AbstractTrace::new(vec![public_vals.clone()]);

            // 2. RUN SOLVER for THIS subset
            // We pass ownership of subset_indices wrapped in Arc
            let (_found, quit) = parallel_solve(
                &mut initial_node,
                public_trace,
                shared_constraints.clone(),
                Arc::new(subset_indices), // Subset specific plan
                shared_range_types.clone(),
                align_pc_to_program.clone(),
                min_row_id,
                max_row_id,
                max_expansions,
                8, // Number of workers (adjust as needed)
                seed,
                prime,
                ui,
                terminal,
                final_check.clone(),
                known_solution,
                global_count.clone(),
                sleep_time,
            );

            // 3. DECIDE NEXT STEP
            if quit {
                // User pressed Q
                break 'outer;
            }
        }
    }
}

pub fn prepare_constraints_and_range_type(
    num_cols: usize,
    u8_cols: &Vec<usize>,
    multiplicities: &HashSet<usize>,
    received_vars_from_cpu: &HashSet<usize>,
    tv_constraints: &mut Vec<LatticeVMSymbolicExpr>,
    lookup_symbolic_constraints: &Vec<LatticeVMSymbolicExpr>,
    prime: u32,
) -> (Vec<usize>, HashMap<usize, RangeType>) {
    let mut refinable_cols: Vec<usize> = (0..num_cols).collect();
    refinable_cols.retain(|c| !multiplicities.contains(c));
    refinable_cols.retain(|c| !received_vars_from_cpu.contains(c));

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

    (refinable_cols, range_types)
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
