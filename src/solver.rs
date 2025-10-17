use std::collections::VecDeque;

use p3_air::{Air, AirBuilder, BaseAir};
use p3_field::{Field, PrimeCharacteristicRing};
use p3_matrix::Matrix;
use p3_mersenne_31::Mersenne31;
use p3_uni_stark::{get_symbolic_constraints, SymbolicExpression};

use rand::{rngs::StdRng, SeedableRng};

use crate::{
    interval::{AbstractInterval, MayBeFlag},
    p3_to_tv::convert_p3_expr,
    symbolic::{eval_air_constraints, refine_trace, AbstractTrace, LatticeVMSymbolicExpr},
};

pub fn solve(
    tv_constraints: &[LatticeVMSymbolicExpr],
    num_steps: usize,
    prime: u32,
    rng: &mut StdRng,
) -> Option<AbstractTrace> {
    let mut deque: VecDeque<AbstractTrace> = VecDeque::new();
    let abs_main_trace_data = vec![vec![AbstractInterval::u8(); 2]; num_steps];
    let abs_main_trace = AbstractTrace::new(abs_main_trace_data);
    deque.push_back(abs_main_trace);

    while !deque.is_empty() {
        let trace = deque.pop_front().unwrap();
        let flag = eval_air_constraints(&trace, tv_constraints, prime);
        if flag == MayBeFlag::True {
            return Some(trace);
        } else if flag == MayBeFlag::MayBe {
            let children = refine_trace(&trace, 1, rng);
            if let Some(children) = children {
                deque.push_back(children.0);
                deque.push_back(children.1);
            }
        } else {
            //println!("UNSAT: {:?}", trace);
        }
    }

    None
}
