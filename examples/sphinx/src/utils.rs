use std::collections::HashSet;

use p3_air::Air;
use p3_baby_bear::BabyBear;
use p3_field::PrimeField32;
use p3_uni_stark::SymbolicAirBuilder;
use p3_uni_stark::{get_symbolic_constraints, SymbolicExpression};

use sphinx_core::air::MachineAir;
use sphinx_core::alu::{AddSubChip, BitwiseChip, DivRemChip, LtChip, MulChip, ShiftLeft, ShiftRightChip};
use sphinx_core::cpu::CpuChip;
use sphinx_core::lookup::InteractionBuilder;
use sphinx_core::runtime::{ExecutionRecord, Program, Runtime};
use sphinx_core::stark::PROOF_MAX_NUM_PVS;
use sphinx_core::utils::SphinxCoreOpts;

use zebra::{
    constraint::ZEBRAConstraints, interval::AbstractInterval,
    solver::prepare_constraints_and_range_type, solver::ConstraintInfo,
    symbolic::GeneralLookupInfo,
};

use crate::lookup::get_symbolic_lookup_constraints;
use crate::p3_to_tv::convert_p3_expr;

fn trace_to_rows(trace: p3_matrix::dense::RowMajorMatrix<BabyBear>) -> Vec<Vec<AbstractInterval>> {
    let nrows = if trace.width > 0 { trace.values.len() / trace.width } else { 0 };
    (0..nrows)
        .map(|i| {
            let start = i * trace.width;
            trace.values[start..start + trace.width]
                .iter()
                .map(|v: &BabyBear| AbstractInterval::from_i128(v.as_canonical_u32() as i128))
                .collect()
        })
        .collect()
}

pub fn run_sphinx_program(program: &Program) -> Vec<(String, Vec<Vec<AbstractInterval>>)> {
    let mut runtime = Runtime::new(program.clone(), SphinxCoreOpts::default());
    let result = runtime.execute_record();
    if result.is_err() {
        return vec![];
    }
    let (record, _done) = result.unwrap();
    let mut output = ExecutionRecord::default();

    macro_rules! chip_trace {
        ($chip:ty) => {{
            let chip = <$chip>::default();
            let name = <$chip as MachineAir<BabyBear>>::name(&chip);
            let trace: p3_matrix::dense::RowMajorMatrix<BabyBear> =
                chip.generate_trace(&record, &mut output);
            (name, trace_to_rows(trace))
        }};
    }

    vec![
        chip_trace!(CpuChip),
        chip_trace!(AddSubChip),
        chip_trace!(BitwiseChip),
        chip_trace!(DivRemChip),
        chip_trace!(LtChip),
        chip_trace!(MulChip),
        chip_trace!(ShiftLeft),
        chip_trace!(ShiftRightChip),
    ]
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
    let true_abstract_traces = run_sphinx_program(&program);
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
    simplify_constraints: bool,
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
        get_symbolic_constraints(air, 0, PROOF_MAX_NUM_PVS);
    let mut air_constraints = symbolic_constraints
        .iter()
        .map(|sc| convert_p3_expr::<F>(&sc))
        .collect::<Vec<_>>();

    let general_lookup_info = get_symbolic_lookup_constraints::<F, A>(
        air,
        0,
        PROOF_MAX_NUM_PVS,
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
        simplify_constraints,
    );

    let constraints = ZEBRAConstraints::new(air_constraints, lookup_constraints);
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
