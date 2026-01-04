use std::collections::HashMap;

use p3_baby_bear::BabyBear;
//use p3_uni_stark::symbolic_builder::get_symbolic_constraints;

use valida_alu_u32::add::columns::NUM_ADD_COLS;
use valida_alu_u32::add::Add32Chip;
use valida_alu_u32::bitwise::Bitwise32Chip;
use valida_alu_u32::com::Com32Chip;
use valida_alu_u32::sub::Sub32Chip;
use valida_basic_api::BasicMachine;
use valida_machine::ChipWithPersistence;
use valida_machine::Machine;
use valida_machine::StarkConfig;

use latticevm::solver::AbsConstraintObj;
use latticevm::solver::RangeType;

use crate::config::MyConfig;
use crate::lookup::get_lookup_interactions;
use crate::p3_to_tv::get_converted_symbolicconstraints;

pub fn get_alu_constraint<M, SC, C>(
    machine: &M,
    chip: &C,
    num_columns: usize,
    prime: u32,
) -> (AbsConstraintObj, Vec<usize>)
where
    M: Machine<SC::Val>,
    SC: StarkConfig,
    C: ChipWithPersistence<M, SC>,
{
    let (symbolic_constraints, boolean_candidate_cols) =
        get_converted_symbolicconstraints(machine, chip);
    let mut col_range_types: HashMap<usize, RangeType> = boolean_candidate_cols
        .iter()
        .map(|k| (*k, RangeType::Bool))
        .collect();

    let mut u8_lookup_cols = Vec::new();
    let mut cpu_lookup_cols = Vec::new();
    let mut multiplicity_col = Vec::new();
    let mut lookup_constraints = Vec::new();

    get_lookup_interactions(
        machine,
        chip,
        &mut u8_lookup_cols,
        &mut cpu_lookup_cols,
        &mut multiplicity_col,
        &mut lookup_constraints,
        prime,
    );
    for c in &u8_lookup_cols {
        col_range_types.insert(*c, RangeType::U8);
    }
    println!("cpu: {:?}", cpu_lookup_cols);
    println!("mul: {:?}", multiplicity_col);

    let cpu_selector_cols: Vec<usize> = cpu_lookup_cols[0].clone();
    let cpu_input_cols: Vec<usize> = cpu_lookup_cols
        .iter()
        .cloned()
        .skip(1)
        .take(8)
        .flatten()
        .collect();
    let mut refinable_cols: Vec<usize> = (0..num_columns).collect();
    refinable_cols.retain(|c| !cpu_selector_cols.contains(c));
    refinable_cols.retain(|c| !cpu_input_cols.contains(c));
    refinable_cols.retain(|c| !multiplicity_col.contains(c));

    for c in &refinable_cols {
        if !col_range_types.contains_key(c) {
            col_range_types.insert(*c, RangeType::Top);
        }
    }

    (
        AbsConstraintObj {
            name: "Add".to_string(),
            aux_constraints: symbolic_constraints,
            aux_refinement_plan: refinable_cols,
            aux_range_types: col_range_types,
        },
        cpu_input_cols,
    )
}
