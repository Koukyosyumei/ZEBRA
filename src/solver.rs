use std::collections::HashSet;
use std::i32;
use std::io::Stdout;
use std::time::Duration;

use crossterm::event::KeyModifiers;
use crossterm::event::{self, Event, KeyCode};
use itertools::{Itertools, Powerset};
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

/// Auxiliary constraint object for multi-phase validation.
#[derive(Clone)]
pub struct AbsConstraintObj {
    pub name: String,
    pub aux_constraints: LatticeVMConstraints,
    pub aux_refinement_plan: Vec<usize>,
    pub aux_potential_boolean_vars: Vec<usize>,
}

/// Top-level orchestration function.
/// Responsibilities split into:
///  - iterate over column subset sizes
///  - build initial abstract traces for each subset
///  - run main solve() (refinement search)
///  - validate auxiliary constraints if a candidate is found
///  - perform final_check if everything passes
pub fn run_solver<ProgramCounterRefinFn, FinalCheckFn, AuxTableGenFn, AlignPcToProgramFn>(
    constraints: &LatticeVMConstraints,
    refinement_plan: &Vec<usize>,
    potential_boolean_vars: &Vec<usize>,
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
    ui: &mut UiState,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) where
    ProgramCounterRefinFn: Fn(&mut Vec<Vec<AbstractInterval>>, usize, usize, usize),
    FinalCheckFn: Fn(&AbstractTrace, usize, u32, &mut HashSet<String>, &mut UiState),
    AuxTableGenFn: Fn(&Vec<Vec<AbstractInterval>>, &Vec<usize>, u32) -> Vec<Vec<AbstractInterval>>,
    AlignPcToProgramFn: Fn(&mut AbstractTrace, u32),
{
    // RNG and bookkeeping
    let mut rng = StdRng::seed_from_u64(seed);
    let mut found_solution_flag = false;
    let mut global_expansion_count = 0;
    let mut exit_flag = false;
    let mut known_solution = HashSet::<String>::new();

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
            //// refinment_target_indicies_main.push(1);

            // apply coarse domain constraints for the chosen columns across rows
            for i in min_row_id..(max_row_id + 1) {
                for c in &refinment_target_indicies_main {
                    if potential_boolean_vars.contains(c) {
                        abs_main_trace_data[i][*c] = AbstractInterval::bool();
                    } else {
                        abs_main_trace_data[i][*c] = AbstractInterval::i4();
                    }

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
                        let aux_table = gfn(&trace.data, &co.aux_potential_boolean_vars, prime);
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
                            &mut known_solution,
                            ui,
                        );
                        found_solution_flag = true;

                        // re-draw UI to show final result
                        terminal
                            .draw(|f| {
                                ui.render::<CrosstermBackend<Stdout>>(f);
                            })
                            .unwrap();
                    }
                }
                // update global expansion counter and check for exit signal
                global_expansion_count += result.1;

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
