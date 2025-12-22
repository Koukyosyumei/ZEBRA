use std::collections::HashSet;
use std::fs;
use std::io;
use std::mem::transmute;

use itertools::Itertools;

use p3_koala_bear::KoalaBear;

use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::control_flow::BranchColumns;
use zkm_core_machine::misc::MovCondCols;
use zkm_core_machine::misc::NUM_MOV_COND_COLS;
use zkm_core_machine::MovCondChip;
use zkm_stark::MachineProver;

use latticevm::quick::quick_api;
use latticevm::solver::RangeType;
use latticevm::ui::UiState;
use latticevm::utils::create_or_clear_dir;
use latticevm::{symbolic::AbstractTrace, symbolic::LatticeVMConstraints};

use latticevm_ziren::utils::{
    dummy_adjust_pc_program, dummy_program_counter_refine_fn, dummy_table_deriver,
    extract_constraints_and_range, generate_abstract_trace, get_program_str, indices_arr,
};

// ############## Final Check Function ##############################
fn final_check(
    trace: &AbstractTrace,
    num_trial: usize,
    prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    let string_representation = format!(
        "op_a_value: [{}, {}, {}, {}], prev_a_value: [{}, {}, {}, {}], op_b_value: [{}, {}, {}, {}], op_c_value: [{}, {}, {}, {}]",
        trace.data[0][2],
        trace.data[0][3],
        trace.data[0][4],
        trace.data[0][5],
        trace.data[0][6],
        trace.data[0][7],
        trace.data[0][8],
        trace.data[0][9],
        trace.data[0][10],
        trace.data[0][11],
        trace.data[0][12],
        trace.data[0][13],
        trace.data[0][14],
        trace.data[0][15],
        trace.data[0][16],
        trace.data[0][17],
    );

    if !known_reprt.contains(&string_representation) {
        known_reprt.insert(string_representation.clone());
        ui.recovered = string_representation;

        fs::write(
            format!("voutput/{}_states.txt", known_reprt.len()),
            ui.recovered.clone(),
        )
        .unwrap();
        fs::write(
            format!("voutput/{}_assignments.txt", known_reprt.len()),
            ui.logs.clone(),
        )
        .unwrap();
    }
}

const fn make_col_map() -> MovCondCols<usize> {
    let indices_arr = indices_arr::<{ NUM_MOV_COND_COLS }>();
    unsafe { transmute::<[usize; NUM_MOV_COND_COLS], MovCondCols<usize>>(indices_arr) }
}

pub fn target_program(pc_start: u32, pc_base: u32) -> Program {
    let mut instructions = vec![Instruction::new(Opcode::MEQ, 1, 2, 3, true, true)];
    Program::new(instructions, pc_start, pc_base)
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;

    // ######################## Solver Parameters ###############################
    let max_iteration = 10000000;
    let min_row_id = 0;
    let max_row_id = 0;
    let num_extracted_rows = 1;
    let seed = 41;
    let aux_tg_fns: Vec<_> = vec![dummy_table_deriver];

    // ######################## Extract CPU Constraints ##########################
    let air = MovCondChip::default();
    let air_name = "MovCond";
    let colmap = make_col_map();
    println!("op_a_value: {:?}", colmap.op_a_value);
    println!("prev_a_value: {:?}", colmap.prev_a_value);
    println!("op_b_value: {:?}", colmap.op_b_value);
    println!("op_c_value: {:?}", colmap.op_c_value);
    println!("c_eq_0: {:?}", colmap.c_eq_0);

    let (tv_constraints, mut refinable_cols, mut range_types) =
        extract_constraints_and_range::<KoalaBear, MovCondChip>(&air, NUM_MOV_COND_COLS, prime);
    refinable_cols.extend(&[2, 3, 4, 5]); // output
                                          //refinable_cols.extend(&[10, 14]); // input
                                          //range_types.insert(6, RangeType::U4);
                                          //range_types.insert(10, RangeType::U4);
                                          //range_types.insert(14, RangeType::U4);
    for t in &tv_constraints {
        println!("---- {}", t);
    }

    println!("{:?}", refinable_cols);
    println!("{:?}", range_types);

    let constraints = LatticeVMConstraints {
        air_constraints: tv_constraints.clone(),
        pv_pos_constraints: vec![],
        pv_neg_constraints: vec![],
    };
    let minimum_num_taregt_cols = refinable_cols.len();
    println!("{}", minimum_num_taregt_cols);

    // ######################## Program Initialization ###########################
    let program = target_program(4, 4);
    let base_abs_main_trace_data =
        generate_abstract_trace(&program, air_name.to_string(), num_extracted_rows);

    // ######################## Solve ############################################
    quick_api(
        get_program_str(&program),
        &constraints,
        &refinable_cols,
        &range_types,
        &vec![],
        &aux_tg_fns,
        &vec![],
        &base_abs_main_trace_data,
        vec![],
        max_iteration,
        minimum_num_taregt_cols,
        min_row_id,
        max_row_id,
        program.instructions.len(),
        dummy_program_counter_refine_fn,
        dummy_adjust_pc_program,
        final_check,
        prime,
        seed,
    )
}
