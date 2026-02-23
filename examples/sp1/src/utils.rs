use std::collections::HashSet;

use p3_air::Air;
use p3_field::PrimeField32;
use p3_uni_stark::SymbolicAirBuilder;
use p3_uni_stark::{get_symbolic_constraints, SymbolicExpression};

use sp1_core_executor::{Executor, Program};
use sp1_core_machine::{riscv::RiscvAir, utils::trace_checkpoint, utils::SP1CoreProverError};
use sp1_stark::{
    air::SP1_PROOF_NUM_PV_ELTS, baby_bear_poseidon2::BabyBearPoseidon2, CpuProver,
    InteractionBuilder, MachineProver, SP1CoreOpts,
};

use latticevm::{
    constraint::LatticeVMConstraints, interval::AbstractInterval,
    solver::prepare_constraints_and_range_type, solver::ConstraintInfo,
    symbolic::GeneralLookupInfo,
};

use crate::lookup::get_symbolic_lookup_constraints;
use crate::p3_to_tv::convert_p3_expr;

pub fn run_sp1_program(program: &Program) -> Vec<(String, Vec<Vec<AbstractInterval>>)> {
    // # Execute the Target Program
    let mut runtime = Executor::new(program.clone(), SP1CoreOpts::default());
    let (checkpoint, _pv, _done) = runtime.execute_state(false).unwrap();

    let mut checkpoint_file = tempfile::tempfile()
        .map_err(SP1CoreProverError::IoError)
        .unwrap();
    checkpoint
        .save(&mut checkpoint_file)
        .map_err(SP1CoreProverError::IoError)
        .unwrap();

    type SC = BabyBearPoseidon2;
    let config = BabyBearPoseidon2::new();
    let machine = RiscvAir::machine(config);
    let prover = CpuProver::new(machine);

    //let mut reader = io::BufReader::new(checkpoint_file);
    //let execution_state: ExecutionState =
    //    bincode::deserialize_from(&mut reader).expect("failed to deserialize state");
    let (records, _report) = trace_checkpoint::<SC>(
        program.clone(),
        &checkpoint_file,
        SP1CoreOpts::default(),
        None,
    );
    let mut main_traces = records
        .iter()
        .map(|record| prover.generate_traces(record))
        .collect::<Vec<_>>();

    let mut true_abs_traces = vec![];
    for mt in &mut main_traces[0] {
        let nrows = mt.1.values.len() / mt.1.width;
        let mut rows = vec![];
        for i in 0..nrows {
            let row = mt.1.row_mut(i);
            rows.push(
                row.iter()
                    .map(|v| AbstractInterval::from_i64(v.as_canonical_u32() as i64))
                    .collect(),
            );
        }
        true_abs_traces.push((mt.0.clone(), rows));
    }

    true_abs_traces
}

pub fn get_program_str(program: &Program) -> String {
    program
        .instructions
        .iter()
        .map(|inst| format!("{:?}\n", inst))
        .collect::<String>()
}

pub fn generate_abstract_trace(
    program: &Program,
    key: String,
    num_extracted_rows: usize,
) -> Vec<Vec<AbstractInterval>> {
    let true_abstract_traces = run_sp1_program(&program);
    let mut base_abs_main_trace_data = vec![];
    for st in &true_abstract_traces {
        if st.0 == key {
            base_abs_main_trace_data = st.1[..num_extracted_rows].to_vec();
        }
    }

    base_abs_main_trace_data
}

pub fn extract_constraints_and_range<F, A>(
    air: &A,
    num_cols: usize,
    prime: u32,
) -> (ConstraintInfo, GeneralLookupInfo)
where
    F: p3_field::PrimeField32,
    A: Air<InteractionBuilder<F>> + Air<SymbolicAirBuilder<F>>,
{
    let mut u8_cols = vec![];
    let mut u16_cols = vec![];
    let mut multiplicities = HashSet::new();
    let mut lookup_constraints = Vec::new();
    let mut received_vars_from_cpu = HashSet::new();

    let symbolic_constraints: Vec<SymbolicExpression<F>> =
        get_symbolic_constraints(air, 0, SP1_PROOF_NUM_PV_ELTS);
    let mut air_constraints = symbolic_constraints
        .iter()
        .map(|sc| convert_p3_expr::<F>(&sc))
        .collect::<Vec<_>>();

    let general_lookup_info = get_symbolic_lookup_constraints::<F, A>(
        air,
        0,
        SP1_PROOF_NUM_PV_ELTS,
        &mut u8_cols,
        &mut u16_cols,
        &mut multiplicities,
        &mut air_constraints,
        &mut lookup_constraints,
        &mut received_vars_from_cpu,
        prime,
    );

    let (refinable_cols, range_types) = prepare_constraints_and_range_type(
        num_cols,
        &u8_cols,
        &u16_cols,
        &multiplicities,
        &received_vars_from_cpu,
        &mut air_constraints,
        &lookup_constraints,
        prime,
    );

    let constraints = LatticeVMConstraints::new(air_constraints, lookup_constraints);
    let constraint_info = ConstraintInfo {
        constraints: constraints,
        num_total_columns: num_cols,
        num_pv_columns: 0,
        output_columns: vec![],
        refinable_cols: refinable_cols,
        range_types: range_types,
        prime: prime,
    };

    (constraint_info, general_lookup_info)
}
