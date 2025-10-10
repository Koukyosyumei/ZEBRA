use std::fmt::Debug;
use std::marker::PhantomData;

use p3_air::{Air, AirBuilder, BaseAir};
use p3_field::{Field, PrimeCharacteristicRing};
use p3_matrix::dense::RowMajorMatrix;
use p3_matrix::Matrix;

use p3_challenger::{HashChallenger, SerializingChallenger32};
use p3_circle::CirclePcs;
use p3_commit::ExtensionMmcs;
use p3_field::extension::BinomialExtensionField;
use p3_fri::FriFoldingStrategy;
use p3_keccak::Keccak256Hash;
use p3_merkle_tree::MerkleTreeMmcs;
use p3_mersenne_31::Mersenne31;
use p3_symmetric::{CompressionFunctionFromHasher, SerializingHasher};
use p3_uni_stark::{
    get_symbolic_constraints, prove, verify, StarkConfig, SymbolicAirBuilder, SymbolicExpression,
    SymbolicVariable,
};
use twinvm::{interval::AbstractInterval, p3_to_tv::convert_p3_expr};

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
        let final_value = AB::Expr::from_u32(self.final_value);
        builder
            .when_last_row()
            .assert_eq(local[1].clone(), final_value);
    }
}

fn main() -> Result<(), ()> {
    type Val = Mersenne31;

    let num_steps = 8; // Choose the number of Fibonacci steps
    let final_value = 21; // Choose the final Fibonacci value
    let air = FibonacciAir {
        num_steps,
        final_value,
    };
    let symbolic_constraints: Vec<SymbolicExpression<Val>> = get_symbolic_constraints(&air, 0, 0);
    println!("#symbolic_constraints: {}", symbolic_constraints.len());
    for sc in symbolic_constraints {
        //println!("{:?}", sc);
        println!("{:?}", convert_p3_expr::<Val>(&sc));
    }

    Ok(())
}
