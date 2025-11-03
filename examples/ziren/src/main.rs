use std::rc::Rc;

use itertools::Itertools;
use rand::{rngs::StdRng, SeedableRng};

use p3_mersenne_31::Mersenne31;
use p3_uni_stark::{get_symbolic_constraints, SymbolicExpression};

use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::{cpu::columns::NUM_CPU_COLS, CpuChip};
use zkm_stark::MachineProver;
use zkm_stark::ZKM_PROOF_NUM_PV_ELTS;

use latticevm::symbolic::eval_constraints;
use latticevm::symbolic::LatticeVMSymbolicEntry;
use latticevm::symbolic::LatticeVMSymbolicExpr;
use latticevm::symbolic::LatticeVMSymbolicVal;
use latticevm::{
    interval::AbstractInterval, solver::run_solver, symbolic::gather_boolean_variables,
    symbolic::AbstractTrace, symbolic::LatticeVMConstraints,
};

use latticevm_ziren::executor::run_ziren_program;
use latticevm_ziren::p3_to_tv::convert_p3_expr;
use latticevm_ziren::pv_constraints::get_pv_constraints;
use latticevm_ziren::state::ziren_abstract_trace_to_abstract_state;

pub fn add_program(pc_start: u32, pc_base: u32) -> Program {
    let instructions = vec![Instruction::new(Opcode::ADD, 1, 0, 3, false, true)];
    Program::new(instructions, pc_start, pc_base)
}

fn main() -> Result<(), ()> {
    // ############### Global Parameters #################################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;
    let program_cols = (8..20).collect::<Vec<_>>();

    // ############### Gather Constraints ################################
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

    // ############### Construct SMT formula ############################
    //let smt = expr_to_smt(&constraints, 1, 68, prime);
    //println!("{}", smt);

    // ############### Preparation of Solver ############################
    let mut rng = StdRng::seed_from_u64(42);
    let max_row_id = 0;
    let num_extracted_rows = 2;
    let potential_boolean_vars = gather_boolean_variables(&tv_constraints);

    // ############### Prepare Public Values ############################
    let mut public_vals = vec![AbstractInterval::zero(); ZKM_PROOF_NUM_PV_ELTS];
    public_vals[40] = AbstractInterval::i4();
    public_vals[41] = AbstractInterval::i4();
    public_vals[44] = AbstractInterval::one();
    let refinment_target_indicies_pv: Vec<usize> = vec![40, 41];

    // ############### Target Program ###################################
    let program = add_program(4, 4);
    let (true_abstract_states, true_abstract_traces) = run_ziren_program(&program);

    let mut base_abs_main_trace_data = vec![];
    for st in &true_abstract_traces {
        if st.0 == "Cpu" {
            base_abs_main_trace_data = st.1[..num_extracted_rows].to_vec();
        }
    }

    // ############## Final Check Function ##############################
    fn final_check(trace: &AbstractTrace, prime: u32) {
        let recovered_states = trace
            .data
            .iter()
            .map(|row| ziren_abstract_trace_to_abstract_state(row, prime))
            .collect::<Vec<_>>();
        for rs in &recovered_states {
            println!("{}", rs);
        }
        println!("========");

        let program = add_program(
            recovered_states[0].pc.as_canonical_u32(prime),
            recovered_states[0].pc.as_canonical_u32(prime),
        );
        let (true_abstract_states, _true_abstract_traces) = run_ziren_program(&program);
        for tas in &true_abstract_states {
            println!("{}", tas);
        }
    }

    // ############## Solve! ###########################################
    let mut target_cols = (0..NUM_CPU_COLS).collect::<Vec<_>>();
    target_cols.retain(|x| !program_cols.contains(x));

    run_solver(
        &constraints,
        &target_cols,
        &potential_boolean_vars,
        &refinment_target_indicies_pv,
        &base_abs_main_trace_data,
        public_vals,
        max_row_id,
        final_check,
        prime,
        42,
    );

    Ok(())
}
