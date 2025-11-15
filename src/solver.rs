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
                    "Target Columns: {}\n #Total Trial: {}\n #Trial {}\n #UNSAT Trial: {}\n #Qued: {}\n Potential: {}\n Sum-Potential: {}",
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

fn derive_add_table(
    cpu_main_trace: &Vec<Vec<AbstractInterval>>,
    potential_boolean_vars: &Vec<usize>,
    prime: u32,
) -> Vec<Vec<AbstractInterval>> {
    let mut out = vec![];
    for row in cpu_main_trace {
        if row[3].as_canonical_u32(prime) == 100 {
            let mut r: Vec<_> = (0..16).map(|_| AbstractInterval::top(prime)).collect();
            for i in potential_boolean_vars {
                r[*i] = AbstractInterval::bool();
            }

            let cpu_columns = vec![32, 33, 34, 35, 38, 39, 40, 41, 44, 45, 46, 47];
            let add_columns = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
            for i in 0..(cpu_columns.len()) {
                r[add_columns[i]] = row[cpu_columns[i]].clone();
            }
            r[15] = AbstractInterval::one();
            out.push(r);
        }
    }

    out
}

pub struct AbsConstraintObj {
    pub name: String,
    pub aux_constraints: LatticeVMConstraints,
    pub aux_target_cols: Vec<usize>,
    pub aux_potential_boolean_vars: Vec<usize>,
}

pub fn run_solver<FinalCheckFn, AuxTableGenFn>(
    constraints: &LatticeVMConstraints,
    target_cols: &Vec<usize>,
    potential_boolean_vars: &Vec<usize>,
    aux_constraints_objs: &Vec<AbsConstraintObj>,
    aux_table_gen_fns: &Vec<AuxTableGenFn>,
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
    FinalCheckFn: Fn(&AbstractTrace, u32, &mut UiState),
    AuxTableGenFn: Fn(&Vec<Vec<AbstractInterval>>, &Vec<usize>, u32) -> Vec<Vec<AbstractInterval>>,
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
                1000,
                cum_num_trial,
                prime,
                &mut rng,
                &format!("{:?}", combo),
                ui,
                terminal,
            );

            if let (Some(trace), _, _, _) = result {
                let mut output = String::new();
                output.push_str(&format!("#{}\n{}", cum_num_trial + result.1, trace));

                let mut pass_all_aux = true;
                for (co, gfn) in aux_constraints_objs.iter().zip(aux_table_gen_fns.iter()) {
                    let aux_table = gfn(&trace.data, &co.aux_potential_boolean_vars, prime);
                    let abs_aux_trace = AbstractTrace::new(aux_table);
                    let abs_result = solve(
                        abs_aux_trace,
                        public_vals.clone(),
                        &co.aux_constraints,
                        1,
                        &co.aux_target_cols,
                        &refinment_target_indicies_pv,
                        0,
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
                    final_check(&trace, prime, ui);
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
