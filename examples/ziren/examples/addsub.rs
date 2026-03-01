use clap::Parser;
use core::mem::transmute;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashSet;
use std::io;

use p3_koala_bear::KoalaBear;

use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::alu::{AddSubCols, NUM_ADD_SUB_COLS};
use zkm_core_machine::AddSubChip;

use latticevm::canonicalizer::save_repr_if_unique;
use latticevm::interval::MayBeFlag;
use latticevm::quick::{
    experiment_harness, generate_report, load_config, write_output, Args, ProgramInfo,
};
use latticevm::solver::{dummy_program_counter_refine_fn, nop_post_process};
use latticevm::trace::{trace_fmt_with_idxs, AbstractTrace};
use latticevm::ui::UiState;
use latticevm::utils::PrettySet;
use latticevm::utils::{create_or_clear_dir, indices_arr};

use latticevm_ziren::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

fn cr_add(trace: &AbstractTrace, prime: u32) -> PrettySet<String> {
    let mut record_reprs = HashSet::new();
    for i in 0..trace.data.len() {
        if trace.data[i][17].is_zero(prime) != MayBeFlag::True {
            record_reprs.insert(format!(
                "input0: [{}], input1: [{}], output: [{}]",
                trace_fmt_with_idxs(trace, 0, &[9, 10, 11, 12]),
                trace_fmt_with_idxs(trace, 0, &[13, 14, 15, 16]),
                trace_fmt_with_idxs(trace, 0, &[2, 3, 4, 5]),
            ));
        };
    }
    PrettySet(record_reprs)
}

fn cr_sub(trace: &AbstractTrace, prime: u32) -> PrettySet<String> {
    let mut record_reprs = HashSet::new();
    for i in 0..trace.data.len() {
        if trace.data[i][18].is_zero(prime) != MayBeFlag::True {
            record_reprs.insert(format!(
                "input0: [{}], input1: [{}], output: [{}]",
                trace_fmt_with_idxs(trace, 0, &[2, 3, 4, 5]),
                trace_fmt_with_idxs(trace, 0, &[13, 14, 15, 16]),
                trace_fmt_with_idxs(trace, 0, &[9, 10, 11, 12]),
            ));
        };
    }
    PrettySet(record_reprs)
}

const fn make_col_map() -> AddSubCols<usize> {
    let indices_arr = indices_arr::<{ NUM_ADD_SUB_COLS }>();
    unsafe { transmute::<[usize; NUM_ADD_SUB_COLS], AddSubCols<usize>>(indices_arr) }
}

pub fn target_program(opcode: Opcode, pc_start: u32, pc_base: u32, x: u32, y: u32) -> Program {
    let instructions = vec![Instruction::new(opcode, 1, x, y, true, true)];
    Program::new(instructions, pc_start, pc_base)
}

pub fn get_opcode(opcode_str: &str) -> Opcode {
    match opcode_str {
        "ADD" => Opcode::ADD,
        "SUB" => Opcode::SUB,
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

    // ######################## Canonicalization ##################################
    let cr = if opcode_str == "ADD" { cr_add } else { cr_sub };
    let final_check =
        |at: &AbstractTrace, _n: usize, p: u32, kr: &mut HashSet<String>, ui: &mut UiState| {
            save_repr_if_unique(&cr(at, p), kr, ui);
        };

    // ######################## Extract CPU Constraints ##########################
    let air = AddSubChip::default();
    let air_name = "AddSub";
    let _colmap = make_col_map();

    let output_columns = if opcode_str == "ADD" {
        vec![2, 3, 4, 5]
    } else {
        vec![9, 10, 11, 12]
    };

    let (mut constraint_info, _general_lookup_info) =
        extract_constraints_and_range::<KoalaBear, AddSubChip>(&air, NUM_ADD_SUB_COLS, prime);
    constraint_info
        .refinable_cols
        .extend(&output_columns.clone());
    constraint_info.output_columns = output_columns.clone();
    if search_config.minimum_num_taregt_cols == 0 {
        search_config.minimum_num_taregt_cols = constraint_info.refinable_cols.len();
    }

    let mut rng = StdRng::seed_from_u64(search_config.seed);
    let mut ds = vec![];
    for _ in 0..args.num_trial {
        let x: u32 = rng.random();
        let y: u32 = rng.random();

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
            &vec![], // vec![0],
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
