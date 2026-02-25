use clap::Parser;
use core::mem::transmute;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashSet;
use std::fs;
use std::io;

use p3_koala_bear::KoalaBear;

use zkm_core_executor::syscalls::SyscallCode;
use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::{
    cpu::columns::{CPU_COL_MAP, NUM_CPU_COLS},
    CpuChip,
};
use zkm_stark::MachineProver;
use zkm_stark::ZKM_PROOF_NUM_PV_ELTS;

use latticevm::canonicalizer::save_repr_if_unique;
use latticevm::constraint::eval_constraints;
use latticevm::interval::AbstractInterval;
use latticevm::interval::MayBeFlag;
use latticevm::memory::check_memory_consistency;
use latticevm::quick::{
    experiment_harness, generate_report, load_config, write_output, Args, ProgramInfo,
};
use latticevm::solver::dummy_program_counter_refine_fn;
use latticevm::state::AbstractState;
use latticevm::trace::AbstractTrace;
use latticevm::ui::{pad_dummy_rows_with_last_dummy, UiState};
use latticevm::utils::create_or_clear_dir;

use latticevm_ziren::utils::get_pv_constraints;
use latticevm_ziren::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

pub fn ziren_abstract_trace_to_abstract_state(
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

fn reconstruct_word(row: &[AbstractInterval], base: usize) -> AbstractInterval {
    let mut val = AbstractInterval::from_i128(0);
    let mut mul = 1_i128;
    for i in 0..4 {
        val = val + row[base + i].clone() * AbstractInterval::from_i128(mul);
        mul *= 256;
    }
    val
}

fn memory_check(trace: &AbstractTrace, prime: u32) -> MayBeFlag {
    let mut ops = vec![];
    for row in &trace.data {
        if let MayBeFlag::True = row[65].is_zero(prime) {
        } else {
            if let MayBeFlag::True = row[18].is_non_zero(prime) {
            } else {
                let addr = row[9].clone();
                let val = reconstruct_word(
                    &[
                        row[26].clone(),
                        row[27].clone(),
                        row[28].clone(),
                        row[29].clone(),
                    ],
                    0,
                );
                ops.push((addr, val, true));
            }
            if let MayBeFlag::True = row[19].is_non_zero(prime) {
            } else {
                let addr = reconstruct_word(
                    &[
                        row[10].clone(),
                        row[11].clone(),
                        row[12].clone(),
                        row[13].clone(),
                    ],
                    0,
                );
                let val = reconstruct_word(
                    &[
                        row[47].clone(),
                        row[48].clone(),
                        row[49].clone(),
                        row[50].clone(),
                    ],
                    0,
                );
                ops.push((addr, val, false));
            }
            if let MayBeFlag::True = row[20].is_non_zero(prime) {
            } else {
                let addr = reconstruct_word(
                    &[
                        row[14].clone(),
                        row[15].clone(),
                        row[16].clone(),
                        row[17].clone(),
                    ],
                    0,
                );
                let val = reconstruct_word(
                    &[
                        row[56].clone(),
                        row[57].clone(),
                        row[58].clone(),
                        row[59].clone(),
                    ],
                    0,
                );
                ops.push((addr, val, false));
            }
        }
    }

    check_memory_consistency(&ops)
}

// ############## Final Check Function ##############################
fn final_check(
    trace: &AbstractTrace,
    num_trial: usize,
    prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    let mut string_representation = String::new();
    let mut recovered_states = vec![];
    for row in &trace.data {
        if let MayBeFlag::False = row[65].is_zero(prime) {
            recovered_states.push(ziren_abstract_trace_to_abstract_state(&row, prime));
        }
    }
    string_representation.push_str("Malicious States:\n");
    for rs in &recovered_states {
        string_representation.push_str(&format!("\t{}\n", rs));
    }
    string_representation.push_str("-----------------\n");

    save_repr_if_unique(&string_representation, known_reprt, ui);
}

pub fn target_program(pc_start: u32, pc_base: u32) -> Program {
    // this program is expected to invalid according to the semantics of ziren, while
    // we can find the satisfying solution.
    let mut instructions = vec![Instruction::new(Opcode::ADD, 1, 5, 3, false, true)];
    instructions.extend(vec![
        Instruction::new(Opcode::ADD, 2, 0, SyscallCode::HALT as u32, false, true),
        Instruction::new(Opcode::ADD, 4, 0, 0, false, true),
        Instruction::new(Opcode::SYSCALL, 2, 4, 5, false, false),
    ]);

    Program::new(instructions, pc_start, pc_base)
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    let args = Args::parse();
    let mut search_config = load_config(&args.config).unwrap();

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;
    let program_cols = (8..35).collect::<Vec<_>>();

    // ######################## Solver Parameters ###############################
    let max_iteration = 100000;
    let min_row_id = 0;
    let max_row_id = 3;
    let num_extracted_rows = 5;
    let seed = 41;

    // ######################## Extract CPU Constraints ##########################
    let air = CpuChip::default();
    let air_name = "Cpu";
    println!("{:?}", CPU_COL_MAP);

    let (mut constraint_info, general_lookup_info) =
        extract_constraints_and_range::<KoalaBear, CpuChip>(&air, NUM_CPU_COLS, prime);
    constraint_info
        .refinable_cols
        .retain(|x| !program_cols.contains(x));
    constraint_info.refinable_cols.push(65);

    let (pv_pos_constraints, pv_neg_constraints) = get_pv_constraints();
    constraint_info.constraints.pv_pos_constraints = pv_pos_constraints;
    constraint_info.constraints.pv_neg_constraints = pv_neg_constraints;

    // ######################## Program Initialization ###########################
    let program = target_program(2130706433 - 8, 2130706433 - 8);
    let base_abs_main_trace_data =
        generate_abstract_trace(&program, air_name.to_string(), num_extracted_rows);

    // ######################## Public Values ####################################
    let mut public_vals = vec![AbstractInterval::zero(); ZKM_PROOF_NUM_PV_ELTS];
    public_vals[40] = AbstractInterval::from_i128(2130706433 - 8);
    public_vals[41] = AbstractInterval::zero();
    public_vals[44] = AbstractInterval::one();
    let refinment_target_indicies_pv: Vec<usize> = vec![];

    let pad_fn = pad_dummy_rows_with_last_dummy(general_lookup_info.clone());
    let post_process = move |trace: &mut AbstractTrace, prime: u32| -> MayBeFlag {
        pad_fn(trace, prime);
        memory_check(trace, prime)
    };

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
