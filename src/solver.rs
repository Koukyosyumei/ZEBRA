use std::collections::VecDeque;

use rand::rngs::StdRng;

use crate::{
    interval::{AbstractInterval, MayBeFlag},
    symbolic::{eval_air_constraints, refine_trace, AbstractTrace, LatticeVMSymbolicExpr},
};

pub fn solve(
    initial_abs_main_trace: AbstractTrace,
    initial_public_vals: Vec<AbstractInterval>,
    tv_constraints: &[LatticeVMSymbolicExpr],
    num_refined_points: usize,
    prime: u32,
    rng: &mut StdRng,
) -> Option<AbstractTrace> {
    let mut deque: VecDeque<AbstractTrace> = VecDeque::new();
    deque.push_back(initial_abs_main_trace);

    let mut public_vals = initial_public_vals;

    while !deque.is_empty() {
        let mut trace = deque.pop_front().unwrap();

        {
            public_vals[40] = trace.data[0][5].clone();
            //public_vals[41] = trace.data[trace.data.len() - 1][6].clone();
            for i in 0..trace.data.len() {
                trace.data[i][0] = public_vals[44].clone();
            }
        }

        let flag = eval_air_constraints(&trace, Some(&public_vals), tv_constraints, prime);
        if flag == MayBeFlag::True {
            return Some(trace);
        } else if flag == MayBeFlag::MayBe {
            let children = refine_trace(&trace, num_refined_points, rng);
            if let Some(children) = children {
                deque.push_back(children.0);
                deque.push_back(children.1);
            }
        } else {
            // println!("UNSAT: {}", trace);
        }
    }

    None
}
