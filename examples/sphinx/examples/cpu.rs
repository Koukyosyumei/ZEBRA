use clap::Parser;
use rand::prelude::IndexedRandom;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashSet;
use std::io;

use p3_baby_bear::BabyBear;

use sphinx_core::cpu::columns::{CPU_COL_MAP, NUM_CPU_COLS};
use sphinx_core::cpu::CpuChip;
use sphinx_core::runtime::{Instruction, Opcode, Program, SyscallCode};
use sphinx_core::stark::PROOF_MAX_NUM_PVS;

use zebra::canonicalizer::save_repr_if_unique;
use zebra::interval::AbstractInterval as AI;
use zebra::interval::MayBeFlag;
use zebra::memory::IntervalMemory;
use zebra::memory::{check_memory_consistency, reconstruct_word as rec_word};
use zebra::quick::{experiment_harness, load_config, Args, ProgramInfo};
use zebra::trace::AbstractTrace;
use zebra::ui::{pad_dummy_rows_with_last_dummy, UiState};
use zebra::utils::create_or_clear_dir;
use zebra::utils::PrettySet;

use zebra_sphinx::pv_constraints::{get_pv_constraints, ShardPosition};
use zebra_sphinx::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

fn clk(row: &[AI]) -> AI {
    // clk_16bit_limb: 4,  clk_8bit_limb: 5
    row[4].clone() + row[5].clone() * AI::from_i128(2_usize.pow(16) as i128)
}

fn get_memory(trace: &AbstractTrace, prime: u32) -> Vec<(AI, AI, AI, bool)> {
    let mut ops = vec![];
    for row in &trace.data {
        // is_real: 140
        if MayBeFlag::True != row[140].is_zero(prime) {
            // op_a is always written; register address in instruction.op_a (cols 9-12),
            // new value in op_a_access.access.value (cols 64-67)
            ops.push((clk(row), rec_word(row, 9, 4), rec_word(row, 64, 4), true));
            // op_b read when not immediate (imm_b: 38)
            // register address in instruction.op_b (cols 13-16),
            // value in op_b_access.access.value (cols 73-76)
            if MayBeFlag::True != row[38].is_non_zero(prime) {
                ops.push((clk(row), rec_word(row, 13, 4), rec_word(row, 73, 4), false));
            }
            // op_c read when not immediate (imm_c: 39)
            // register address in instruction.op_c (cols 17-20),
            // value in op_c_access.access.value (cols 82-85)
            if MayBeFlag::True != row[39].is_non_zero(prime) {
                ops.push((clk(row), rec_word(row, 17, 4), rec_word(row, 82, 4), false));
            }
        }
    }

    ops
}

// ############## Final Check Function ##############################
fn final_check(
    trace: &AbstractTrace,
    _num_trial: usize,
    prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
    _area: &mut i128,
) {
    let mut record_reprs = HashSet::new();
    for row in &trace.data {
        // is_real: 140,  pc: 6
        if MayBeFlag::True != row[140].is_zero(prime) {
            record_reprs
                .insert(format!("\tins: (clk: {}, pc: {})", clk(row), row[6].clone()).to_string());
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

fn opcode_from_u8(value: u8) -> Option<Opcode> {
    match value {
        0 => Some(Opcode::ADD),
        1 => Some(Opcode::SUB),
        2 => Some(Opcode::XOR),
        3 => Some(Opcode::OR),
        4 => Some(Opcode::AND),
        8 => Some(Opcode::SLT),
        9 => Some(Opcode::SLTU),
        10 => Some(Opcode::MUL),
        11 => Some(Opcode::MULH),
        12 => Some(Opcode::MULHU),
        13 => Some(Opcode::MULHSU),
        _ => None,
    }
}

pub fn get_random_target_program(pc_start: u32, pc_base: u32, rng: &mut StdRng) -> Program {
    let v = vec![0, 1];
    let mut instructions = vec![Instruction::new(
        opcode_from_u8(*v.choose(rng).unwrap()).unwrap(),
        rng.random_range(0..32),
        rng.random(),
        rng.random(),
        true,
        true,
    )];
    instructions.extend(vec![
        Instruction::new(Opcode::ADD, 2, 0, SyscallCode::HALT as u32, false, true),
        Instruction::new(Opcode::ADD, 4, 0, 0, false, true),
        Instruction::new(Opcode::ECALL, 2, 4, 5, false, false),
    ]);

    Program::new(instructions, pc_start, pc_base)
}

pub fn get_random_offset(prime: u32, rng: &mut StdRng) -> u32 {
    let random_value: f64 = rng.random();
    if random_value < 0.4 {
        rng.random_range(0..16)
    } else if random_value < 0.8 {
        rng.random_range((prime - 16)..prime)
    } else {
        rng.random()
    }
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    let args = Args::parse();
    let mut search_config = load_config(&args.config).unwrap();
    search_config.enable_heuristic = !args.no_heuristic;
    search_config.enable_interval_refinement = !args.no_refinement;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(27) + 1;
    let program_cols = (8..60).collect::<Vec<_>>();
    let num_extracted_rows = 8;

    search_config.max_expansions = 3000;
    search_config.time_out_ms = 10000;
    search_config.seed = 41;
    search_config.min_row_id = 0;
    search_config.max_row_id = 6;

    // ######################## Extract CPU Constraints ##########################
    let air = CpuChip::default();
    let air_name = "CPU";
    //println!("{:?}", CPU_COL_MAP);

    let (mut constraint_info, general_lookup_info) =
        extract_constraints_and_range::<BabyBear, CpuChip>(
            &air,
            NUM_CPU_COLS,
            prime,
            args.method == "bb" && !args.no_simplify,
        );
    println!("{}", constraint_info.constraints.air_constraints[336]);
    constraint_info.constraints.air_constraints.remove(336);

    constraint_info
        .refinable_cols
        .retain(|x| !program_cols.contains(x));
    constraint_info.refinable_cols.push(140);
    // Model the sphinx single-shard scenario: first (and only) shard has no halt check.
    let pc_offset = 4; //prime - 2;
    let (pv_pos_constraints, pv_neg_constraints) = get_pv_constraints(ShardPosition::First {
        pc_start: pc_offset,
    });
    constraint_info.constraints.pv_pos_constraints = pv_pos_constraints;
    constraint_info.constraints.pv_neg_constraints = pv_neg_constraints;

    // ######################## Program Initialization ###########################
    let mut rng = StdRng::seed_from_u64(search_config.seed);
    for i in 0..args.num_trial {
        search_config.seed = i as u64;
        let program = get_random_target_program(pc_offset, pc_offset, &mut rng);

        let base_abs_main_trace_data =
            generate_abstract_trace(&program, air_name.to_string(), num_extracted_rows);

        let pad_fn = pad_dummy_rows_with_last_dummy(general_lookup_info.clone());
        let post_process = move |trace: &mut AbstractTrace, prime: u32| -> MayBeFlag {
            pad_fn(trace, prime);
            memory_check(trace, prime).1
        };

        // ######################## Public Values ####################################
        // Sphinx PublicValues layout:
        //   [40] start_pc, [41] next_pc, [42] exit_code, [43] shard
        let mut public_vals = vec![AI::zero(); PROOF_MAX_NUM_PVS];
        public_vals[40] = AI::from_i128(pc_offset as i128);
        public_vals[41] = AI::zero();
        public_vals[43] = AI::one();

        // ######################## Set Info ##########################################
        let program_info = ProgramInfo {
            program_str: get_program_str(&program),
            program_len: program.instructions.len(),
        };

        // ######################## Solve ############################################
        let mut known_solution = HashSet::new();
        let result = experiment_harness(
            &program_info,
            &mut constraint_info,
            &search_config,
            &base_abs_main_trace_data,
            public_vals,
            &if args.blocking_closure && args.range_interval == 0 {
                vec![0usize]
            } else {
                vec![]
            },
            post_process,
            final_check,
            &args.method,
            &mut known_solution,
            args.turn_off_ui,
        );
        println!("{:?}", result);
    }

    Ok(())
}
