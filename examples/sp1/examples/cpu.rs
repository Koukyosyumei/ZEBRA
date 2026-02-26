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

use latticevm::canonicalizer::save_repr_if_unique;
use latticevm::constraint::eval_constraints;
use latticevm::interval::AbstractInterval;
use latticevm::interval::MayBeFlag;
use latticevm::memory::IntervalMemory;
use latticevm::memory::{check_memory_consistency, reconstruct_word as rec_word};
use latticevm::quick::{
    experiment_harness, generate_report, load_config, write_output, Args, ProgramInfo,
};
use latticevm::solver::dummy_program_counter_refine_fn;
use latticevm::state::AbstractState;
use latticevm::trace::AbstractTrace;
use latticevm::ui::{pad_dummy_rows_with_last_dummy, UiState};
use latticevm::utils::create_or_clear_dir;

use latticevm_sp1::pv_constraints::get_pv_constraints;
use latticevm_sp1::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

pub fn sp1_abstract_trace_to_abstract_state(
    abstract_row: &Vec<AbstractInterval>,
    prime: u32,
) -> AbstractState {
    AbstractState {
        clk: abstract_row[1].clone()
            + abstract_row[2].clone() * AbstractInterval::from_i128(2_usize.pow(16) as i128),
        pc: abstract_row[5].clone(),
        is_done: abstract_row[5].clone().is_zero(prime),
        memory_ops: Vec::new(),
    }
}

fn memory_check(trace: &AbstractTrace, prime: u32) -> (IntervalMemory, MayBeFlag) {
    let mut ops = vec![];
    for row in &trace.data {
        if MayBeFlag::True != row[56].is_zero(prime) {
            if MayBeFlag::True != row[17].is_non_zero(prime) {
                ops.push((row[8].clone(), rec_word(row, 29, 4), true));
            }
            if MayBeFlag::True != row[18].is_non_zero(prime) {
                ops.push((rec_word(row, 9, 4), rec_word(row, 38, 4), false));
            }
            if MayBeFlag::True != row[19].is_non_zero(prime) {
                ops.push((rec_word(row, 13, 4), rec_word(row, 47, 4), false));
            }
        }
    }

    check_memory_consistency(&ops)
}

// ############## Final Check Function ##############################
fn final_check(
    trace: &AbstractTrace,
    _num_trial: usize,
    prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    let mut string_representation = String::new();
    let mut recovered_states = vec![];
    for row in &trace.data {
        if let MayBeFlag::True = row[56].is_zero(prime) {
        } else {
            recovered_states.push(sp1_abstract_trace_to_abstract_state(&row, prime));
        }
    }
    string_representation.push_str("**PC Transition**:\n");
    for rs in &recovered_states {
        string_representation.push_str(&format!("\t{}\n", rs));
    }
    string_representation.push_str("\n**Memory**:\n");
    string_representation.push_str(&format!("\t{}", memory_check(trace, prime).0).to_string());

    save_repr_if_unique(&string_representation, known_reprt, ui);
}

pub fn target_program(pc_start: u32, pc_base: u32) -> Program {
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

    let args = Args::parse();
    let mut search_config = load_config(&args.config).unwrap();

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(27) + 1;
    let program_cols = (8..35).collect::<Vec<_>>();

    // ######################## Solver Parameters ###############################
    let min_row_id = 0;
    let max_row_id = 6;
    let num_extracted_rows = 8;

    // ######################## Extract CPU Constraints ##########################
    let air = CpuChip::default();
    let air_name = "Cpu";
    println!("{:?}", CPU_COL_MAP);

    let (mut constraint_info, general_lookup_info) =
        extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, prime);
    constraint_info
        .refinable_cols
        .retain(|x| !program_cols.contains(x));
    let (pv_pos_constraints, pv_neg_constraints) = get_pv_constraints();
    constraint_info.constraints.pv_pos_constraints = pv_pos_constraints;
    constraint_info.constraints.pv_neg_constraints = pv_neg_constraints;

    // ######################## Program Initialization ###########################
    let program = target_program(2013265921 - 8, 2013265921 - 8);
    let base_abs_main_trace_data =
        generate_abstract_trace(&program, air_name.to_string(), num_extracted_rows);

    let pad_fn = pad_dummy_rows_with_last_dummy(general_lookup_info.clone());
    let post_process = move |trace: &mut AbstractTrace, prime: u32| -> MayBeFlag {
        pad_fn(trace, prime);
        memory_check(trace, prime).1
    };

    // ######################## Public Values ####################################
    let mut public_vals = vec![AbstractInterval::zero(); SP1_PROOF_NUM_PV_ELTS];
    public_vals[40] = AbstractInterval::from_i128(2013265921 - 8);
    public_vals[41] = AbstractInterval::zero();
    public_vals[44] = AbstractInterval::one();

    // ######################## Set Info ##########################################
    let program_info = ProgramInfo {
        program_str: get_program_str(&program),
        program_len: program.instructions.len(),
    };
    if search_config.minimum_num_taregt_cols == 0 {
        search_config.time_out_ms = 100000;
        search_config.max_expansions = 50000;
        search_config.minimum_num_taregt_cols = 1; //constraint_info.refinable_cols.len();
        search_config.min_row_id = min_row_id;
        search_config.max_row_id = max_row_id;
    }

    // ######################## Solve ############################################
    let result = experiment_harness(
        &program_info,
        &mut constraint_info,
        &search_config,
        &base_abs_main_trace_data,
        public_vals,
        &vec![], // vec![0],
        post_process,
        final_check,
        &args.method,
    );
    println!("{:?}", result);

    Ok(())
}
