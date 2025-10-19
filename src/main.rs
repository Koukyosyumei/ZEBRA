use p3_mersenne_31::Mersenne31;
use p3_uni_stark::{get_symbolic_constraints, SymbolicExpression};

use latticevm::{
    interval::AbstractInterval, p3_to_tv::convert_p3_expr, solver::solve, symbolic::AbstractTrace,
};
use rand::{rngs::StdRng, SeedableRng};
use zkm_core_machine::CpuChip;
use zkm_stark::ZKM_PROOF_NUM_PV_ELTS;

fn main() -> Result<(), ()> {
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1; //2_u32.pow(31) - 1;
    let mut rng = StdRng::seed_from_u64(42);

    let air = CpuChip::default();
    let symbolic_constraints: Vec<SymbolicExpression<Mersenne31>> =
        get_symbolic_constraints(&air, 0, ZKM_PROOF_NUM_PV_ELTS);
    println!("#symbolic_constraints: {}", symbolic_constraints.len());

    let mut tv_constraints = vec![];
    for sc in symbolic_constraints {
        println!("{} = 0", convert_p3_expr::<Mersenne31>(&sc));
        tv_constraints.push(convert_p3_expr::<Mersenne31>(&sc));
    }

    let rows = vec![vec![
        1, 0, 0, 0, 0, 0, 4, 8, 0, 29, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 5, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0,
        0, 0, 0, 0, 0, 0, 1, 0,
    ]];
    let mut abs_main_trace_data = rows
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|v| AbstractInterval { lo: v, hi: v })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for i in 0..abs_main_trace_data.len() {
        abs_main_trace_data[i][5] = AbstractInterval::u8();
        abs_main_trace_data[i][6] = AbstractInterval::u8();
        abs_main_trace_data[i][7] = AbstractInterval::u8();
    }

    //let mut abs_main_trace_data =
    //    vec![vec![AbstractInterval::zero(); ZKM_PROOF_NUM_PV_ELTS]; num_steps];
    let abs_main_trace = AbstractTrace::new(abs_main_trace_data);

    let mut public_vals = vec![AbstractInterval::zero(); ZKM_PROOF_NUM_PV_ELTS];
    public_vals[44] = AbstractInterval::one();

    let result = solve(
        abs_main_trace,
        public_vals,
        &tv_constraints,
        1,
        prime,
        &mut rng,
    );
    if let Some(trace) = result {
        println!("Find SAT assignment: {}", trace);
    } else {
        println!("Couln't Find SAT assignment");
    }

    Ok(())
}
