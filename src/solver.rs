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

pub fn solve(
    initial_abs_main_trace: AbstractTrace,
    initial_public_vals: Vec<AbstractInterval>,
    constraints: &LatticeVMConstraints,
    num_refined_points: usize,
    refinment_target_indicies_main: &Vec<usize>,
    refinment_target_indicies_pv: &Vec<usize>,
    max_row_id: usize,
    maximum_num_trial: usize,
    cum_num_trial: usize,
    prime: u32,
    rng: &mut StdRng,
    meta_info: &str,
    ui: &mut UiState,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> (Option<AbstractTrace>, usize, i32, bool) {
    let mut queue: PriorityQueue<(AbstractTrace, AbstractTrace), i32> = PriorityQueue::new();
    queue.push(
        (
            initial_abs_main_trace,
            AbstractTrace::new(vec![initial_public_vals]),
        ),
        i32::MAX,
    );

    let mut num_trial = 0;
    let mut num_unsat_trial = 0;
    let mut sum_potential = 0;

    while !queue.is_empty() && num_trial < maximum_num_trial {
        num_trial += 1;

        let (head, potential) = queue.pop().unwrap();
        let trace = head.0;
        let public_vals = head.1;

        ui.status = format!(
                    "{}, #Total Trial: {}, #Trial {},  #UNSAT Trial: {}, #Qued: {}, Potential: {}, Sum-Potential: {}",
                    meta_info,
                    num_trial + cum_num_trial,
                    num_trial,
                    num_unsat_trial,
                    queue.len(),
                    -potential,
                    sum_potential,
                );

        terminal
            .draw(|f| {
                ui.render::<CrosstermBackend<Stdout>>(f);
            })
            .unwrap();

        if num_trial > 1 {
            sum_potential += potential;
        }

        let trace_children = refine_trace(
            &trace,
            num_refined_points,
            refinment_target_indicies_main,
            max_row_id,
            rng,
        );

        let pv_children = refine_trace(
            &public_vals,
            num_refined_points,
            refinment_target_indicies_pv,
            max_row_id,
            rng,
        );

        let mut chinldren = vec![];
        if let Some(trace_children) = trace_children {
            if let Some(pv_children) = pv_children {
                chinldren.push((trace_children.0.clone(), pv_children.0.clone()));
                chinldren.push((trace_children.1.clone(), pv_children.0));
                chinldren.push((trace_children.0, pv_children.1.clone()));
                chinldren.push((trace_children.1, pv_children.1));
            } else {
                chinldren.push((trace_children.0.clone(), public_vals.clone()));
                chinldren.push((trace_children.1.clone(), public_vals));
            }
        } else if let Some(pv_children) = pv_children {
            chinldren.push((trace.clone(), pv_children.0));
            chinldren.push((trace, pv_children.1));
        }
        for kid in chinldren {
            let (flag, potential) =
                eval_constraints(&kid.0, Some(&kid.1.data[0]), constraints, prime);
            match flag {
                MayBeFlag::True => {
                    return (Some(kid.0), num_trial, sum_potential, false);
                }
                MayBeFlag::False => {
                    num_unsat_trial += 1;
                }
                MayBeFlag::MayBe => {
                    queue.push(kid, -potential);
                }
            }
        }

        if event::poll(Duration::from_millis(1)).unwrap() {
            if let Event::Key(key) = event::read().unwrap() {
                if key.code == KeyCode::Char('q')
                    || (key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL))
                {
                    return (None, num_trial, sum_potential, true);
                }
            }
        }
    }

    (None, num_trial, sum_potential, false)
}

pub fn run_solver<FinalCheckFn>(
    constraints: &LatticeVMConstraints,
    target_cols: &Vec<usize>,
    potential_boolean_vars: &Vec<usize>,
    refinment_target_indicies_pv: &Vec<usize>,
    base_abs_main_trace_data: &Vec<Vec<AbstractInterval>>,
    public_vals: Vec<AbstractInterval>,
    max_row_id: usize,
    final_check: FinalCheckFn,
    prime: u32,
    seed: u64,
    ui: &mut UiState,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) where
    FinalCheckFn: Fn(&AbstractTrace, u32, &mut Terminal<CrosstermBackend<std::io::Stdout>>),
{
    let mut rng = StdRng::seed_from_u64(seed);
    let mut found_solution_flag = false;
    let mut cum_num_trial = 0;
    let mut exit_flag = false;

    for k in 1..(target_cols.len() + 1) {
        for combo in target_cols.iter().combinations(k) {
            let mut abs_main_trace_data = base_abs_main_trace_data.clone();

            for i in 0..(max_row_id + 1) {
                for c in &combo {
                    if potential_boolean_vars.contains(c) {
                        abs_main_trace_data[i][**c] = AbstractInterval::bool();
                    } else {
                        abs_main_trace_data[i][**c] = AbstractInterval::i4();
                    }
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
                max_row_id,
                1000000,
                cum_num_trial,
                prime,
                &mut rng,
                &format!("{:?}", combo),
                ui,
                terminal,
            );

            if let (Some(trace), _, _, _) = result {
                ui.logs = format!(
                    "#{}, Find SAT assignment: {}",
                    cum_num_trial + result.1,
                    trace
                );
                terminal
                    .draw(|f| {
                        ui.render::<CrosstermBackend<Stdout>>(f);
                    })
                    .unwrap();
                terminal
                    .draw(|f| {
                        ui.render::<CrosstermBackend<Stdout>>(f);
                    })
                    .unwrap();

                final_check(&trace, prime, terminal);
                found_solution_flag = true;
                //break;
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
