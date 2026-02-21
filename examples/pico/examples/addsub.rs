use clap::Parser;
use core::mem::transmute;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashSet;
use std::io;

use p3_koala_bear::KoalaBear;

use pico_vm::chips::chips::alu::add_sub::columns::{AddSubCols, NUM_ADD_SUB_COLS};
use pico_vm::chips::chips::alu::add_sub::AddSubChip;
use pico_vm::compiler::riscv::program::Program;
use pico_vm::compiler::riscv::{instruction::Instruction, opcode::Opcode};

use latticevm::quick::{experiment_harness, load_config, mean_variance, Args, ProgramInfo};
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn};
use latticevm::trace::{trace_fmt_with_idxs, AbstractTrace};
use latticevm::utils::{create_or_clear_dir, indices_arr};
use latticevm::{canonicalizer::save_repr_if_unique, ui::UiState};

use latticevm_pico::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

fn cr_add(trace: &AbstractTrace) -> String {
    format!(
        "input0: [{}], input1: [{}], output: [{}]",
        trace_fmt_with_idxs(trace, 0, &[7, 8, 9, 10]),
        trace_fmt_with_idxs(trace, 0, &[11, 12, 13, 14]),
        trace_fmt_with_idxs(trace, 0, &[0, 1, 2, 3]),
    )
}

fn cr_sub(trace: &AbstractTrace) -> String {
    format!(
        "input0: [{}], input1: [{}], output: [{}]",
        trace_fmt_with_idxs(trace, 0, &[0, 1, 2, 3]),
        trace_fmt_with_idxs(trace, 0, &[11, 12, 13, 14]),
        trace_fmt_with_idxs(trace, 0, &[7, 8, 9, 10]),
    )
}

const fn make_col_map() -> AddSubCols<usize> {
    let indices_arr = indices_arr::<{ NUM_ADD_SUB_COLS }>();
    unsafe { transmute::<[usize; NUM_ADD_SUB_COLS], AddSubCols<usize>>(indices_arr) }
}

pub fn target_program(opcode: Opcode, pc_start: u32, pc_base: u32, x: u32, y: u32) -> Program {
    let instructions = vec![Instruction::new(opcode, 1, x, y, true, true)];
    Program::new(instructions, pc_start, pc_base)
}

pub fn get_opcode_addsub(target_opcode: &str) -> Opcode {
    match target_opcode {
        "ADD" => Opcode::ADD,
        "SUB" => Opcode::SUB,
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
    let cr = if opcode_str == "ADD" { cr_add } else { cr_sub };
    let final_check =
        |at: &AbstractTrace, _n: usize, _p: u32, kr: &mut HashSet<String>, ui: &mut UiState| {
            save_repr_if_unique(&cr(at), kr, ui);
        };

    // ######################## Extract CPU Constraints ##########################
    let air: AddSubChip<KoalaBear> = AddSubChip::default();
    let air_name = "AddSub";
    let _colmap = make_col_map();

    let output_columns = if opcode_str == "ADD" {
        vec![0, 1, 2, 3]
    } else {
        vec![7, 8, 9, 10]
    };

    let (mut constraint_info, _general_lookup_info) = extract_constraints_and_range::<
        KoalaBear,
        AddSubChip<KoalaBear>,
    >(&air, NUM_ADD_SUB_COLS, prime);
    constraint_info
        .refinable_cols
        .extend(&output_columns.clone());

    /*
    use latticevm::solver::RangeType;
    constraint_info.refinable_cols.push(7);
    constraint_info.refinable_cols.push(11);
    constraint_info.range_types.insert(7, RangeType::U8);
    constraint_info.range_types.insert(11, RangeType::U8);*/

    constraint_info.output_columns = output_columns.clone();
    if search_config.minimum_num_taregt_cols == 0 {
        search_config.minimum_num_taregt_cols = constraint_info.refinable_cols.len();
    }

    let mut rng = StdRng::seed_from_u64(search_config.seed);
    let mut ds = vec![];
    for _ in 0..30 {
        let x: u32 = rng.random();
        let y: u32 = rng.random();

        // ######################## Program Initialization ###########################
        let program = target_program(get_opcode_addsub(&opcode_str), 4, 4, x, y);
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
            dummy_program_counter_refine_fn,
            dummy_adjust_pc_program,
            final_check,
            &args.method,
        );
        println!("({} {}), {:?}", x, y, result);
        ds.push(result.unwrap().0.execution_time);
    }
    println!("{:?}", mean_variance(&ds));

    Ok(())
}
