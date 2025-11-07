use priority_queue::PriorityQueue;

use itertools::Itertools;
use rand::{rngs::StdRng, SeedableRng};

use crate::{
    interval::{AbstractInterval, MayBeFlag},
    symbolic::{eval_constraints, refine_trace, AbstractTrace, LatticeVMConstraints},
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
    prime: u32,
    rng: &mut StdRng,
    meta_info: &str,
) -> Option<AbstractTrace> {
    let mut queue: PriorityQueue<(AbstractTrace, AbstractTrace), i32> = PriorityQueue::new();
    queue.push(
        (
            initial_abs_main_trace,
            AbstractTrace::new(vec![initial_public_vals]),
        ),
        1000,
    );

    let mut num_trial = 0;
    let mut num_unsat_trial = 0;

    while !queue.is_empty() && num_trial < maximum_num_trial {
        num_trial += 1;

        let (head, potential) = queue.pop().unwrap();
        let trace = head.0;
        let public_vals = head.1;

        print!(
            "\r{}, #Trial: {}, #UNSAT Trial: {}, #Qued: {}, Potential: {}   ",
            meta_info,
            num_trial,
            num_unsat_trial,
            queue.len(),
            potential
        );
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
                    return Some(kid.0);
                }
                MayBeFlag::False => {
                    num_unsat_trial += 1;
                }
                MayBeFlag::MayBe => {
                    queue.push(kid, potential);
                }
            }
        }
    }

    None
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
) where
    FinalCheckFn: Fn(&AbstractTrace, u32),
{
    let mut rng = StdRng::seed_from_u64(seed);
    let mut found_solution_flag = false;

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
                100000,
                prime,
                &mut rng,
                &format!("{:?}", combo),
            );

            if let Some(trace) = result {
                println!("\nFind SAT assignment: {}", trace);
                final_check(&trace, prime);
                found_solution_flag = true;
                break;
            }
        }

        if found_solution_flag {
            break;
        }
    }
}
