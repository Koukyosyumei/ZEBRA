use std::collections::{HashMap, HashSet};
use std::io;

use p3_air::{Air, BaseAir};
use p3_field::PrimeField32;
use p3_uni_stark::{get_symbolic_constraints, SymbolicAirBuilder, SymbolicExpression};

use zkm_core_executor::{ExecutionState, Executor, Program};
use zkm_core_machine::mips::MipsAir;
use zkm_core_machine::utils::{trace_checkpoint, ZKMCoreProverError};
use zkm_stark::koala_bear_poseidon2::KoalaBearPoseidon2;
use zkm_stark::{CpuProver, LookupBuilder, MachineProver, ZKMCoreOpts, ZKM_PROOF_NUM_PV_ELTS};

use latticevm::interval::AbstractInterval;
use latticevm::solver::{prepare_constraints_and_range_type, RangeType};
use latticevm::symbolic::LatticeVMSymbolicExpr;
use latticevm::utils::GeneralLookupInfo;

use crate::lookup::get_symbolic_lookup_constraints;
use crate::p3_to_tv::convert_p3_expr;

pub fn run_ziren_program(program: &Program) -> Vec<(String, Vec<Vec<AbstractInterval>>)> {
    // # Execute the Target Program
    let mut runtime = Executor::new(program.clone(), ZKMCoreOpts::default());
    let (checkpoint, done) = runtime.execute_state(false).unwrap();

    let mut checkpoint_file = tempfile::tempfile()
        .map_err(ZKMCoreProverError::IoError)
        .unwrap();
    checkpoint
        .save(&mut checkpoint_file)
        .map_err(ZKMCoreProverError::IoError)
        .unwrap();

    type SC = KoalaBearPoseidon2;
    let config = KoalaBearPoseidon2::new();
    let machine = MipsAir::machine(config);
    let prover = CpuProver::new(machine);

    let mut reader = io::BufReader::new(checkpoint_file);
    let execution_state: ExecutionState =
        bincode::deserialize_from(&mut reader).expect("failed to deserialize state");
    let (records, report) = trace_checkpoint::<SC>(
        program.clone(),
        execution_state,
        ZKMCoreOpts::default(),
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
    let true_abstract_traces = run_ziren_program(&program);
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
) -> (
    Vec<LatticeVMSymbolicExpr>,
    Vec<usize>,
    HashMap<usize, RangeType>,
    GeneralLookupInfo,
)
where
    F: p3_field::PrimeField32,
    A: Air<LookupBuilder<F>> + Air<SymbolicAirBuilder<F>>,
{
    let mut u8_cols = vec![];
    let mut multiplicities = HashSet::new();
    let mut lookup_symbolic_constraints = Vec::new();
    let mut received_vars_from_cpu = HashSet::new();

    let symbolic_constraints: Vec<SymbolicExpression<F>> =
        get_symbolic_constraints(air, 0, ZKM_PROOF_NUM_PV_ELTS);
    let general_lookup_info = get_symbolic_lookup_constraints::<F, A>(
        air,
        0,
        ZKM_PROOF_NUM_PV_ELTS,
        &mut u8_cols,
        &mut multiplicities,
        &mut lookup_symbolic_constraints,
        &mut received_vars_from_cpu,
        prime,
    );

    let mut tv_constraints = symbolic_constraints
        .iter()
        .map(|sc| convert_p3_expr::<F>(&sc))
        .collect::<Vec<_>>();

    let (refinable_cols, range_types) = prepare_constraints_and_range_type(
        num_cols,
        &u8_cols,
        &multiplicities,
        &received_vars_from_cpu,
        &mut tv_constraints,
        lookup_symbolic_constraints,
        prime,
    );

    (
        tv_constraints,
        refinable_cols,
        range_types,
        general_lookup_info,
    )
}
