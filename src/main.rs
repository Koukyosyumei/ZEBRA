use p3_mersenne_31::Mersenne31;
use p3_uni_stark::{get_symbolic_constraints, SymbolicExpression};

use latticevm::{p3_to_tv::convert_p3_expr, solver::solve, test_data::FibonacciAir};
use rand::{rngs::StdRng, SeedableRng};
use zkm_core_machine::CpuChip;
use zkm_stark::ZKM_PROOF_NUM_PV_ELTS;

fn main() -> Result<(), ()> {
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1; //2_u32.pow(31) - 1;

    let mut rng = StdRng::seed_from_u64(42);

    let num_steps = 2; // Choose the number of Fibonacci steps
                       //let final_value = 21; // Choose the final Fibonacci value
                       /*
                       let air = FibonacciAir {
                           num_steps,
                           final_value,
                       };
                       */
    let air = CpuChip::default();
    let symbolic_constraints: Vec<SymbolicExpression<Mersenne31>> =
        get_symbolic_constraints(&air, 0, ZKM_PROOF_NUM_PV_ELTS);
    println!("#symbolic_constraints: {}", symbolic_constraints.len());

    let mut tv_constraints = vec![];
    for sc in symbolic_constraints {
        println!("{}", convert_p3_expr::<Mersenne31>(&sc));
        tv_constraints.push(convert_p3_expr::<Mersenne31>(&sc));
    }

    let result = solve(
        &tv_constraints,
        ZKM_PROOF_NUM_PV_ELTS,
        num_steps,
        ZKM_PROOF_NUM_PV_ELTS,
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
