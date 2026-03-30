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

use zebra::canonicalizer::save_repr_if_unique;
use zebra::quick::{
    experiment_harness, generate_report, load_config, write_output, Args, ProgramInfo,
};
use zebra::solver::nop_post_process;
use zebra::solver::RangeType;
use zebra::trace::trace_fmt_with_idxs;
use zebra::trace::AbstractTrace;
use zebra::ui::UiState;
use zebra::utils::PrettySet;
use zebra::utils::{create_or_clear_dir, indices_arr};

use zebra_sp1::utils::{extract_constraints_and_range, generate_abstract_trace, get_program_str};

// ############## Final Check Function ##############################
fn final_check(
    trace: &AbstractTrace,
    _num_trial: usize,
    _prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
    _area: &mut i128,
) {
    let mut record_reprs = HashSet::new();
    for i in 0..trace.data.len() {
        let string_representation = format!(
            "pc: [{}], next_pc: [{}], op_a_value: [{}], op_b_value: [{}], op_c_value: [{}]",
            trace_fmt_with_idxs(trace, i, &[0, 1, 2, 3]),
            trace_fmt_with_idxs(trace, i, &[5, 6, 7, 8]),
            trace_fmt_with_idxs(trace, i, &[10, 11, 12, 13]),
            trace_fmt_with_idxs(trace, i, &[14, 15, 16, 17]),
            trace_fmt_with_idxs(trace, i, &[18, 19, 20, 21]),
        );
        record_reprs.insert(string_representation);
    }
    save_repr_if_unique(&PrettySet(record_reprs), known_reprt, ui);
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
    let opcode_str = args.opcode_str.clone();
    let mut search_config = load_config(&args.config).unwrap();
    search_config.enable_heuristic = !args.no_heuristic;
    search_config.enable_interval_refinement = !args.no_refinement;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(27) + 1;

    // ######################## Extract CPU Constraints ##########################
    let air = BranchChip::default();
    let air_name = "Branch";
    let _colmap = make_col_map();
    //println!("{:?}", colmap);

    let (mut constraint_info, _general_lookup_info) =
        extract_constraints_and_range::<BabyBear, BranchChip>(&air, NUM_BRANCH_COLS, prime, args.method == "bb" && !args.no_simplify);
    //println!("{:?}", general_lookup_info);
    let output_columns = vec![5, 6, 7, 8];
    for i in vec![5, 6, 7, 8] {
        constraint_info.range_types.insert(i, RangeType::U8);
    }
    constraint_info.range_types.insert(8, RangeType::U7);

    constraint_info
        .refinable_cols
        .extend(&output_columns.clone());
    constraint_info.output_columns = output_columns.clone();
    if search_config.minimum_num_taregt_cols == 0 {
        search_config.minimum_num_taregt_cols = constraint_info.refinable_cols.len();
    }

    let mut rng = StdRng::seed_from_u64(search_config.seed);
    let mut ds = vec![];
    for i in 0..args.num_trial {
        search_config.seed += i as u64;

        let mut new_constraint_info = constraint_info.clone();
        if args.range_interval != 0 {
            use rand::prelude::IndexedRandom;
            use rand::seq::SliceRandom;
            let mut rng = StdRng::seed_from_u64(search_config.seed);
            let op_b = vec![14, 15, 16, 17];
            let op_c = vec![18, 19, 20, 21];
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
        let z: u32 = rng.random_range(0..prime);

        // ######################## Program Initialization ###########################
        let program = target_program(get_opcode(&opcode_str), 4, 4, x, y, z);
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
        println!("({} {} {}), {:?}", x, y, z, result);
        ds.push(result.unwrap());
    }
    let report = generate_report(&ds);
    println!("{:?}", report);
    let _ = write_output(args, search_config, report);

    Ok(())
}
