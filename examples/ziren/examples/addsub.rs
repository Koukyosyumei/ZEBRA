use clap::Parser;
use core::mem::transmute;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashSet;
use std::io;

use p3_koala_bear::KoalaBear;

use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::alu::{AddSubCols, NUM_ADD_SUB_COLS};
use zkm_core_machine::AddSubChip;

use zebra::canonicalizer::save_repr_if_unique;
use zebra::interval::MayBeFlag;
use zebra::quick::{
    experiment_harness, generate_report, load_config, write_output, Args, ProgramInfo,
};
use zebra::solver::nop_post_process;
use zebra::trace::{trace_fmt_with_idxs, AbstractTrace};
use zebra::ui::UiState;
use zebra::utils::PrettySet;
use zebra::utils::{create_or_clear_dir, indices_arr};

use zebra_ziren::utils::{extract_constraints_and_range, generate_abstract_trace, get_program_str};

fn cr_add(trace: &AbstractTrace, prime: u32) -> PrettySet<String> {
    let mut record_reprs = HashSet::new();
    for i in 0..trace.data.len() {
        if trace.data[i][17].is_zero(prime) != MayBeFlag::True {
            record_reprs.insert(format!(
                "input0: [{}], input1: [{}], output: [{}]",
                trace_fmt_with_idxs(trace, i, &[9, 10, 11, 12]),
                trace_fmt_with_idxs(trace, i, &[13, 14, 15, 16]),
                trace_fmt_with_idxs(trace, i, &[2, 3, 4, 5]),
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
    let final_check = |at: &AbstractTrace,
                       _n: usize,
                       p: u32,
                       kr: &mut HashSet<String>,
                       ui: &mut UiState,
                       _area: &mut i128| {
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
        let mut new_constraint_info = constraint_info.clone();
        if args.range_interval != 0 {
            use rand::prelude::IndexedRandom;
            use rand::seq::SliceRandom;
            use zebra::solver::RangeType;
            let mut rng = StdRng::seed_from_u64(search_config.seed);
            let op_b = if opcode_str == "ADD" {
                vec![9, 10, 11, 12]
            } else {
                vec![2, 3, 4, 5]
            };
            let op_c = vec![13, 14, 15, 16];
            let b = op_b.choose(&mut rng).unwrap();
            let c = op_c.choose(&mut rng).unwrap();
            let b_lo = rng.random_range(0..(255 - args.range_interval));
            let c_lo = rng.random_range(0..(255 - args.range_interval));

            new_constraint_info.refinable_cols.push(*b);
            new_constraint_info.refinable_cols.push(*c);
            new_constraint_info.range_types.insert(
                *b,
                RangeType::Any(b_lo as i128, (b_lo + args.range_interval) as i128),
            );
            new_constraint_info.range_types.insert(
                *c,
                RangeType::Any(c_lo as i128, (c_lo + args.range_interval) as i128),
            );
            search_config.minimum_num_taregt_cols = new_constraint_info.refinable_cols.len();
        }
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
        let mut known_solution = HashSet::new();
        let result = experiment_harness(
            &program_info,
            &mut new_constraint_info,
            &search_config,
            &base_abs_main_trace_data,
            vec![],
            &if args.blocking_closure && args.range_interval == 0 { vec![0usize] } else { vec![] },
            nop_post_process,
            final_check,
            &args.method,
            &mut known_solution,
        );
        println!("({} {}), {:?}", x, y, result);
        ds.push(result.unwrap());
    }
    let report = generate_report(&ds);
    println!("{:?}", report);
    let _ = write_output(args, search_config, report);

    Ok(())
}
