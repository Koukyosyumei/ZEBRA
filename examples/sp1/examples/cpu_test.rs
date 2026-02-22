use clap::Parser;
use core::mem::transmute;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashSet;
use std::fs;
use std::io;

use p3_baby_bear::BabyBear;

use sp1_core_executor::syscalls::SyscallCode;
use sp1_core_executor::{Instruction, Opcode, Program};
use sp1_core_machine::{
    cpu::columns::{CPU_COL_MAP, NUM_CPU_COLS},
    cpu::CpuChip,
};
use sp1_stark::air::SP1_PROOF_NUM_PV_ELTS;

use latticevm::constraint::eval_constraints;
use latticevm::interval::AbstractInterval;
use latticevm::interval::MayBeFlag;
use latticevm::quick::{experiment_harness, load_config, mean_variance, Args, ProgramInfo};
use latticevm::solver::dummy_program_counter_refine_fn;
use latticevm::state::AbstractState;
use latticevm::trace::AbstractTrace;
use latticevm::ui::{pad_dummy_rows_with_last_dummy, UiState};
use latticevm::utils::create_or_clear_dir;

use latticevm_sp1::pv_constraints::get_pv_constraints;
use latticevm_sp1::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

pub fn target_program(pc_start: u32, pc_base: u32) -> Program {
    // this program is expected to invalid according to the semantics of ziren, while
    // we can find the satisfying solution.
    let mut instructions = vec![Instruction::new(Opcode::ADD, 1, 5, 3, false, true)];
    instructions.extend(vec![
        Instruction::new(Opcode::ADD, 2, 0, SyscallCode::HALT as u32, false, true),
        Instruction::new(Opcode::ADD, 4, 0, 0, false, true),
        Instruction::new(Opcode::ECALL, 2, 4, 5, false, false),
    ]);

    Program::new(instructions, pc_start, pc_base)
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(27) + 1;

    // ######################## Solver Parameters ###############################
    let num_extracted_rows = 5;

    // ######################## Extract CPU Constraints ##########################
    let air = CpuChip::default();
    let air_name = "Cpu";
    println!("{:?}", CPU_COL_MAP);

    let (mut constraint_info, general_lookup_info) =
        extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, prime);

    // ######################## Program Initialization ###########################
    let program = target_program(4, 4);
    let base_abs_main_trace_data =
        generate_abstract_trace(&program, air_name.to_string(), num_extracted_rows);

    let mut public_vals = vec![AbstractInterval::zero(); SP1_PROOF_NUM_PV_ELTS];
    public_vals[40] = AbstractInterval::from_i64(4);
    public_vals[41] = AbstractInterval::zero();
    public_vals[44] = AbstractInterval::one();

    let at = AbstractTrace::new(base_abs_main_trace_data);
    let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, prime);
    println!("{:?}", result);

    Ok(())
}
