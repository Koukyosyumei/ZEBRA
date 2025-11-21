use std::collections::{HashMap, HashSet};
use std::i32;
use std::io::Stdout;
use std::{io, thread, time::Duration};

use crossterm::event::KeyModifiers;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use itertools::Itertools;
use rand::seq::SliceRandom;
use rand::{rngs::StdRng, SeedableRng};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

use priority_queue::PriorityQueue;

use crate::{
    interval::{AbstractInterval, MayBeFlag},
    symbolic::{eval_constraints, refine_trace, AbstractTrace, LatticeVMConstraints},
    ui::UiState,
};

pub fn solve<AdjustPcProgramFn>(
    initial_abs_main_trace: AbstractTrace,
    initial_public_vals: Vec<AbstractInterval>,
    constraints: &LatticeVMConstraints,
    num_refined_points: usize,
    base_refinment_target_indicies_main: &Vec<usize>,
    refinment_target_indicies_pv: &Vec<usize>,
    min_row_id: usize,
    max_row_id: usize,
    adjust_pc_program: &AdjustPcProgramFn,
    maximum_num_trial: usize,
    cum_num_trial: usize,
    prime: u32,
    rng: &mut StdRng,
    _meta_info: &str,
    ui: &mut UiState,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> (Option<AbstractTrace>, usize, i32, bool, HashSet<usize>)
where
    AdjustPcProgramFn: Fn(&mut AbstractTrace, u32),
{
    let mut queue: PriorityQueue<(AbstractTrace, AbstractTrace, usize), (i32, i32)> =
        PriorityQueue::new();
    queue.push(
        (
            initial_abs_main_trace,
            AbstractTrace::new(vec![initial_public_vals]),
            0,
        ),
        (0, i32::MAX),
    );

    let mut num_trial = 0;
    let mut num_unsat_trial = 0;
    let mut sum_potential = 0;
    let refinment_target_indicies_main = base_refinment_target_indicies_main.clone();
    let mut final_memo = HashSet::new();

    while !queue.is_empty() && num_trial < maximum_num_trial {
        num_trial += 1;

        let (head, potential) = queue.pop().unwrap();
        let trace = head.0;
        let public_vals = head.1;

        ui.status = format!(
                    "Target Columns: {:?}\n #Total Trial: {}\n #Trial {}\n #UNSAT Trial: {}\n #Qued: {}\n Potential: {}\n Sum-Potential: {}",
                    refinment_target_indicies_main,
                    num_trial + cum_num_trial,
                    num_trial,
                    num_unsat_trial,
                    queue.len(),
                    -potential.0,
                    sum_potential,
                );

        terminal
            .draw(|f| {
                ui.render::<CrosstermBackend<Stdout>>(f);
            })
            .unwrap();

        if num_trial > 1 {
            sum_potential += potential.1;
        }

        let trace_children = refine_trace(
            &trace,
            &refinment_target_indicies_main,
            min_row_id,
            max_row_id,
            prime,
            rng,
        );

        let pv_children = refine_trace(
            &public_vals,
            refinment_target_indicies_pv,
            min_row_id,
            max_row_id,
            prime,
            rng,
        );

        let mut chinldren = vec![];
        if let Some(trace_children) = trace_children {
            if let Some(pv_children) = pv_children {
                for pc in pv_children {
                    for tc in &trace_children {
                        chinldren.push((tc.clone(), pc.clone()));
                    }
                }
            } else {
                for tc in trace_children {
                    chinldren.push((tc.clone(), public_vals.clone()));
                }
            }
        } else if let Some(pv_children) = pv_children {
            for pc in pv_children {
                chinldren.push((trace.clone(), pc.clone()));
            }
        }

        chinldren.shuffle(rng);

        for kid in &mut chinldren {
            adjust_pc_program(&mut kid.0, prime);
            let (flag, potential, memo) =
                eval_constraints(&kid.0, Some(&kid.1.data[0]), constraints, prime);
            final_memo = memo;
            match flag {
                MayBeFlag::True => {
                    num_unsat_trial += 1;
                    return (
                        Some(kid.0.clone()),
                        num_trial,
                        sum_potential,
                        false,
                        final_memo,
                    );
                }
                MayBeFlag::False => {
                    num_unsat_trial += 1;
                }
                MayBeFlag::MayBe => {
                    queue.push(
                        (kid.0.clone(), kid.1.clone(), head.2 + 1),
                        (-potential, (head.2 as i32)),
                    );
                }
            }
        }

        if event::poll(Duration::from_millis(1)).unwrap() {
            if let Event::Key(key) = event::read().unwrap() {
                if key.code == KeyCode::Char('q')
                    || (key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL))
                {
                    return (None, num_trial, sum_potential, true, final_memo);
                }

                if key.code == KeyCode::Down {
                    ui.scroll_recovered = ui.scroll_logs.saturating_add(1);
                } else if key.code == KeyCode::Up {
                    ui.scroll_recovered = ui.scroll_logs.saturating_sub(1);
                }
            }
        }
    }

    (None, num_trial, sum_potential, false, final_memo)
}

pub struct AbsConstraintObj {
    pub name: String,
    pub aux_constraints: LatticeVMConstraints,
    pub aux_target_cols: Vec<usize>,
    pub aux_potential_boolean_vars: Vec<usize>,
}

pub fn run_solver<ProgramCounterRefinFn, FinalCheckFn, AuxTableGenFn, AdjustPcProgramFn>(
    constraints: &LatticeVMConstraints,
    target_cols: &Vec<usize>,
    potential_boolean_vars: &Vec<usize>,
    aux_constraints_objs: &Vec<AbsConstraintObj>,
    aux_table_gen_fns: &Vec<AuxTableGenFn>,
    refinment_target_indicies_pv: &Vec<usize>,
    base_abs_main_trace_data: &Vec<Vec<AbstractInterval>>,
    public_vals: Vec<AbstractInterval>,
    min_row_id: usize,
    max_row_id: usize,
    program_counter_refine_fn: ProgramCounterRefinFn,
    adjust_pc_program: AdjustPcProgramFn,
    final_check: FinalCheckFn,
    prime: u32,
    seed: u64,
    ui: &mut UiState,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) where
    ProgramCounterRefinFn: Fn(&mut Vec<Vec<AbstractInterval>>, usize, usize),
    FinalCheckFn: Fn(&AbstractTrace, usize, u32, &mut UiState),
    AuxTableGenFn: Fn(&Vec<Vec<AbstractInterval>>, &Vec<usize>, u32) -> Vec<Vec<AbstractInterval>>,
    AdjustPcProgramFn: Fn(&mut AbstractTrace, u32),
{
    let mut rng = StdRng::seed_from_u64(seed);
    let mut found_solution_flag = false;
    let mut cum_num_trial = 0;
    let mut exit_flag = false;

    for k in 1..(target_cols.len() + 1) {
        let mut combos: Vec<_> = target_cols.iter().combinations(k).collect();
        combos.shuffle(&mut rng);
        for combo in combos {
            let mut abs_main_trace_data = base_abs_main_trace_data.clone();

            for i in min_row_id..(max_row_id + 1) {
                for c in &combo {
                    if potential_boolean_vars.contains(c) {
                        abs_main_trace_data[i][**c] = AbstractInterval::bool();
                    } else {
                        abs_main_trace_data[i][**c] = AbstractInterval::i4();
                    }

                    program_counter_refine_fn(&mut abs_main_trace_data, i, **c);
                }
            }

            let abs_main_trace = AbstractTrace::new(abs_main_trace_data.clone());
            let refinment_target_indicies_main = combo.clone().into_iter().cloned().collect();

            let result = solve(
                abs_main_trace,
                public_vals.clone(),
                &constraints,
                1,
                &refinment_target_indicies_main,
                &refinment_target_indicies_pv,
                min_row_id,
                max_row_id,
                &adjust_pc_program,
                1000,
                cum_num_trial,
                prime,
                &mut rng,
                &format!("{:?}", combo),
                ui,
                terminal,
            );

            /*
            if let (None, _, _, _, final_memo) = result {
                if final_memo.len() <= 2 {
                    for v in final_memo {
                        if !refinment_target_indicies_main.contains(&v) {
                            for i in min_row_id..(max_row_id + 1) {
                                if potential_boolean_vars.contains(&v) {
                                    abs_main_trace_data[i][v] = AbstractInterval::bool();
                                } else {
                                    abs_main_trace_data[i][v] = AbstractInterval::i4();
                                }
                                program_counter_refine_fn(&mut abs_main_trace_data, i, v);
                            }
                            refinment_target_indicies_main.push(v);
                        }
                    }

                    let abs_main_trace = AbstractTrace::new(abs_main_trace_data.clone());
                    result = solve(
                        abs_main_trace,
                        public_vals.clone(),
                        &constraints,
                        1,
                        &refinment_target_indicies_main,
                        &potential_boolean_vars,
                        &refinment_target_indicies_pv,
                        min_row_id,
                        max_row_id,
                        &program_counter_refine_fn,
                        1000,
                        cum_num_trial,
                        prime,
                        &mut rng,
                        &format!("{:?}", combo),
                        ui,
                        terminal,
                    );
                }
            }*/

            if let (Some(trace), _, _, _, _) = result {
                let mut output = String::new();
                output.push_str(&format!(
                    "Trial ID: {}\n\n#CPU\n{}",
                    cum_num_trial + result.1,
                    trace
                ));

                let mut pass_all_aux = true;

                fn dummy_adjust_pc_program(_at: &mut AbstractTrace, _prime: u32) {}

                for (co, gfn) in aux_constraints_objs.iter().zip(aux_table_gen_fns.iter()) {
                    let aux_table = gfn(&trace.data, &co.aux_potential_boolean_vars, prime);
                    let abs_aux_trace = AbstractTrace::new(aux_table.clone());
                    let abs_result = solve(
                        abs_aux_trace,
                        public_vals.clone(),
                        &co.aux_constraints,
                        1,
                        &co.aux_target_cols,
                        &refinment_target_indicies_pv,
                        0,
                        aux_table.len() - 1,
                        &dummy_adjust_pc_program,
                        1000,
                        cum_num_trial,
                        prime,
                        &mut rng,
                        &format!("{:?}", combo),
                        ui,
                        terminal,
                    );

                    if let Some(abs_trace) = abs_result.0 {
                        output.push_str(&format!("\n#{}\n{}", co.name, abs_trace));
                        //break;
                    } else {
                        pass_all_aux = false;
                        break;
                    }
                }

                if pass_all_aux {
                    ui.logs = output;
                    final_check(&trace, cum_num_trial + result.1, prime, ui);
                    found_solution_flag = true;

                    terminal
                        .draw(|f| {
                            ui.render::<CrosstermBackend<Stdout>>(f);
                        })
                        .unwrap();
                }
            } else {
                cum_num_trial += result.1;
            }

            if result.3 {
                exit_flag = true;
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
