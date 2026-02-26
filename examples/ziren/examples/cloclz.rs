use clap::Parser;
use core::mem::transmute;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashSet;
use std::io;

use p3_koala_bear::KoalaBear;

use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::alu::{CloClzCols, NUM_CLOCLZ_COLS};
use zkm_core_machine::CloClzChip;

use latticevm::canonicalizer::save_repr_if_unique;
use latticevm::quick::{
    experiment_harness, generate_report, load_config, write_output, Args, ProgramInfo,
};
use latticevm::solver::{dummy_program_counter_refine_fn, nop_post_process, RangeType};
use latticevm::trace::trace_fmt_with_idxs;
use latticevm::trace::AbstractTrace;
use latticevm::ui::UiState;
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
    let opcode_str = args.opcode_str.clone();
    let mut search_config = load_config(&args.config).unwrap();

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;

    // ######################## Extract CPU Constraints ##########################
    let air = CloClzChip::default();
    let air_name = "CloClz";
    let _colmap = make_col_map();

    let (mut constraint_info, _general_lookup_info) =
        extract_constraints_and_range::<KoalaBear, CloClzChip>(&air, NUM_CLOCLZ_COLS, prime);
    let output_columns = vec![2, 3, 4, 5];

    constraint_info
        .refinable_cols
        .extend(&output_columns.clone());
    constraint_info.output_columns = output_columns.clone();
    if search_config.minimum_num_taregt_cols == 0 {
        search_config.minimum_num_taregt_cols = constraint_info.refinable_cols.len();
    }

    let mut rng = StdRng::seed_from_u64(search_config.seed);
    let mut ds = vec![];
    for i in 0..30 {
        search_config.seed += i;

        let x: u32 = rng.random();
        let y: u32 = rng.random_range(0..256);

        // ######################## Program Initialization ###########################
        let program = target_program(get_opcode(&opcode_str), 4, 4, x, y);
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
            nop_post_process,
            final_check,
            &args.method,
        );
        println!("({} {}), {:?}", x, y, result);
        ds.push(result.unwrap());
    }
    let report = generate_report(&ds);
    println!("{:?}", report);
    let _ = write_output(args, search_config, report);

    Ok(())
}
