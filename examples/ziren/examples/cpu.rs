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
use latticevm::interval::AbstractInterval as AI;
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
use latticevm::utils::PrettySet;

use latticevm_ziren::utils::get_pv_constraints;
use latticevm_ziren::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

fn clk(row: &[AI]) -> AI {
    row[1].clone() + row[2].clone() * AI::from_i128(2_usize.pow(16) as i128)
}

fn get_memory(trace: &AbstractTrace, prime: u32) -> Vec<(AI, AI, AI, bool)> {
    let mut ops = vec![];
    for row in &trace.data {
        if MayBeFlag::True != row[65].is_zero(prime) {
            if MayBeFlag::True != row[18].is_non_zero(prime) {
                ops.push((clk(row), row[9].clone(), rec_word(row, 26, 4), true));
            }
            if MayBeFlag::True != row[19].is_non_zero(prime) {
                ops.push((clk(row), rec_word(row, 10, 4), rec_word(row, 47, 4), false));
            }
            if MayBeFlag::True != row[20].is_non_zero(prime) {
                ops.push((clk(row), rec_word(row, 14, 4), rec_word(row, 56, 4), false));
            }
        }
    }

    ops
}

// ############## Final Check Function ##############################
fn final_check(
    trace: &AbstractTrace,
    num_trial: usize,
    prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    let mut record_reprs = HashSet::new();
    for row in &trace.data {
        if MayBeFlag::True != row[65].is_zero(prime) {
            record_reprs
                .insert(format!("\tins: (clk: {}, pc: {})", clk(row), row[5].clone()).to_string());
        }
    }
    for ms in &get_memory(trace, prime) {
        if ms.3 {
            record_reprs.insert(
                format!("\tmem: (clk: {}, addr: {}, val: {})", ms.0, ms.1, ms.2).to_string(),
            );
        }
    }

    save_repr_if_unique(&PrettySet(record_reprs), known_reprt, ui);
}

fn memory_check(trace: &AbstractTrace, prime: u32) -> (IntervalMemory, MayBeFlag) {
    let ops = get_memory(trace, prime);
    let rw_ops_wo_clk: Vec<(AI, AI, bool)> =
        ops.into_iter().map(|x| (x.1, x.2, x.3)).clone().collect();
    check_memory_consistency(&rw_ops_wo_clk)
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
    let mut public_vals = vec![AI::zero(); ZKM_PROOF_NUM_PV_ELTS];
    public_vals[40] = AI::from_i128(2130706433 - 8);
    public_vals[41] = AI::zero();
    public_vals[44] = AI::one();
    let refinment_target_indicies_pv: Vec<usize> = vec![];

    let pad_fn = pad_dummy_rows_with_last_dummy(general_lookup_info.clone());
    let post_process = move |trace: &mut AbstractTrace, prime: u32| -> MayBeFlag {
        pad_fn(trace, prime);
        memory_check(trace, prime).1
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
    let mut known_solution = HashSet::new();
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
        &mut known_solution,
    );
    println!("{:?}", result);

    Ok(())
}
