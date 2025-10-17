use p3_air::{Air, AirBuilder, BaseAir};
use p3_field::{Field, PrimeCharacteristicRing};
use p3_matrix::Matrix;
use p3_mersenne_31::Mersenne31;
use p3_uni_stark::{get_symbolic_constraints, SymbolicExpression};

use latticevm::{
    p3_to_tv::convert_p3_expr,
    solver::solve,
};
use rand::{rngs::StdRng, SeedableRng};

pub struct FibonacciAir {
    pub num_steps: usize,
    pub final_value: u32,
}

impl<F: Field> BaseAir<F> for FibonacciAir {
    fn width(&self) -> usize {
        2 // For current and next Fibonacci number
    }
}

impl<AB: AirBuilder> Air<AB> for FibonacciAir
where
    <AB as AirBuilder>::F: Field,
{
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let local = main.row_slice(0).unwrap();
        let next = main.row_slice(1).unwrap();

        // Enforce starting values
        builder
            .when_first_row()
            .assert_eq(local[0].clone(), AB::Expr::ZERO);
        builder
            .when_first_row()
            .assert_eq(local[1].clone(), AB::Expr::ONE);

        // Enforce state transition constraints
        builder
            .when_transition()
            .assert_eq(next[0].clone(), local[1].clone());
        builder
            .when_transition()
            .assert_eq(next[1].clone(), local[0].clone() + local[1].clone());

        // Constrain the final value
        //let final_value = AB::Expr::from_u32(self.final_value);
        //builder
        //    .when_last_row()
        //    .assert_eq(local[1].clone(), final_value);
    }
}

fn main() -> Result<(), ()> {
    let prime = 2_u32.pow(31) - 1;

    let mut rng = StdRng::seed_from_u64(42);

    let num_steps = 2; // Choose the number of Fibonacci steps
    let final_value = 21; // Choose the final Fibonacci value
    let air = FibonacciAir {
        num_steps,
        final_value,
    };
    let symbolic_constraints: Vec<SymbolicExpression<Mersenne31>> =
        get_symbolic_constraints(&air, 0, 0);
    println!("#symbolic_constraints: {}", symbolic_constraints.len());

    let mut tv_constraints = vec![];
    for sc in symbolic_constraints {
        println!("{:?}", convert_p3_expr::<Mersenne31>(&sc));
        tv_constraints.push(convert_p3_expr::<Mersenne31>(&sc));
    }

    let result = solve(&tv_constraints, num_steps, prime, &mut rng);
    if let Some(trace) = result {
        println!("Find SAT assignment: {:?}", trace);
    }

    Ok(())
}
