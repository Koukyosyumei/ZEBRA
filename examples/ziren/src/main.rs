use std::rc::Rc;

use itertools::Itertools;
use rand::{rngs::StdRng, SeedableRng};

use p3_mersenne_31::Mersenne31;
use p3_uni_stark::{get_symbolic_constraints, SymbolicExpression};

use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::CpuChip;
use zkm_stark::MachineProver;
use zkm_stark::ZKM_PROOF_NUM_PV_ELTS;

use latticevm::symbolic::LatticeVMSymbolicEntry;
use latticevm::symbolic::LatticeVMSymbolicExpr;
use latticevm::symbolic::LatticeVMSymbolicVal;
use latticevm::{
    interval::AbstractInterval, solver::solve, symbolic::gather_boolean_variables,
    symbolic::AbstractTrace, symbolic::LatticeVMConstraints,
};

use latticevm_ziren::p3_to_tv::convert_p3_expr;
use latticevm_ziren::pv_constraints::get_pv_constraints;

pub fn add_program(pc_start: u32, pc_base: u32) -> Program {
    let instructions = vec![Instruction::new(Opcode::ADD, 1, 0, 3, false, true)];
    Program::new(instructions, pc_start, pc_base)
}

fn main() -> Result<(), ()> {
    // ############### Global Parameters ###############
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;
    let program_cols = (8..20).collect::<Vec<_>>();

    // # Gather Constraints
    let air = CpuChip::default();
    let symbolic_constraints: Vec<SymbolicExpression<Mersenne31>> =
        get_symbolic_constraints(&air, 0, ZKM_PROOF_NUM_PV_ELTS);
    let tv_constraints = symbolic_constraints
        .iter()
        .map(|sc| convert_p3_expr::<Mersenne31>(&sc))
        .collect::<Vec<_>>();

    // # Additional Public Value Verification
    let (pv_pos_constraints, pv_neg_constraints) = get_pv_constraints();

    // # Gather Symbolic Constraints
    let constraints = LatticeVMConstraints {
        air_constraints: tv_constraints.clone(),
        pv_pos_constraints,
        pv_neg_constraints,
    };

    // # Target Program
    let program = add_program(4, 4);

    // # Gather Real Trace
    let real_rows = vec![vec![
        1, 0, 0, 0, 0, 0, 4, 8, 0, 29, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 5, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0,
        0, 0, 0, 0, 0, 0, 1, 0,
    ]];
    // code location: 8 - 20

    // # Construct SMT formula
    //let smt = expr_to_smt(&constraints, 1, 68, prime);
    //println!("{}", smt);

    let mut rng = StdRng::seed_from_u64(42);
    let mut found_solution_flag = false;
    let potential_boolean_vars = gather_boolean_variables(&tv_constraints);

    let mut target_cols = (0..68).collect::<Vec<_>>();
    target_cols.retain(|x| !program_cols.contains(x));
    for k in 1..(target_cols.len() + 1) {
        for combo in target_cols.iter().combinations(k) {
            //println!("{:?}", combo);
            let mut abs_main_trace_data = real_rows
                .clone()
                .into_iter()
                .map(|row| {
                    row.into_iter()
                        .map(|v| AbstractInterval { lo: v, hi: v })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();

            for i in 0..abs_main_trace_data.len() {
                for c in &combo {
                    if potential_boolean_vars.contains(c) {
                        abs_main_trace_data[i][**c] = AbstractInterval::bool();
                    } else {
                        abs_main_trace_data[i][**c] = AbstractInterval::i4();
                    }
                }
            }
            let abs_main_trace = AbstractTrace::new(abs_main_trace_data);

            let mut public_vals = vec![AbstractInterval::zero(); ZKM_PROOF_NUM_PV_ELTS];
            public_vals[40] = AbstractInterval::i4();
            public_vals[41] = AbstractInterval::i4();
            public_vals[44] = AbstractInterval::one();

            let refinment_target_indicies_main = combo.clone().into_iter().cloned().collect();
            let refinment_target_indicies_pv: Vec<usize> = vec![40, 41];

            let result = solve(
                abs_main_trace,
                public_vals.clone(),
                &constraints,
                1,
                &refinment_target_indicies_main,
                &refinment_target_indicies_pv,
                0,
                prime,
                &mut rng,
                &format!("{:?}", combo),
            );

            if let Some(trace) = result {
                println!("\nFind SAT assignment: {}", trace);
                found_solution_flag = true;
                break;
            } else {
                //println!("\nCouln't Find SAT assignment");
            }
        }

        if found_solution_flag {
            break;
        }
    }

    Ok(())
}
