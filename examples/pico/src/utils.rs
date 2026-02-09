use std::collections::HashSet;
use std::sync::Arc;

use p3_air::Air;
use p3_field::PrimeField32;
use p3_koala_bear::KoalaBear;
use p3_uni_stark::SymbolicExpression;

use pico_vm::{
    chips::chips::public_values::columns::NUM_PUBLIC_VALUES_COLS,
    compiler::riscv::program::Program,
    configs::config::StarkGenericConfig,
    emulator::opts::EmulatorOpts,
    emulator::riscv::emulator::RiscvEmulator,
    instances::{
        chiptype::riscv_chiptype::RiscvChipType,
        configs::embed_kb_bn254_poseidon2::KoalaBearBn254Poseidon2, machine::riscv::RiscvMachine,
    },
    iter::PicoIterator,
    machine::{
        chip::ChipBehavior, folder::SymbolicConstraintFolder, machine::MachineBehavior,
        utils::get_symbolic_constraints,
    },
    primitives::consts::RISCV_NUM_PVS,
};

use latticevm::interval::AbstractInterval;
use latticevm::quick::ConstraintInfo;
use latticevm::solver::prepare_constraints_and_range_type;
use latticevm::symbolic::LatticeVMConstraints;
use latticevm::utils::GeneralLookupInfo;

use crate::lookup::get_symbolic_lookup_constraints;
use crate::p3_to_tv::convert_p3_expr;

pub fn run_pico_program(program: &Program) -> Vec<(String, Vec<Vec<AbstractInterval>>)> {
    let config = KoalaBearBn254Poseidon2::new();
    let riscv_machine = RiscvMachine::new(config, RiscvChipType::all_chips(), RISCV_NUM_PVS);

    let mut runtime = RiscvEmulator::new_single::<KoalaBear>(
        Arc::new(program.clone()),
        EmulatorOpts::default(),
        None,
    );
    // runtime.state.input_stream.push(vec![2, 0, 0, 0]);
    let batch_records = runtime.run(None).unwrap().0;
    let chunk = &batch_records[0];

    let mut chips_and_main_traces = riscv_machine
        .base_machine()
        .prover
        .generate_main(&riscv_machine.chips(), chunk);

    let mut true_abs_traces = vec![];
    for mt in &mut chips_and_main_traces {
        let nrows = mt.1.values.len() / mt.1.width;
        let mut rows: Vec<_> = vec![];
        for i in 0..nrows {
            let row = mt.1.row_mut(i);
            rows.push(
                row.iter()
                    .map(|v| AbstractInterval::from_i64(v.as_canonical_u32() as i64))
                    .collect::<Vec<_>>(),
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
    let true_abstract_traces = run_pico_program(&program);
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
    A: Air<SymbolicConstraintFolder<F>>,
{
    let mut u8_cols = vec![];
    let mut multiplicities = HashSet::new();
    let mut lookup_constraints = Vec::new();
    let mut received_vars_from_cpu = HashSet::new();

    let symbolic_constraints: Vec<SymbolicExpression<F>> = get_symbolic_constraints(air, 0);
    let general_lookup_info = get_symbolic_lookup_constraints::<F, A>(
        air,
        0,
        NUM_PUBLIC_VALUES_COLS,
        &mut u8_cols,
        &mut multiplicities,
        &mut lookup_constraints,
        &mut received_vars_from_cpu,
        prime,
    );

    let mut air_constraints = symbolic_constraints
        .iter()
        .map(|sc| convert_p3_expr::<F>(&sc))
        .collect::<Vec<_>>();

    let (refinable_cols, range_types) = prepare_constraints_and_range_type(
        num_cols,
        &u8_cols,
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
