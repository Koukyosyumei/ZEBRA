use clap::Parser;
use core::mem::transmute;
use rand::prelude::IndexedRandom;
use rand::seq::SliceRandom;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashSet;
use std::io;

use p3_baby_bear::BabyBear;

use sp1_core_executor::syscalls::SyscallCode;
use sp1_core_executor::{Instruction, Opcode, Program};
use sp1_core_machine::{
    cpu::columns::{CPU_COL_MAP, NUM_CPU_COLS},
    cpu::CpuChip,
};
use sp1_stark::air::SP1_PROOF_NUM_PV_ELTS;

use zebra::canonicalizer::save_repr_if_unique;
use zebra::interval::AbstractInterval as AI;
use zebra::interval::MayBeFlag;
use zebra::memory::IntervalMemory;
use zebra::memory::{check_memory_consistency, reconstruct_word as rec_word};
use zebra::quick::{experiment_harness, load_config, load_config_for_args, print_sparsity_report, Args, ProgramInfo};
use zebra::state::AbstractState;
use zebra::trace::AbstractTrace;
use zebra::ui::{pad_dummy_rows_with_last_dummy, UiState};
use zebra::utils::create_or_clear_dir;
use zebra::utils::PrettySet;

use zebra_sp1::pv_constraints::get_pv_constraints;
use zebra_sp1::utils::{extract_constraints_and_range, generate_abstract_trace, get_program_str};

fn clk(row: &[AI]) -> AI {
    row[1].clone() + row[2].clone() * AI::from_i128(2_usize.pow(16) as i128)
}

fn get_memory(trace: &AbstractTrace, prime: u32) -> Vec<(AI, AI, AI, bool)> {
    let mut ops = vec![];
    for row in &trace.data {
        if MayBeFlag::True != row[56].is_zero(prime) {
            if MayBeFlag::True != row[17].is_non_zero(prime) {
                ops.push((clk(row), row[8].clone(), rec_word(row, 29, 4), true));
            }
            if MayBeFlag::True != row[18].is_non_zero(prime) {
                ops.push((clk(row), rec_word(row, 9, 4), rec_word(row, 38, 4), false));
            }
            if MayBeFlag::True != row[19].is_non_zero(prime) {
                ops.push((clk(row), rec_word(row, 13, 4), rec_word(row, 47, 4), false));
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
        if MayBeFlag::True != row[56].is_zero(prime) {
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

fn opcode_from_u8(value: u8) -> Option<Opcode> {
    match value {
        0 => Some(Opcode::ADD),
        1 => Some(Opcode::SUB),
        2 => Some(Opcode::XOR),
        3 => Some(Opcode::OR),
        4 => Some(Opcode::AND),
        //5 => Some(Opcode::SLL),
        //6 => Some(Opcode::SRL),
        //7 => Some(Opcode::SRA),
        8 => Some(Opcode::SLT),
        9 => Some(Opcode::SLTU),
        10 => Some(Opcode::MUL),
        11 => Some(Opcode::MULH),
        12 => Some(Opcode::MULHU),
        13 => Some(Opcode::MULHSU),
        14 => Some(Opcode::DIV),
        15 => Some(Opcode::DIVU),
        /*
        16 => Some(Opcode::REM),
        17 => Some(Opcode::REMU),*/
        _ => None,
    }
}
pub fn get_random_target_program(pc_start: u32, pc_base: u32, rng: &mut StdRng) -> Program {
    let mut instructions = vec![];
    // SP1 ECALL reads syscall code from X5 (t0 = register 5), arg1 from X10 (a0), arg2 from X11.
    // Result is written back to X5.
    instructions.extend(vec![
        Instruction::new(Opcode::ADD, 5, 0, SyscallCode::HINT_LEN as u32, false, true),
        Instruction::new(Opcode::ADD, 10, 0, 0, false, true),
        Instruction::new(Opcode::ECALL, 5, 10, 11, false, false),
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
    let mut search_config = load_config_for_args(&args);
    search_config.enable_heuristic = !args.no_heuristic;
    search_config.enable_interval_refinement = !args.no_refinement;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(27) + 1;
    let program_cols = (8..35).collect::<Vec<_>>();
    let num_extracted_rows = 8;

    // ######################## Extract CPU Constraints ##########################
    let air = CpuChip::default();
    let air_name = "Cpu";
    println!("{:?}", CPU_COL_MAP);

    let (mut constraint_info, general_lookup_info) =
        extract_constraints_and_range::<BabyBear, CpuChip>(
            &air,
            NUM_CPU_COLS,
            prime,
            args.method == "bb" && !args.no_simplify,
        );
    constraint_info
        .refinable_cols
        .retain(|x| !program_cols.contains(x));
    let (pv_pos_constraints, pv_neg_constraints) = get_pv_constraints();
    constraint_info.constraints.pv_pos_constraints = pv_pos_constraints;
    constraint_info.constraints.pv_neg_constraints = pv_neg_constraints;

    // ######################## Program Initialization ###########################
    if args.sparsity {
        print_sparsity_report(&constraint_info, "Cpu");
        return Ok(());
    }
    let mut rng = StdRng::seed_from_u64(search_config.seed);
    for i in 0..args.num_trial {
        search_config.seed = i as u64;
        let pc_offset = 4; //prime - 4; //get_random_offset(prime, &mut rng);
        let program = get_random_target_program(pc_offset, pc_offset, &mut rng);

        let mut base_abs_main_trace_data =
            generate_abstract_trace(&program, air_name.to_string(), num_extracted_rows);

        // Widen the HINT_LEN result in the ECALL row to [0, 255] per byte.
        // op_a_access.access.value (columns 29..33) holds the value written back to t0 (X5).
        // HINT_LEN is non-deterministic: its return value is unconstrained by the CPU AIR.
        let ecall_row = 2;
        if base_abs_main_trace_data.len() > ecall_row {
            for &col in CPU_COL_MAP.op_a_access.access.value.0.iter() {
                base_abs_main_trace_data[ecall_row][col] = AI::u8();
            }
        }

        let pad_fn = pad_dummy_rows_with_last_dummy(general_lookup_info.clone());
        let post_process = move |trace: &mut AbstractTrace, prime: u32| -> MayBeFlag {
            pad_fn(trace, prime);
            memory_check(trace, prime).1
        };

        // ######################## Public Values ####################################
        let mut public_vals = vec![AI::zero(); SP1_PROOF_NUM_PV_ELTS];
        public_vals[40] = AI::from_i128(pc_offset as i128);
        public_vals[41] = AI::zero();
        public_vals[44] = AI::one();

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
