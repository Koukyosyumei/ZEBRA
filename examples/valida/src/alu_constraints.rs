use std::collections::HashMap;


use p3_baby_bear::BabyBear;
//use p3_uni_stark::symbolic_builder::get_symbolic_constraints;

use valida_alu_u32::add::Add32Chip;
use valida_alu_u32::bitwise::Bitwise32Chip;
use valida_alu_u32::com::Com32Chip;
use valida_alu_u32::sub::Sub32Chip;
use valida_basic_api::BasicMachine;

use latticevm::solver::AbsConstraintObj;

use crate::config::MyConfig;
use crate::p3_to_tv::get_converted_symbolicconstraints;

pub fn get_alu_constraints() -> HashMap<String, AbsConstraintObj> {
    let machine = BasicMachine::<BabyBear>::default();
    let mut result = HashMap::new();

    let add_air = Add32Chip::default();
    let (add_constraints, add_potential_boolean_vars) =
        get_converted_symbolicconstraints::<BasicMachine<BabyBear>, MyConfig, _>(
            &machine, &add_air,
        );
    let add_target_cols = vec![8, 9, 10, 11, 12, 13, 14];
    let aux_add_obj = AbsConstraintObj {
        name: "Add".to_string(),
        aux_constraints: add_constraints,
        aux_target_cols: add_target_cols,
        aux_potential_boolean_vars: add_potential_boolean_vars,
    };
    result.insert("Add".to_string(), aux_add_obj);

    let sub_air = Sub32Chip::default();
    let (sub_constraints, sub_potential_boolean_vars) =
        get_converted_symbolicconstraints::<BasicMachine<BabyBear>, MyConfig, _>(
            &machine, &sub_air,
        );
    let sub_target_cols = vec![8, 9, 10, 11];
    let aux_sub_obj = AbsConstraintObj {
        name: "Sub".to_string(),
        aux_constraints: sub_constraints,
        aux_target_cols: sub_target_cols,
        aux_potential_boolean_vars: sub_potential_boolean_vars,
    };
    result.insert("Sub".to_string(), aux_sub_obj);

    let bitwise_air = Bitwise32Chip::default();
    let (bitsise_constraints, bitsise_potential_boolean_vars) = get_converted_symbolicconstraints::<
        BasicMachine<BabyBear>,
        MyConfig,
        _,
    >(&machine, &bitwise_air);
    let bitsise_target_cols = (0..64).collect::<Vec<usize>>();
    //result.insert("Bitwise".to_string(), aux_sub_obj);

    let com_air = Com32Chip::default();
    let (com_constraints, com_potential_boolean_vars) =
        get_converted_symbolicconstraints::<BasicMachine<BabyBear>, MyConfig, _>(
            &machine, &com_air,
        );
    let com_target_cols = vec![8, 9, 10];
    let aux_com_obj = AbsConstraintObj {
        name: "Com".to_string(),
        aux_constraints: com_constraints,
        aux_target_cols: com_target_cols,
        aux_potential_boolean_vars: com_potential_boolean_vars,
    };
    result.insert("Com".to_string(), aux_com_obj);

    result
}
