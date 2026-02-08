use clap::Parser;
use core::mem::transmute;
use std::collections::HashSet;
use std::fs;
use std::io;

use itertools::Itertools;

use p3_koala_bear::KoalaBear;

use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::alu::CloClzCols;
use zkm_core_machine::alu::NUM_ADD_SUB_COLS;
use zkm_core_machine::alu::NUM_BITWISE_COLS;
use zkm_core_machine::alu::NUM_CLOCLZ_COLS;
use zkm_core_machine::control_flow::BranchColumns;
use zkm_core_machine::memory::MemoryLocalChip;
use zkm_core_machine::AddSubChip;
use zkm_core_machine::BitwiseChip;
use zkm_core_machine::BranchChip;
use zkm_core_machine::CloClzChip;
use zkm_stark::MachineProver;

use latticevm::interval::AbstractInterval;
use latticevm::quick::quick_api;
use latticevm::quick::{experiment_harness, load_config, Args, ProgramInfo, SearchConfig};
use latticevm::smt::expr_to_smt;
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn, RangeType};
use latticevm::symbolic::LatticeVMSymbolicEntry;
use latticevm::symbolic::LatticeVMSymbolicExpr;
use latticevm::symbolic::LatticeVMSymbolicVal;
use latticevm::ui::save_repr_if_unique;
use latticevm::ui::UiState;
use latticevm::utils::trace_fmt_with_idxs;
use latticevm::utils::{create_or_clear_dir, indices_arr};
use latticevm::{symbolic::AbstractTrace, symbolic::LatticeVMConstraints};

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
        "input0: [{}], output: [{}]",
        trace_fmt_with_idxs(trace, 0, &[6, 7, 8, 9]),
        trace_fmt_with_idxs(trace, 0, &[2, 3, 4, 5]),
    );

    save_repr_if_unique(&string_representation, known_reprt, ui);
}

const fn make_col_map() -> CloClzCols<usize> {
    let indices_arr = indices_arr::<{ NUM_CLOCLZ_COLS }>();
    unsafe { transmute::<[usize; NUM_CLOCLZ_COLS], CloClzCols<usize>>(indices_arr) }
}

pub fn target_program(opcode: Opcode, pc_start: u32, pc_base: u32, x: u32, y: u32) -> Program {
    let instructions = vec![Instruction::new(opcode, 1, x, y, true, true)];
    Program::new(instructions, pc_start, pc_base)
}

pub fn get_opcode(opcode_str: &str) -> Opcode {
    match opcode_str {
        "CLO" => Opcode::CLO,
        "CLZ" => Opcode::CLZ,
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
    let air = CloClzChip::default();
    let air_name = "CloClz";
    let _colmap = make_col_map();

    let (mut constraint_info, general_lookup_info) =
        extract_constraints_and_range::<KoalaBear, CloClzChip>(&air, NUM_CLOCLZ_COLS, prime);
    let output_columns = vec![2, 3, 4, 5];
    for i in &output_columns {
        constraint_info.range_types.insert(*i, RangeType::U8);
    }

    constraint_info
        .refinable_cols
        .extend(&output_columns.clone());
    constraint_info.output_columns = output_columns.clone();

    // ######################## Program Initialization ###########################
    let program = target_program(get_opcode(&opcode_str), 4, 4, 3, 4);

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
