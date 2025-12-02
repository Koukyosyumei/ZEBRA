use std::collections::HashMap;

use p3_baby_bear::BabyBear;
//use p3_uni_stark::symbolic_builder::get_symbolic_constraints;

use valida_alu_u32::add::columns::NUM_ADD_COLS;
use valida_alu_u32::add::Add32Chip;
use valida_alu_u32::bitwise::Bitwise32Chip;
use valida_alu_u32::com::Com32Chip;
use valida_alu_u32::sub::Sub32Chip;
use valida_basic_api::BasicMachine;
use valida_machine::symbolic::symbolic_builder::get_lookup_interactions;
use valida_machine::ChipWithPersistence;
use valida_machine::Machine;
use valida_machine::StarkConfig;

use latticevm::interval::AbstractInterval;
use latticevm::solver::AbsConstraintObj;
use latticevm::solver::RangeType;

use crate::config::MyConfig;
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

    get_lookup_interactions(
        machine,
        chip,
        &mut u8_lookup_cols,
        &mut cpu_lookup_cols,
        &mut multiplicity_col,
    );
    for c in &u8_lookup_cols {
        col_range_types.insert(*c, RangeType::U8);
    }
    println!("cpu: {:?}", cpu_lookup_cols);
    println!("mul: {:?}", multiplicity_col);

    let cpu_input_cols: Vec<usize> = cpu_lookup_cols.iter().cloned().take(8).collect();
    let mut refinable_cols: Vec<usize> = (0..num_columns).collect();
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

pub fn get_alu_constraints() -> HashMap<String, AbsConstraintObj> {
    let machine = BasicMachine::<BabyBear>::default();
    let mut result = HashMap::new();

    let add_air = Add32Chip::default();
    let (add_constraints, add_potential_boolean_vars) =
        get_converted_symbolicconstraints::<BasicMachine<BabyBear>, MyConfig, _>(
            &machine, &add_air,
        );
    let mut add_range_types: HashMap<usize, RangeType> = add_potential_boolean_vars
        .iter()
        .map(|k| (*k, RangeType::Bool))
        .collect();
    let mut cols_constrained_by_u8_chip = Vec::new();
    let mut cols_constrained_by_cpu_chip = Vec::new();
    let mut counter_col = Vec::new();
    get_lookup_interactions::<BasicMachine<BabyBear>, MyConfig, _>(
        &machine,
        &add_air,
        &mut cols_constrained_by_u8_chip,
        &mut cols_constrained_by_cpu_chip,
        &mut counter_col,
    );
    println!("u8: {:?}", cols_constrained_by_u8_chip);
    println!("bus: {:?}", cols_constrained_by_cpu_chip);
    println!("counter: {:?}", counter_col);
    for c in &cols_constrained_by_u8_chip {
        add_range_types.insert(*c, RangeType::U8);
    }
    let input_cols: Vec<usize> = cols_constrained_by_cpu_chip
        .iter()
        .cloned()
        .take(8)
        .collect();
    println!("input: {:?}", input_cols);
    let mut add_target_cols: Vec<usize> = (0..NUM_ADD_COLS).collect();
    add_target_cols.retain(|x| !input_cols.contains(x));
    add_target_cols.retain(|x| !counter_col.contains(x));
    println!("{:?}", add_target_cols);

    //let add_target_cols = vec![8, 9, 10, 11, 12, 13, 14];
    let aux_add_obj = AbsConstraintObj {
        name: "Add".to_string(),
        aux_constraints: add_constraints,
        aux_refinement_plan: add_target_cols,
        aux_range_types: add_range_types,
    };
    result.insert("Add".to_string(), aux_add_obj);

    let sub_air = Sub32Chip::default();
    let (sub_constraints, sub_potential_boolean_vars) =
        get_converted_symbolicconstraints::<BasicMachine<BabyBear>, MyConfig, _>(
            &machine, &sub_air,
        );
    let sub_range_types = sub_potential_boolean_vars
        .iter()
        .map(|k| (*k, RangeType::Bool))
        .collect();
    let sub_target_cols = vec![8, 9, 10, 11, 12, 13, 14, 15];
    let aux_sub_obj = AbsConstraintObj {
        name: "Sub".to_string(),
        aux_constraints: sub_constraints,
        aux_refinement_plan: sub_target_cols,
        aux_range_types: sub_range_types,
    };
    result.insert("Sub".to_string(), aux_sub_obj);

    let bitwise_air = Bitwise32Chip::default();
    let (bitsise_constraints, bitsise_potential_boolean_vars) = get_converted_symbolicconstraints::<
        BasicMachine<BabyBear>,
        MyConfig,
        _,
    >(&machine, &bitwise_air);
    let bitwise_range_types = bitsise_potential_boolean_vars
        .iter()
        .map(|k| (*k, RangeType::Bool))
        .collect();
    let bitsise_target_cols = (0..64).collect::<Vec<usize>>();
    let aux_bitwise_obj = AbsConstraintObj {
        name: "Bitwise".to_string(),
        aux_constraints: bitsise_constraints,
        aux_refinement_plan: bitsise_target_cols,
        aux_range_types: bitwise_range_types,
    };
    result.insert("Bitwise".to_string(), aux_bitwise_obj);

    let com_air = Com32Chip::default();
    let (com_constraints, com_potential_boolean_vars) =
        get_converted_symbolicconstraints::<BasicMachine<BabyBear>, MyConfig, _>(
            &machine, &com_air,
        );
    let com_target_cols = vec![8, 9, 10];
    let com_range_types = com_potential_boolean_vars
        .iter()
        .map(|k| (*k, RangeType::Bool))
        .collect();
    let aux_com_obj = AbsConstraintObj {
        name: "Com".to_string(),
        aux_constraints: com_constraints,
        aux_refinement_plan: com_target_cols,
        aux_range_types: com_range_types,
    };
    result.insert("Com".to_string(), aux_com_obj);

    result
}
