use std::collections::VecDeque;

use rand::rngs::StdRng;

use crate::{
    interval::{AbstractInterval, MayBeFlag},
    symbolic::{eval_air_constraints, refine_trace, AbstractTrace, LatticeVMSymbolicExpr},
};

pub fn solve(
    tv_constraints: &[LatticeVMSymbolicExpr],
    num_columns: usize,
    num_steps: usize,
    num_public_vals: usize,
    prime: u32,
    rng: &mut StdRng,
) -> Option<AbstractTrace> {
    let mut deque: VecDeque<AbstractTrace> = VecDeque::new();
    let abs_main_trace_data = vec![vec![AbstractInterval::bool(); num_columns]; num_steps];
    let abs_main_trace = AbstractTrace::new(abs_main_trace_data);
    deque.push_back(abs_main_trace);

    let public_vals = vec![AbstractInterval::zero(); num_public_vals];

    while !deque.is_empty() {
        let trace = deque.pop_front().unwrap();
        let flag = eval_air_constraints(&trace, Some(&public_vals), tv_constraints, prime);
        if flag == MayBeFlag::True {
            return Some(trace);
        } else if flag == MayBeFlag::MayBe {
            let children = refine_trace(&trace, 32, rng);
            if let Some(children) = children {
                deque.push_back(children.0);
                deque.push_back(children.1);
            }
        } else {
            //println!("UNSAT: {}", trace);
        }
    }

    None
}
