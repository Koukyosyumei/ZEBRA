use std::collections::HashSet;

use p3_air::Air;
use p3_uni_stark::SymbolicExpression;

use pico_vm::chips::chips::public_values::columns::NUM_PUBLIC_VALUES_COLS;
use pico_vm::compiler::riscv::program::Program;
use pico_vm::machine::folder::SymbolicConstraintFolder;
use pico_vm::machine::utils::get_symbolic_constraints;

use latticevm::interval::AbstractInterval;
use latticevm::quick::ConstraintInfo;
use latticevm::solver::prepare_constraints_and_range_type;
use latticevm::symbolic::LatticeVMConstraints;
use latticevm::utils::GeneralLookupInfo;

use crate::executor::run_pico_program;
use crate::lookup::get_symbolic_lookup_constraints;
use crate::p3_to_tv::convert_p3_expr;

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
