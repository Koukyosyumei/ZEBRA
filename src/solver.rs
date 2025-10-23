use std::collections::VecDeque;

use rand::rngs::StdRng;

use crate::{
    interval::{AbstractInterval, MayBeFlag},
    symbolic::{
        eval_air_constraints, eval_constraints, refine_trace, AbstractTrace, LatticeVMConstraints,
        LatticeVMSymbolicExpr,
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
) -> Option<AbstractTrace> {
    let mut deque: VecDeque<(AbstractTrace, AbstractTrace)> = VecDeque::new();
    deque.push_back((
        initial_abs_main_trace,
        AbstractTrace::new(vec![initial_public_vals]),
    ));

    while !deque.is_empty() {
        let head = deque.pop_front().unwrap();
        let trace = head.0;
        let mut public_vals = head.1;

        //public_vals.data[0][40] = trace.data[0][5].clone();
        //public_vals.data[0][41] = trace.data[0][6].clone();

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
            println!("UNSAT: ({}, {}), {}", trace, public_vals, deque.len());
        }
    }

    None
}
