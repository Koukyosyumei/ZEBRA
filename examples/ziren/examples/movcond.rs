use std::collections::HashSet;
use std::io;
use std::mem::transmute;

use p3_koala_bear::KoalaBear;

use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::misc::MovCondCols;
use zkm_core_machine::misc::NUM_MOV_COND_COLS;
use zkm_core_machine::MovCondChip;
use zkm_stark::MachineProver;

use latticevm::quick::{experiment_harness, ProgramInfo, SearchConfig};
use latticevm::solver::RangeType;
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn};
use latticevm::ui::save_repr_if_unique;
use latticevm::ui::UiState;
use latticevm::utils::trace_fmt_with_idxs;
use latticevm::utils::{create_or_clear_dir, indices_arr};
use latticevm::symbolic::AbstractTrace;

use latticevm_ziren::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
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
        "op_a_value: [{}], prev_a_value: [{}], op_b_value: [{}], op_c_value: [{}]",
        trace_fmt_with_idxs(trace, 0, &[2, 3, 4, 5]),
        trace_fmt_with_idxs(trace, 0, &[6, 7, 8, 9]),
        trace_fmt_with_idxs(trace, 0, &[10, 11, 12, 13]),
        trace_fmt_with_idxs(trace, 0, &[14, 15, 16, 17]),
    );
    save_repr_if_unique(&string_representation, known_reprt, ui);
}

const fn make_col_map() -> MovCondCols<usize> {
    let indices_arr = indices_arr::<{ NUM_MOV_COND_COLS }>();
    unsafe { transmute::<[usize; NUM_MOV_COND_COLS], MovCondCols<usize>>(indices_arr) }
}

pub fn target_program(
    opcode: Opcode,
    pc_start: u32,
    pc_base: u32,
    x: u32,
    y: u32,
    z: u32,
) -> Program {
    let instructions = vec![
        Instruction::new(Opcode::ADD, 1, 0, x, false, true),
        Instruction::new(Opcode::ADD, 2, 0, y, false, true),
        Instruction::new(Opcode::ADD, 3, 0, z, false, true),
        Instruction::new(opcode, 3, 1, 2, true, true),
    ];
    Program::new(instructions, pc_start, pc_base)
}

pub fn get_opcode(opcode_str: &str) -> Opcode {
    match opcode_str {
        "MEQ" => Opcode::MEQ,
        "MNE" => Opcode::MNE,
        "WSBH" => Opcode::WSBH,
        _ => panic!("unsupported instruction"),
    }
}

fn main() -> Result<(), io::Error> {
    let opcode_str = "MEQ";

    create_or_clear_dir("voutput")?;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;

    // ######################## Extract CPU Constraints ##########################
    let air = MovCondChip::default();
    let air_name = "MovCond";
    let _colmap = make_col_map();

    let (mut constraint_info, general_lookup_info) =
        extract_constraints_and_range::<KoalaBear, MovCondChip>(&air, NUM_MOV_COND_COLS, prime);
    let output_columns = vec![2, 3, 4, 5];

    constraint_info
        .refinable_cols
        .extend(&output_columns.clone());
    constraint_info.output_columns = output_columns.clone();
    for i in output_columns {
        constraint_info.range_types.insert(i, RangeType::U8);
    }

    // ######################## Program Initialization ###########################
    let program = target_program(get_opcode(opcode_str), 4, 4, 11, 12, 13);

    // ######################## Set Info ##########################################
    let program_info = ProgramInfo {
        program_str: get_program_str(&program),
        program_len: program.instructions.len(),
    };
    let base_abs_main_trace_data = generate_abstract_trace(&program, air_name.to_string(), 1);
    let search_config = SearchConfig::new(constraint_info.refinable_cols.len());
    // search_config.minimum_num_taregt_cols = 4;

    // ######################## Solve ############################################
    experiment_harness(
        &program_info,
        &mut constraint_info,
        &search_config,
        &base_abs_main_trace_data,
        vec![],
        &vec![], // vec![0],
        dummy_program_counter_refine_fn,
        dummy_adjust_pc_program,
        final_check,
    )
}
