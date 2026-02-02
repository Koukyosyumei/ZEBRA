use core::mem::transmute;
use std::io;

use p3_koala_bear::KoalaBear;

use pico_vm::chips::chips::alu::divrem::columns::{DivRemCols, NUM_DIVREM_COLS};
use pico_vm::chips::chips::alu::divrem::DivRemChip;
use pico_vm::compiler::riscv::program::Program;
use pico_vm::compiler::riscv::{instruction::Instruction, opcode::Opcode};

use latticevm::interval::AbstractInterval;
use latticevm::quick::quick_api;
use latticevm::smt::expr_to_smt_bv;
use latticevm::solver::RangeType;
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn};
use latticevm::symbolic::eval_constraints;
use latticevm::symbolic::AbstractTrace;
use latticevm::symbolic::LatticeVMConstraints;
use latticevm::ui::generate_alu_final_checker;
use latticevm::utils::create_or_clear_dir;
use latticevm::utils::indices_arr;

use latticevm_pico::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

const fn make_col_map() -> DivRemCols<usize> {
    let indices_arr = indices_arr::<{ NUM_DIVREM_COLS }>();
    unsafe { transmute::<[usize; NUM_DIVREM_COLS], DivRemCols<usize>>(indices_arr) }
}

pub fn target_program(opcode: Opcode, pc_start: u32, pc_base: u32, x: u32, y: u32) -> Program {
    let instructions = vec![Instruction::new(opcode, 1, x, y, true, true)];
    Program::new(instructions, pc_start, pc_base)
}

pub fn get_opcode_addsub(target_opcode: &str) -> Opcode {
    match target_opcode {
        "DIV" => Opcode::DIV,
        "DIVU" => Opcode::DIVU,
        "REM" => Opcode::REM,
        "REMU" => Opcode::REMU,
        _ => panic!("unsupported instruction"),
    }
}

fn main() -> Result<(), io::Error> {
    let target_opcode = "DIV";

    create_or_clear_dir("voutput")?;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;

    // ######################## Solver Parameters ###############################
    let max_iteration = 100000000;
    let min_row_id = 0;
    let max_row_id = 0;
    let num_extracted_rows = 1;
    let seed = 41;

    // ######################## Extract CPU Constraints ##########################
    let air: DivRemChip<KoalaBear> = DivRemChip::default();
    let air_name = "DivRem";
    let colmap = make_col_map();
    println!("-- {:?}", colmap);

    let (
        air_constraints,
        lookup_constraints,
        mut refinable_cols,
        mut range_types,
        general_lookup_info,
    ) = extract_constraints_and_range::<KoalaBear, DivRemChip<KoalaBear>>(
        &air,
        NUM_DIVREM_COLS,
        prime,
    );
    for i in &general_lookup_info.alu_output {
        range_types.insert(*i, RangeType::U8);
    }

    let final_check = generate_alu_final_checker(general_lookup_info.clone());
    refinable_cols.extend(&general_lookup_info.alu_output);

    let constraints = LatticeVMConstraints {
        air_constraints,
        lookup_constraints,
        pv_pos_constraints: vec![],
        pv_neg_constraints: vec![],
    };
    let minimum_num_taregt_cols = 2;

    // ######################## Program Initialization ###########################
    let program = target_program(get_opcode_addsub(&target_opcode), 4, 4, 13, 3);
    let base_abs_main_trace_data =
        generate_abstract_trace(&program, air_name.to_string(), num_extracted_rows);

    let mut constants: Vec<(usize, usize, AbstractInterval)> = vec![];
    let mut neg_constants: Vec<(usize, usize, AbstractInterval)> = vec![];
    for j in 0..NUM_DIVREM_COLS {
        if !refinable_cols.contains(&j) {
            constants.push((0, j, base_abs_main_trace_data[0][j].clone()));
        }
    }
    for j in general_lookup_info.alu_output {
        neg_constants.push((0, j, base_abs_main_trace_data[0][j].clone()));
    }
    let smt_str = expr_to_smt_bv(
        &constraints,
        &constants,
        &neg_constants,
        &range_types,
        1,
        NUM_DIVREM_COLS,
        0,
        prime,
    );
    println!("{}", smt_str);
    println!("rr: {:?}", range_types);

    let v = [
        255, 255, 255, 255, 13, 0, 0, 0, 3, 0, 0, 0, 255, 255, 255, 255, 16, 0, 0, 0, 16,
        2130706433, 2130706433, 0, 3, 4261412866, 4261412866, 0, 4261412869, 4227858436,
        4261412866, 2130706433, 253, 255, 255, 255, 255, 255, 255, 255, 1, 1, 1, 1, 1, 1, 1, 1,
        721420288, 0, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 0, 0, 0, 1157627904, 0, 0, 1, 0, 1,
        16646144, 0, 0, 0, 0, 809500672, 0, 16711423, 0, 16843009, 0, 2164260864, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 1, 1,
    ];
    let mut a = base_abs_main_trace_data.clone();
    for i in 0..NUM_DIVREM_COLS {
        a[0][i] = AbstractInterval::from_i64(v[i]);
    }
    let at = AbstractTrace::new(a);
    let flag = eval_constraints(&at, None, &constraints, prime);
    println!("{}", constraints.air_constraints[4]);
    println!("{:?}", flag);
    /*
        pub fn eval_constraints(
        trace: &AbstractTrace,
        public_vals: Option<&[AbstractInterval]>,
        constraints: &LatticeVMConstraints,
        prime: u32,
    )
         */

    // ######################## Solve ############################################
    quick_api(
        get_program_str(&program),
        &constraints,
        &refinable_cols,
        &range_types,
        &vec![],
        &base_abs_main_trace_data,
        vec![],
        max_iteration,
        minimum_num_taregt_cols,
        min_row_id,
        max_row_id,
        program.instructions.len(),
        dummy_program_counter_refine_fn,
        dummy_adjust_pc_program,
        final_check,
        prime,
        seed,
    )
}
