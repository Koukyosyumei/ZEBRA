use clap::Parser;
use core::mem::transmute;
use std::collections::HashSet;
use std::fs;
use std::io;

use itertools::Itertools;

use p3_koala_bear::KoalaBear;

use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::control_flow::BranchColumns;
use zkm_core_machine::control_flow::NUM_BRANCH_COLS;
use zkm_core_machine::BranchChip;
use zkm_stark::MachineProver;

use latticevm::interval::AbstractInterval;
use latticevm::quick::quick_api;
use latticevm::quick::{experiment_harness, load_config, Args, ProgramInfo, SearchConfig};
use latticevm::smt::expr_to_smt_bv;
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn, RangeType};
use latticevm::symbolic::eval_constraints;
use latticevm::symbolic::LatticeVMSymbolicEntry;
use latticevm::symbolic::LatticeVMSymbolicExpr;
use latticevm::symbolic::LatticeVMSymbolicVal;
use latticevm::ui::save_repr_if_unique;
use latticevm::ui::{generate_alu_final_checker, UiState};
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
        "pc: {}, next_pc: [{}], next_next_pc: [{}], op_a_value: [{}], op_b_value: [{}], op_c_value: [{}]",
        trace.data[0][0],
        trace_fmt_with_idxs(trace, 0, &[1, 2, 3, 4]),
        trace_fmt_with_idxs(trace, 0, &[23, 24, 25, 26]),
        trace_fmt_with_idxs(trace, 0, &[41, 42, 43, 44]),
        trace_fmt_with_idxs(trace, 0, &[45, 46, 47, 48]),
        trace_fmt_with_idxs(trace, 0, &[49, 50, 51, 52]),
    );
    save_repr_if_unique(&string_representation, known_reprt, ui);
}

const fn make_col_map() -> BranchColumns<usize> {
    let indices_arr = indices_arr::<{ NUM_BRANCH_COLS }>();
    unsafe { transmute::<[usize; NUM_BRANCH_COLS], BranchColumns<usize>>(indices_arr) }
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
        Instruction::new(Opcode::ADD, 1, 0, x, true, true),
        Instruction::new(Opcode::ADD, 2, 0, y, true, true),
        Instruction::new(opcode, 1, 2, z, true, true),
    ];
    Program::new(instructions, pc_start, pc_base)
}

pub fn get_opcode(opcode_str: &str) -> Opcode {
    match opcode_str {
        "BEQ" => Opcode::BEQ,
        "BGEZ" => Opcode::BGEZ,
        "BGTZ" => Opcode::BGTZ,
        "BLEZ" => Opcode::BLEZ,
        "BLTZ" => Opcode::BLTZ,
        "BNE" => Opcode::BNE,
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
    let air = BranchChip::default();
    let air_name = "Branch";
    let _colmap = make_col_map();

    let (mut constraint_info, general_lookup_info) =
        extract_constraints_and_range::<KoalaBear, BranchChip>(&air, NUM_BRANCH_COLS, prime);
    let output_columns = vec![23, 24, 25, 26];

    constraint_info
        .refinable_cols
        .extend(&output_columns.clone());
    constraint_info.output_columns = output_columns.clone();

    // ######################## Program Initialization ###########################
    let program = target_program(get_opcode(&opcode_str), 4, 4, 3, 4, 12);

    // ######################## Set Info ##########################################
    let program_info = ProgramInfo {
        program_str: get_program_str(&program),
        program_len: program.instructions.len(),
    };
    let base_abs_main_trace_data = generate_abstract_trace(&program, air_name.to_string(), 1);
    if search_config.minimum_num_taregt_cols == 0 {
        search_config.minimum_num_taregt_cols = 3; //constraint_info.refinable_cols.len();
    }

    let a = vec![
        12, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 27, 20, 155, 0, 20, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 12, 0, 0, 0, 0, 0, 0, 0, 0,
        1, 0, 0, 1,
    ];
    /*
    let a = vec![
        12, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 28, 0, 0, 0, 28, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0, 12, 0, 0, 0, 0, 1, 0, 0, 0, 0,
        1, 0, 1,
    ];*/
    let mut ai = vec![];
    ai.push(a.iter().map(|x| AbstractInterval::from_i64(*x)).collect());
    let at = AbstractTrace::new(ai);
    let re = eval_constraints(&at, None, &constraint_info.constraints, prime);
    println!("{:?}", re);
    println!("{}", constraint_info.constraints.air_constraints[1]);
    println!("{}", constraint_info.constraints.lookup_constraints[1]);

    /*
        pub fn eval_constraints(
        trace: &AbstractTrace,
        public_vals: Option<&[AbstractInterval]>,
        constraints: &LatticeVMConstraints,
        prime: u32,
    )
         */

    /**
     * [12, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 28, 0, 0, 0, 28, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 12, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, ]
     *
     * [12, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 27, 20, 155, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 12, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1]
     */
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
