use std::collections::HashMap;
use std::collections::HashSet;

use itertools::Itertools;

use p3_air::Air;
use p3_uni_stark::SymbolicAirBuilder;
use p3_uni_stark::{get_symbolic_constraints, SymbolicExpression};

use zkm_core_executor::Program;
use zkm_stark::LookupBuilder;
use zkm_stark::MachineProver;
use zkm_stark::ZKM_PROOF_NUM_PV_ELTS;

use latticevm::solver::RangeType;
use latticevm::symbolic::gather_vars;
use latticevm::symbolic::is_koalabear_word_range;
use latticevm::symbolic::LatticeVMSymbolicExpr;
use latticevm::{
    interval::AbstractInterval, symbolic::gather_boolean_variables, symbolic::AbstractTrace,
};

use crate::executor::run_ziren_program;
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
    let (true_abstract_states, true_abstract_traces) = run_ziren_program(&program);
    let mut base_abs_main_trace_data = vec![];
    for st in &true_abstract_traces {
        println!("st.0: {}", st.0);
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
    get_symbolic_lookup_constraints::<F, A>(
        air,
        0,
        ZKM_PROOF_NUM_PV_ELTS,
        &mut u8_cols,
        &mut multiplicities,
        &mut lookup_symbolic_constraints,
        &mut received_vars_from_cpu,
        prime,
    );

    let mut refinable_cols: Vec<usize> = (0..num_cols).collect();
    refinable_cols.retain(|c| !multiplicities.contains(c));
    refinable_cols.retain(|c| !received_vars_from_cpu.contains(c));

    let mut tv_constraints = symbolic_constraints
        .iter()
        .map(|sc| convert_p3_expr::<F>(&sc))
        .collect::<Vec<_>>();

    let mut new_tv_constraints = Vec::new();
    let mut is_in_koalabear_word_range_check = false;
    for t in &tv_constraints {
        if let Some(expr) = is_koalabear_word_range(t, prime) {
            if is_in_koalabear_word_range_check {
                is_in_koalabear_word_range_check = false;
            } else {
                new_tv_constraints.push(expr);
                is_in_koalabear_word_range_check = true;
            }
        } else {
            if !is_in_koalabear_word_range_check {
                new_tv_constraints.push(t.clone());
            }
        }
    }
    tv_constraints = new_tv_constraints;

    for el in &lookup_symbolic_constraints {
        println!("--------- {}", el);
    }

    tv_constraints.extend(lookup_symbolic_constraints);

    let mut used_vars = HashSet::new();
    for t in &tv_constraints {
        gather_vars(0, t, &mut used_vars);
    }
    let mut used_var_ids: HashSet<usize> = used_vars.iter().map(|x| x.1).collect();
    println!("used_var_ids: {:?}", used_var_ids);
    println!("refinable_cols: {:?}", refinable_cols);
    refinable_cols.retain(|c| used_var_ids.contains(c));

    let potential_boolean_vars = gather_boolean_variables(&tv_constraints, &multiplicities);
    let mut range_types: HashMap<usize, RangeType> = potential_boolean_vars
        .iter()
        .map(|k| (*k, RangeType::Bool))
        .collect();
    for c in &u8_cols {
        range_types.insert(*c, RangeType::U8);
    }

    (tv_constraints, refinable_cols, range_types)
}

pub const fn indices_arr<const N: usize>() -> [usize; N] {
    let mut indices_arr = [0; N];
    let mut i = 0;
    while i < N {
        indices_arr[i] = i;
        i += 1;
    }
    indices_arr
}

pub fn dummy_program_counter_refine_fn(
    abs_main_trace_data: &mut Vec<Vec<AbstractInterval>>,
    program_len: usize,
    i: usize,
    j: usize,
) {
}

pub fn dummy_adjust_pc_program(main_trace: &mut AbstractTrace, prime: u32) {}

pub fn dummy_table_deriver(
    cpu_main_trace: &Vec<Vec<AbstractInterval>>,
    range_types: &HashMap<usize, RangeType>,
    prime: u32,
) -> Vec<Vec<AbstractInterval>> {
    let out = vec![];
    out
}
