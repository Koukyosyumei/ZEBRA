use clap::Parser;
use core::mem::transmute;
use std::io;

use p3_koala_bear::KoalaBear;

use pico_vm::chips::chips::alu::sr::columns::ShiftRightCols;
use pico_vm::chips::chips::alu::sr::traces::ShiftRightChip;
use pico_vm::compiler::riscv::program::Program;
use pico_vm::compiler::riscv::{instruction::Instruction, opcode::Opcode};

use latticevm::canonicalizer::generate_alu_final_checker;
use latticevm::quick::{experiment_harness, load_config, Args, ProgramInfo};
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn};
use latticevm::utils::{create_or_clear_dir, indices_arr};

const NUM_SLR_COLS: usize = size_of::<ShiftRightCols<u8>>();

use latticevm_pico::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

const fn make_col_map() -> ShiftRightCols<usize> {
    let indices_arr = indices_arr::<{ NUM_SLR_COLS }>();
    unsafe { transmute::<[usize; NUM_SLR_COLS], ShiftRightCols<usize>>(indices_arr) }
}

pub fn target_program(pc_start: u32, pc_base: u32, x: u32, y: u32) -> Program {
    let instructions = vec![Instruction::new(Opcode::SRL, 1, x, y, true, true)];
    Program::new(instructions, pc_start, pc_base)
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    let args = Args::parse();
    let _opcode_str = args.opcode_str;
    let mut search_config = load_config(&args.config).unwrap();

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;

    // ######################## Extract CPU Constraints ##########################
    let air: ShiftRightChip<KoalaBear> = ShiftRightChip::default();
    let air_name = "ShiftRight";
    let _colmap = make_col_map();

    let (mut constraint_info, general_lookup_info) = extract_constraints_and_range::<
        KoalaBear,
        ShiftRightChip<KoalaBear>,
    >(&air, NUM_SLR_COLS, prime);
    let final_check = generate_alu_final_checker(general_lookup_info.clone());
    constraint_info
        .refinable_cols
        .extend(&general_lookup_info.op_a);
    constraint_info.output_columns = general_lookup_info.op_a;
    if search_config.minimum_num_taregt_cols == 0 {
        search_config.minimum_num_taregt_cols = 3; //constraint_info.refinable_cols.len();
    }

    let mut rng = StdRng::seed_from_u64(search_config.seed);
    let mut ds = vec![];
    for _ in 0..30 {
        let x: u32 = rng.random();
        let y: u32 = rng.random();

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4, x, y);
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
        ds.push(result.unwrap());
    }
    let report = generate_report(&ds);
    println!("{:?}", report);
    let _ = write_output(args, search_config, report);

    Ok(())
}
