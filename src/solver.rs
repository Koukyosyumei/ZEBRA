use std::collections::VecDeque;

use rand::rngs::StdRng;

use crate::{
    interval::{AbstractInterval, MayBeFlag},
    symbolic::{eval_air_constraints, refine_trace, AbstractTrace, LatticeVMSymbolicExpr},
};

/*
(curr[19] * (curr[48] - curr[10]))
(curr[19] * (curr[49] - curr[11]))
(curr[19] * (curr[50] - curr[12]))
(curr[19] * (curr[51] - curr[13]))
(curr[20] * (curr[57] - curr[14]))
(curr[20] * (curr[58] - curr[15]))
(curr[20] * (curr[59] - curr[16]))
(curr[20] * (curr[60] - curr[17]))
((1 - curr[19]) * ((1 - curr[19]) - 1))
((1 - curr[19]) * (curr[54] * (curr[54] - 1)))
((1 - curr[19]) * (curr[54] * (curr[0] - curr[52])))
((1 - curr[19]) * (((((curr[54] * (((65536 * curr[2]) + curr[1]) + 2)) + ((1 - curr[54]) * curr[0])) - ((curr[54] * curr[53]) + ((1 - curr[54]) * curr[52]))) - 1) - (curr[55] + (curr[56] * 65536))))
((1 - curr[20]) * ((1 - curr[20]) - 1))
((1 - curr[20]) * (curr[63] * (curr[63] - 1)))
((1 - curr[20]) * (curr[63] * (curr[0] - curr[61])))
((1 - curr[20]) * (((((curr[63] * (((65536 * curr[2]) + curr[1]) + 1)) + ((1 - curr[63]) * curr[0])) - ((curr[63] * curr[62]) + ((1 - curr[63]) * curr[61]))) - 1) - (curr[64] + (curr[65] * 65536))))
(curr[18] * curr[39])
(curr[18] * curr[40])
(curr[18] * curr[41])
(curr[18] * curr[42])
((curr[18] - 1) * (curr[27] - curr[39]))
((curr[18] - 1) * (curr[28] - curr[40]))
((curr[18] - 1) * (curr[29] - curr[41]))
((curr[18] - 1) * (curr[30] - curr[42]))
(curr[23] * (curr[31] - curr[35]))
(curr[23] * (curr[32] - curr[36]))
(curr[23] * (curr[33] - curr[37]))
(curr[23] * (curr[34] - curr[38]))
(curr[66] * (curr[66] - 1))
(curr[66] * (curr[45] * (curr[45] - 1)))
(curr[66] * (curr[45] * (curr[0] - curr[43])))
(curr[66] * (((((curr[45] * (((65536 * curr[2]) + curr[1]) + 3)) + ((1 - curr[45]) * curr[0])) - ((curr[45] * curr[44]) + ((1 - curr[45]) * curr[43]))) - 1) - (curr[46] + (curr[47] * 65536))))
(curr[67] * (curr[39] - curr[35]))
(curr[67] * (curr[40] - curr[36]))
(curr[67] * (curr[41] - curr[37]))
(curr[67] * (curr[42] - curr[38]))
(curr[66] * (curr[3] - ((((curr[22] + curr[23]) + curr[24]) * curr[0]) + ((1 - ((curr[22] + curr[23]) + curr[24])) * 0))))
(curr[66] * (curr[4] - ((((curr[22] + curr[23]) + curr[24]) * ((65536 * curr[2]) + curr[1])) + ((1 - ((curr[22] + curr[23]) + curr[24])) * 0))))
(IsTransition * (next[66] * (curr[0] - next[0])))
(IsFirstRow * ((65536 * curr[2]) + curr[1]))
(IsTransition * (next[66] * (((((65536 * curr[2]) + curr[1]) + 5) + curr[21]) - ((65536 * next[2]) + next[1]))))
(curr[66] * (((65536 * curr[2]) + curr[1]) - (curr[1] + (curr[2] * 65536))))
(curr[66] * (public[44] - curr[0]))
(IsFirstRow * (public[40] - curr[5]))
(IsFirstRow * ((curr[25] - 1) * ((curr[5] + 4) - curr[6])))
(IsTransition * (next[66] * (curr[6] - next[5])))
(IsTransition * (next[66] * ((next[25] - 1) * (curr[7] - next[6]))))
(IsTransition * (curr[66] * (curr[26] * (curr[7] - (curr[6] + 4)))))
(IsTransition * ((curr[66] - next[66]) * (public[41] - curr[6])))
(IsLastRow * (curr[66] * (public[41] - curr[6])))
(curr[66] * (curr[66] - 1))
(IsFirstRow * (curr[66] - 1))
(IsTransition * ((curr[66] - 1) * next[66]))
(IsTransition * (curr[25] * next[66]))
((1 - curr[66]) * (1 - curr[19]))
((1 - curr[66]) * (1 - curr[20]))
((1 - curr[66]) * (1 - curr[23]))
*/

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

    while !deque.is_empty() {
        let trace = deque.pop_front().unwrap();
        let flag = eval_air_constraints(&trace, Some(&initial_public_vals), tv_constraints, prime);
        if flag == MayBeFlag::True {
            return Some(trace);
        } else if flag == MayBeFlag::MayBe {
            let children = refine_trace(&trace, num_refined_points, rng);
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
