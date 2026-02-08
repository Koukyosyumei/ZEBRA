use clap::Parser;
use core::mem::transmute;
use std::collections::HashSet;
use std::fs;
use std::io;

use p3_koala_bear::KoalaBear;

use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::alu::DivRemCols;
use zkm_core_machine::alu::NUM_DIVREM_COLS;
use zkm_core_machine::DivRemChip;
use zkm_stark::MachineProver;

use latticevm::quick::quick_api;
use latticevm::quick::SearchConfig;
use latticevm::quick::{experiment_harness, load_config, Args, ProgramInfo};
use latticevm::solver::RangeType;
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn};
use latticevm::ui::save_repr_if_unique;
use latticevm::ui::UiState;
use latticevm::utils::trace_fmt_with_idxs;
use latticevm::utils::{create_or_clear_dir, indices_arr};
use latticevm::{symbolic::AbstractTrace, symbolic::LatticeVMConstraints};

use latticevm_ziren::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

fn canonical_repr_div(trace: &AbstractTrace) -> String {
    format!(
        "input0: [{}], input1: [{}], output: [{}]",
        trace_fmt_with_idxs(trace, 0, &[2, 3, 4, 5]),
        trace_fmt_with_idxs(trace, 0, &[6, 7, 8, 9]),
        trace_fmt_with_idxs(trace, 0, &[10, 11, 12, 13]),
    )
}

fn canonical_repr_rem(trace: &AbstractTrace) -> String {
    format!(
        "input0: [{}], input1: [{}], output: [{}]",
        trace_fmt_with_idxs(trace, 0, &[2, 3, 4, 5]),
        trace_fmt_with_idxs(trace, 0, &[6, 7, 8, 9]),
        trace_fmt_with_idxs(trace, 0, &[14, 15, 16, 17]),
    )
}

// ############## Final Check Function ##############################
fn final_check_div(
    trace: &AbstractTrace,
    _num_trial: usize,
    _prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    save_repr_if_unique(&canonical_repr_div(trace), known_reprt, ui);
}

fn final_check_rem(
    trace: &AbstractTrace,
    _num_trial: usize,
    _prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    save_repr_if_unique(&canonical_repr_rem(trace), known_reprt, ui);
}

const fn make_col_map() -> DivRemCols<usize> {
    let indices_arr = indices_arr::<{ NUM_DIVREM_COLS }>();
    unsafe { transmute::<[usize; NUM_DIVREM_COLS], DivRemCols<usize>>(indices_arr) }
}

pub fn target_program(opcode: Opcode, pc_start: u32, pc_base: u32, x: u32, y: u32) -> Program {
    let instructions = vec![Instruction::new(opcode, 1, x, y, true, true)];
    Program::new(instructions, pc_start, pc_base)
}

pub fn get_opcode(opcode_str: &str) -> Opcode {
    match opcode_str {
        "DIV" => Opcode::DIV,
        "DIVU" => Opcode::DIVU,
        "MOD" => Opcode::MOD,
        "MODU" => Opcode::MODU,
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

    // ######################## Canonicalization ##################################
    let cr = if opcode_str == "DIV" || opcode_str == "DIVU" {
        canonical_repr_div
    } else {
        canonical_repr_rem
    };
    let final_check =
        |at: &AbstractTrace, _n: usize, _p: u32, kr: &mut HashSet<String>, ui: &mut UiState| {
            save_repr_if_unique(&cr(at), kr, ui);
        };

    let output_columns = if opcode_str == "DIV" || opcode_str == "DIVU" {
        vec![10, 11, 12, 13]
    } else {
        vec![14, 15, 16, 17]
    };

    // ######################## Extract CPU Constraints ##########################
    let air = DivRemChip::default();
    let air_name = "DivRem";
    let _colmap = make_col_map();
    let (mut constraint_info, general_lookup_info) =
        extract_constraints_and_range::<KoalaBear, DivRemChip>(&air, NUM_DIVREM_COLS, prime);
    constraint_info
        .refinable_cols
        .extend(&output_columns.clone());
    constraint_info.output_columns = output_columns.clone();

    for i in &output_columns {
        constraint_info.range_types.insert(*i, RangeType::U8);
    }

    // ######################## Program Initialization ###########################
    let program = target_program(get_opcode(&opcode_str), 4, 4, 13, 3);

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
        &vec![], // vec![0],
        dummy_program_counter_refine_fn,
        dummy_adjust_pc_program,
        final_check,
        &args.method,
    );
    println!("{:?}", result);

    Ok(())
}
