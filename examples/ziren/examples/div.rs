use clap::Parser;
use core::mem::transmute;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashSet;
use std::io;

use p3_koala_bear::KoalaBear;

use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::alu::{DivRemCols, NUM_DIVREM_COLS};
use zkm_core_machine::DivRemChip;

use latticevm::canonicalizer::save_repr_if_unique;
use latticevm::quick::{
    experiment_harness, generate_report, load_config, write_output, Args, ProgramInfo,
};
use latticevm::solver::RangeType;
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn};
use latticevm::trace::{trace_fmt_with_idxs, AbstractTrace};
use latticevm::ui::UiState;
use latticevm::utils::{create_or_clear_dir, indices_arr};

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
    let opcode_str = args.opcode_str.clone();
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
    let (mut constraint_info, _general_lookup_info) =
        extract_constraints_and_range::<KoalaBear, DivRemChip>(&air, NUM_DIVREM_COLS, prime);
    constraint_info
        .refinable_cols
        .extend(&output_columns.clone());
    constraint_info.output_columns = output_columns.clone();
    for i in &output_columns {
        constraint_info.range_types.insert(*i, RangeType::U8);
    }
    if search_config.minimum_num_taregt_cols == 0 {
        search_config.minimum_num_taregt_cols = 3; //constraint_info.refinable_cols.len();
    }

    let mut rng = StdRng::seed_from_u64(search_config.seed);
    let mut ds = vec![];
    for _ in 0..30 {
        let x: u32 = rng.random();
        let y: u32 = rng.random_range(1..u32::MAX);

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
            dummy_adjust_pc_program,
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
