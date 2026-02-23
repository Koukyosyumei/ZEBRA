use clap::Parser;
use core::mem::transmute;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashSet;
use std::io;

use p3_baby_bear::BabyBear;

use sp1_core_executor::{Instruction, Opcode, Program};
use sp1_core_machine::control_flow::BranchChip;
use sp1_core_machine::control_flow::BranchColumns;
use sp1_core_machine::control_flow::NUM_BRANCH_COLS;
use sp1_stark::MachineProver;

use latticevm::quick::{
    experiment_harness, generate_report, load_config, write_output, Args, ProgramInfo,
};
use latticevm::solver::RangeType;
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn};
use latticevm::symbolic::AbstractTrace;
use latticevm::ui::save_repr_if_unique;
use latticevm::ui::UiState;
use latticevm::utils::trace_fmt_with_idxs;
use latticevm::utils::{create_or_clear_dir, indices_arr};

use latticevm_sp1::utils::{
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
        "pc: [{}], next_pc: [{}], op_a_value: [{}], op_b_value: [{}], op_c_value: [{}]",
        trace_fmt_with_idxs(trace, 0, &[0, 1, 2, 3]),
        trace_fmt_with_idxs(trace, 0, &[5, 6, 7, 8]),
        trace_fmt_with_idxs(trace, 0, &[10, 11, 12, 13]),
        trace_fmt_with_idxs(trace, 0, &[14, 15, 16, 17]),
        trace_fmt_with_idxs(trace, 0, &[18, 19, 20, 21]),
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
        "BGE" => Opcode::BGE,
        "BLT" => Opcode::BLT,
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
    let prime = 2_u32.pow(31) - 2_u32.pow(27) + 1;

    // ######################## Extract CPU Constraints ##########################
    let air = BranchChip::default();
    let air_name = "Branch";
    let colmap = make_col_map();
    println!("{:?}", colmap);

    let (mut constraint_info, general_lookup_info) =
        extract_constraints_and_range::<BabyBear, BranchChip>(&air, NUM_BRANCH_COLS, prime);
    let output_columns = vec![5, 6, 7, 8];
    for i in vec![5, 6, 7] {
        constraint_info.range_types.insert(i, RangeType::U8);
    }
    for i in vec![8] {
        constraint_info.range_types.insert(i, RangeType::U7);
    }

    constraint_info
        .refinable_cols
        .extend(&output_columns.clone());
    constraint_info.output_columns = output_columns.clone();
    if search_config.minimum_num_taregt_cols == 0 {
        search_config.minimum_num_taregt_cols = constraint_info.refinable_cols.len();
    }

    let mut rng = StdRng::seed_from_u64(search_config.seed);
    let mut ds = vec![];
    for _ in 0..100 {
        let x: u32 = rng.random();
        let y: u32 = rng.random();
        let z: u32 = rng.random();

        // ######################## Program Initialization ###########################
        let program = target_program(get_opcode(&opcode_str), 4, 4, x, y, z);
        let base_abs_main_trace_data = generate_abstract_trace(&program, air_name.to_string(), 1);

        // ######################## Set Info ##########################################
        let program_info = ProgramInfo {
            program_str: get_program_str(&program),
            program_len: program.instructions.len(),
        };
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
        println!("({} {} {}), {:?}", x, y, z, result);
        ds.push(result.unwrap());
    }
    let report = generate_report(&ds);
    println!("{:?}", report);
    let _ = write_output(args, search_config, report);

    Ok(())
}
