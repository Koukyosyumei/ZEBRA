use std::collections::VecDeque;

use rand::rngs::StdRng;

use crate::{
    interval::{AbstractInterval, MayBeFlag},
    symbolic::{
        eval_constraints, refine_trace, AbstractTrace, LatticeVMConstraints,
    },
};

pub fn solve(
    initial_abs_main_trace: AbstractTrace,
    initial_public_vals: Vec<AbstractInterval>,
    constraints: &LatticeVMConstraints,
    num_refined_points: usize,
    refinment_target_indicies_main: &Vec<usize>,
    refinment_target_indicies_pv: &Vec<usize>,
    prime: u32,
    rng: &mut StdRng,
    meta_info: &str,
) -> Option<AbstractTrace> {
    let mut deque: VecDeque<(AbstractTrace, AbstractTrace)> = VecDeque::new();
    deque.push_back((
        initial_abs_main_trace,
        AbstractTrace::new(vec![initial_public_vals]),
    ));

    let mut num_trial = 0;
    let mut num_unsat_trial = 0;

    while !deque.is_empty() && deque.len() < 100000 {
        num_trial += 1;

        let head = deque.pop_front().unwrap();
        let trace = head.0;
        let public_vals = head.1;

        let flag = eval_constraints(&trace, Some(&public_vals.data[0]), constraints, prime);
        if flag == MayBeFlag::True {
            return Some(trace);
        } else if flag == MayBeFlag::MayBe {
            let trace_children = refine_trace(
                &trace,
                num_refined_points,
                refinment_target_indicies_main,
                rng,
            );

            let pv_children = refine_trace(
                &public_vals,
                num_refined_points,
                refinment_target_indicies_pv,
                rng,
            );

            if let Some(trace_children) = trace_children {
                if let Some(pv_children) = pv_children {
                    deque.push_back((trace_children.0.clone(), pv_children.0.clone()));
                    deque.push_back((trace_children.1.clone(), pv_children.0));
                    deque.push_back((trace_children.0, pv_children.1.clone()));
                    deque.push_back((trace_children.1, pv_children.1));
                } else {
                    deque.push_back((trace_children.0, public_vals.clone()));
                    deque.push_back((trace_children.1, public_vals));
                }
            } else if let Some(pv_children) = pv_children {
                deque.push_back((trace.clone(), pv_children.0));
                deque.push_back((trace, pv_children.1));
            }
        } else {
            num_unsat_trial += 1;
        }

        print!(
            "\r{}, #Trial: {}, #UNSAT Trial: {}, #Qued: {}",
            meta_info,
            num_trial,
            num_unsat_trial,
            deque.len()
        );
    }

    None
}
