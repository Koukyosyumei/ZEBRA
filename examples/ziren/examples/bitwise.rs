use std::io;

use p3_koala_bear::KoalaBear;

use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::alu::NUM_BITWISE_COLS;
use zkm_core_machine::BitwiseChip;
use zkm_stark::MachineProver;

use latticevm::interval::AbstractInterval;
use latticevm::quick::{experiment_harness, quick_api, ProgramInfo, SearchConfig};
use latticevm::smt::expr_to_smt_bv;
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn, make_init_val};
use latticevm::symbolic::{eval_constraints, AbstractTrace, LatticeVMConstraints};
use latticevm::ui::generate_alu_final_checker;
use latticevm::utils::create_or_clear_dir;

use latticevm_ziren::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

pub fn target_program(opcode: Opcode, pc_start: u32, pc_base: u32, x: u32, y: u32) -> Program {
    let instructions = vec![Instruction::new(opcode, 1, 2, 3, true, true)];
    Program::new(instructions, pc_start, pc_base)
}

pub fn get_opcode(opcode_str: &str) -> Opcode {
    match opcode_str {
        "AND" => Opcode::AND,
        "OR" => Opcode::OR,
        "XOR" => Opcode::XOR,
        _ => panic!("unsupported instruction"),
    }
}

fn main() -> Result<(), io::Error> {
    let opcode_str = "AND";

    create_or_clear_dir("voutput")?;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;

    // ######################## Extract CPU Constraints ##########################
    let air = BitwiseChip::default();
    let air_name = "Bitwise";

    let (mut constraint_info, general_lookup_info) =
        extract_constraints_and_range::<KoalaBear, BitwiseChip>(&air, NUM_BITWISE_COLS, prime);
    let final_check = generate_alu_final_checker(general_lookup_info.clone());

    constraint_info
        .refinable_cols
        .extend(&general_lookup_info.alu_output.clone());
    constraint_info.output_columns = general_lookup_info.alu_output.clone();

    // ######################## Program Initialization ###########################
    let program = target_program(get_opcode(&opcode_str), 4, 4, 2, 3);

    // ######################## Set Info ##########################################
    let program_info = ProgramInfo {
        program_str: get_program_str(&program),
        program_len: program.instructions.len(),
    };
    let base_abs_main_trace_data = generate_abstract_trace(&program, air_name.to_string(), 1);
    let search_config = SearchConfig::new(constraint_info.refinable_cols.len());

    // ######################## Solve ############################################
    experiment_harness(
        &program_info,
        &mut constraint_info,
        &search_config,
        &base_abs_main_trace_data,
        vec![],
        &vec![], // vec![0],
        dummy_program_counter_refine_fn,
        dummy_adjust_pc_program,
        final_check,
    )
}
