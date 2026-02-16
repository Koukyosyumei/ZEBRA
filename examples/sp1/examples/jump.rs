use clap::Parser;
use core::mem::transmute;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashSet;
use std::io;

use p3_baby_bear::BabyBear;

use sp1_core_executor::{Instruction, Opcode, Program};
use sp1_core_machine::control_flow::JumpChip;
use sp1_core_machine::control_flow::JumpColumns;
use sp1_core_machine::control_flow::NUM_JUMP_COLS;

use latticevm::quick::{experiment_harness, load_config, mean_variance, Args, ProgramInfo};
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn, RangeType};
use latticevm::symbolic::eval_constraints;
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
        "pc: [{}], next_pc: [{}], op_b_value: [{}], op_c_value: [{}]",
        trace_fmt_with_idxs(trace, 0, &[0, 1, 2, 3]),
        trace_fmt_with_idxs(trace, 0, &[5, 6, 7, 8]),
        //trace_fmt_with_idxs(trace, 0, &[10, 11, 12, 13]),
        trace_fmt_with_idxs(trace, 0, &[14, 15, 16, 17]),
        trace_fmt_with_idxs(trace, 0, &[18, 19, 20, 21]),
    );
    save_repr_if_unique(&string_representation, known_reprt, ui);
}

const fn make_col_map() -> JumpColumns<usize> {
    let indices_arr = indices_arr::<{ NUM_JUMP_COLS }>();
    unsafe { transmute::<[usize; NUM_JUMP_COLS], JumpColumns<usize>>(indices_arr) }
}

pub fn target_program(opcode: Opcode, pc_start: u32, pc_base: u32, x: u8, y: u32) -> Program {
    let mut instructions = vec![
        //Instruction::new(Opcode::ADD, x, 0, 0, false, true), // initialize the register
        Instruction::new(Opcode::JAL, x, y, 0, true, true),
    ];
    Program::new(instructions, pc_start, pc_base)
}

pub fn get_opcode(opcode_str: &str) -> Opcode {
    match opcode_str {
        "JAL" => Opcode::JAL,
        "JALR" => Opcode::JALR,
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
    let air = JumpChip::default();
    let air_name = "Jump";
    let colmap = make_col_map();
    println!("{:?}", colmap);

    let (mut constraint_info, _general_lookup_info) =
        extract_constraints_and_range::<BabyBear, JumpChip>(&air, NUM_JUMP_COLS, prime);
    let output_columns = vec![5, 6, 7, 8];
    for i in vec![5, 6, 7, 8, 0, 1, 2] {
        constraint_info.range_types.insert(i, RangeType::U8);
    }
    for i in vec![8] {
        constraint_info.range_types.insert(i, RangeType::U8);
    }

    constraint_info
        .refinable_cols
        .extend(&output_columns.clone());
    constraint_info.output_columns = output_columns.clone();

    for t in &constraint_info.constraints.air_constraints {
        println!("# {}", t);
    }
    for t in &constraint_info.constraints.lookup_constraints {
        println!("* {}", t);
    }

    if search_config.minimum_num_taregt_cols == 0 {
        search_config.minimum_num_taregt_cols = constraint_info.refinable_cols.len();
    }
    search_config.seed = 1;

    let mut rng = StdRng::seed_from_u64(search_config.seed);
    let mut ds = vec![];
    for _ in 0..100 {
        let x: u8 = rng.random_range(0..32);
        let y: u32 = rng.random_range(0..10000); //rng.random(); // rng.random_range(0..prime);

        // ######################## Program Initialization ###########################
        let program = target_program(get_opcode(&opcode_str), 8, 8, x, y);
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
        println!("({} {}), {:?}", x, y, result);
        ds.push(result.unwrap().execution_time);
    }
    println!("{:?}", mean_variance(&ds));

    Ok(())
}
