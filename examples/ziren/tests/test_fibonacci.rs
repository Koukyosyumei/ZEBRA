use p3_air::{Air, AirBuilder, BaseAir};
use p3_field::Field;
use p3_field::FieldAlgebra;
use p3_matrix::Matrix;

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
        let local = main.row_slice(0);
        let next = main.row_slice(1);

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

#[test]
fn test_smt_fibonacci_air() {
    use latticevm::interval::{AbstractInterval, MayBeFlag};
    use latticevm::symbolic::expr_to_smt_over_trace;
    use latticevm::symbolic::{eval_air_constraints, AbstractTrace};
    use latticevm_ziren::p3_to_tv::convert_p3_expr;

    use p3_mersenne_31::Mersenne31;
    use p3_uni_stark::{get_symbolic_constraints, SymbolicExpression};

    let prime = 2_u32.pow(31) - 1;

    let num_steps = 2; // Choose the number of Fibonacci steps
    let final_value = 21; // Choose the final Fibonacci value
    let air = FibonacciAir {
        num_steps,
        final_value,
    };
    let symbolic_constraints: Vec<SymbolicExpression<Mersenne31>> =
        get_symbolic_constraints(&air, 0, 0);

    let mut tv_constraints = vec![];
    for sc in symbolic_constraints {
        tv_constraints.push(convert_p3_expr::<Mersenne31>(&sc));
    }

    let smt = expr_to_smt_over_trace(&tv_constraints, 2, 2, prime);
    println!("{}", smt);
    assert!(false);
}

#[test]
fn test_eval_fibonacci_air() {
    use latticevm::interval::{AbstractInterval, MayBeFlag};
    use latticevm::symbolic::{eval_air_constraints, AbstractTrace};
    use latticevm_ziren::p3_to_tv::convert_p3_expr;

    use p3_mersenne_31::Mersenne31;
    use p3_uni_stark::{get_symbolic_constraints, SymbolicExpression};

    let prime = 2_u32.pow(31) - 1;

    let num_steps = 2; // Choose the number of Fibonacci steps
    let final_value = 21; // Choose the final Fibonacci value
    let air = FibonacciAir {
        num_steps,
        final_value,
    };
    let symbolic_constraints: Vec<SymbolicExpression<Mersenne31>> =
        get_symbolic_constraints(&air, 0, 0);

    let mut tv_constraints = vec![];
    for sc in symbolic_constraints {
        tv_constraints.push(convert_p3_expr::<Mersenne31>(&sc));
    }

    let true_trace_data = vec![
        vec![AbstractInterval::zero(), AbstractInterval::one()],
        vec![AbstractInterval::one(), AbstractInterval::one()],
    ];
    let true_trace = AbstractTrace::new(true_trace_data);
    assert_eq!(
        eval_air_constraints(&true_trace, None, &tv_constraints, prime),
        MayBeFlag::True
    );

    let false_trace_data = vec![
        vec![
            AbstractInterval { lo: 0, hi: 7 },
            AbstractInterval { lo: 0, hi: 7 },
        ],
        vec![
            AbstractInterval { lo: 1, hi: 1 },
            AbstractInterval { lo: 0, hi: 15 },
        ],
    ];
    let false_trace = AbstractTrace::new(false_trace_data);
    assert_eq!(
        eval_air_constraints(&false_trace, None, &tv_constraints, prime),
        MayBeFlag::MayBe
    );
}
