use std::collections::HashMap;
use std::collections::HashSet;

use itertools::Itertools;

use p3_air::Air;
use p3_uni_stark::SymbolicExpression;

use pico_vm::chips::chips::public_values::columns::NUM_PUBLIC_VALUES_COLS;
use pico_vm::compiler::riscv::program::Program;
use pico_vm::machine::folder::SymbolicConstraintFolder;
use pico_vm::machine::utils::get_symbolic_constraints;

use latticevm::solver::prepare_constraints_and_range_type;
use latticevm::solver::RangeType;
use latticevm::symbolic::gather_vars;
use latticevm::symbolic::LatticeVMSymbolicExpr;
use latticevm::symbolic::{is_iszero_operator, is_koalabear_word_range};
use latticevm::utils::GeneralLookupInfo;
use latticevm::{interval::AbstractInterval, symbolic::gather_boolean_variables};

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
) -> (
    Vec<LatticeVMSymbolicExpr>,
    Vec<LatticeVMSymbolicExpr>,
    Vec<usize>,
    HashMap<usize, RangeType>,
    GeneralLookupInfo,
)
where
    F: p3_field::PrimeField32,
    A: Air<SymbolicConstraintFolder<F>>,
{
    let mut u8_cols = vec![];
    let mut multiplicities = HashSet::new();
    let mut lookup_symbolic_constraints = Vec::new();
    let mut received_vars_from_cpu = HashSet::new();

    let symbolic_constraints: Vec<SymbolicExpression<F>> = get_symbolic_constraints(air, 0);
    let general_lookup_info = get_symbolic_lookup_constraints::<F, A>(
        air,
        0,
        NUM_PUBLIC_VALUES_COLS,
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
        &lookup_symbolic_constraints,
        prime,
    );

    (
        tv_constraints,
        lookup_symbolic_constraints,
        refinable_cols,
        range_types,
        general_lookup_info,
    )
}
