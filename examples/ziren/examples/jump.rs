use clap::Parser;
use core::mem::transmute;
use std::collections::HashSet;
use std::io;

use p3_koala_bear::KoalaBear;

use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::control_flow::JumpColumns;
use zkm_core_machine::control_flow::NUM_JUMP_COLS;
use zkm_core_machine::JumpChip;

use latticevm::quick::{experiment_harness, load_config, Args, ProgramInfo};
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn, RangeType};
use latticevm::symbolic::AbstractTrace;
use latticevm::ui::save_repr_if_unique;
use latticevm::ui::UiState;
use latticevm::utils::trace_fmt_with_idxs;
use latticevm::utils::{create_or_clear_dir, indices_arr};

use latticevm_ziren::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

// ############## Final Check Function ##############################
fn final_check(
    trace: &AbstractTrace,
    _num_trial: usize,
    _prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    let string_representation = format!(
        "pc: {}, next_pc: [{}], next_next_pc: [{}], op_a_value: [{}], op_b_value: [{}], op_c_value: [{}]",
        trace.data[0][0],
        trace_fmt_with_idxs(trace, 0, &[1, 2, 3, 4]),
        trace_fmt_with_idxs(trace, 0, &[19, 20, 21, 22]),
        trace_fmt_with_idxs(trace, 0, &[37, 38, 39, 40]),
        trace_fmt_with_idxs(trace, 0, &[41, 42, 43, 44]),
        trace_fmt_with_idxs(trace, 0, &[45, 46, 47, 48]),
    );
    save_repr_if_unique(&string_representation, known_reprt, ui);
}

const fn make_col_map() -> JumpColumns<usize> {
    let indices_arr = indices_arr::<{ NUM_JUMP_COLS }>();
    unsafe { transmute::<[usize; NUM_JUMP_COLS], JumpColumns<usize>>(indices_arr) }
}

pub fn target_program(
    opcode: Opcode,
    pc_start: u32,
    pc_base: u32,
    x: u8,
    y: u32,
    z: u32,
) -> Program {
    let instructions = vec![
        Instruction::new(Opcode::ADD, x, 0, y, false, true),
        Instruction::new(opcode, 0, z, 0, false, true),
    ];
    Program::new(instructions, pc_start, pc_base)
}

pub fn get_opcode(opcode_str: &str) -> Opcode {
    match opcode_str {
        "Jump" => Opcode::Jump,
        "Jumpi" => Opcode::Jumpi,
        "JumpDirect" => Opcode::JumpDirect,
        _ => panic!("unsupported instruction"),
    }
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    let args = Args::parse();
    let opcode_str = args.opcode_str;
    let mut search_config = load_config(&args.config).unwrap();

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;

    // ######################## Extract CPU Constraints ##########################
    let air = JumpChip::default();
    let air_name = "Jump";
    let _colmap = make_col_map();

    let (mut constraint_info, _general_lookup_info) =
        extract_constraints_and_range::<KoalaBear, JumpChip>(&air, NUM_JUMP_COLS, prime);
    let output_columns = vec![19, 20, 21, 22, 37, 38, 39, 40];

    constraint_info
        .refinable_cols
        .extend(&output_columns.clone());
    constraint_info.output_columns = output_columns.clone();
    for i in vec![19, 20, 21, 37, 38, 39] {
        constraint_info.range_types.insert(i, RangeType::U8);
    }
    for i in vec![22, 40] {
        constraint_info.range_types.insert(i, RangeType::U7);
    }

    // ######################## Program Initialization ###########################
    let program = target_program(get_opcode(&opcode_str), 4, 4, 1, 32, 1);

    // ######################## Set Info ##########################################
    let program_info = ProgramInfo {
        program_str: get_program_str(&program),
        program_len: program.instructions.len(),
    };
    let base_abs_main_trace_data = generate_abstract_trace(&program, air_name.to_string(), 1);
    if search_config.minimum_num_taregt_cols == 0 {
        search_config.minimum_num_taregt_cols = constraint_info.refinable_cols.len();
    }

    // ######################## Solve ############################################
    let result = experiment_harness(
        &program_info,
        &mut constraint_info,
        &search_config,
        &base_abs_main_trace_data,
        vec![],
        &vec![],
        dummy_program_counter_refine_fn,
        dummy_adjust_pc_program,
        final_check,
        &args.method,
    );
    println!("{:?}", result);

    Ok(())
}
