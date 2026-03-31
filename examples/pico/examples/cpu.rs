use clap::Parser;
use rand::prelude::IndexedRandom;
use rand::seq::SliceRandom;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashSet;
use std::io;

use p3_koala_bear::KoalaBear;

use pico_vm::chips::chips::riscv_cpu::{
    columns::{CpuCols, CPU_COL_MAP, NUM_CPU_COLS},
    CpuChip,
};
use pico_vm::compiler::riscv::{instruction::Instruction, opcode::Opcode, program::Program};
use pico_vm::emulator::riscv::syscalls::SyscallCode;
use pico_vm::primitives::consts::RISCV_NUM_PVS;

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

use zebra_pico::utils::{extract_constraints_and_range, generate_abstract_trace, get_program_str};

// ############## Column Index Constants ##############################
//
// CpuCols<T> layout (total NUM_CPU_COLS = 117):
//   [0]  chunk
//   [1]  clk  (full 24-bit)
//   [2]  clk_16bit_limb
//   [3]  clk_8bit_limb
//   [4]  pc
//   [5]  next_pc
//   [6..19]  instruction (InstructionCols, 14 cols):
//              opcode=6, op_a=[7-10], op_b=[11-14], op_c=[15-18], op_a_0=19
//   [20..41] opcode_selector (22 cols):
//              imm_b=20, imm_c=21, is_alu=22, is_ecall=23,
//              is_lb..is_sw=24-31, is_beq..is_bgeu=32-37,
//              is_jalr=38, is_jal=39, is_auipc=40, is_unimpl=41
//   [42..54] op_a_access (MemoryReadWriteCols, 13 cols):
//              prev_value=[42-45], access.value=[46-49],
//              prev_chunk=50, prev_clk=51, compare_clk=52,
//              diff_16bit_limb=53, diff_8bit_limb=54
//   [55..63] op_b_access (MemoryReadCols, 9 cols):
//              access.value=[55-58], prev_chunk=59, prev_clk=60,
//              compare_clk=61, diff_16bit_limb=62, diff_8bit_limb=63
//   [64..72] op_c_access (MemoryReadCols, 9 cols):
//              access.value=[64-67], prev_chunk=68, prev_clk=69,
//              compare_clk=70, diff_16bit_limb=71, diff_8bit_limb=72
//   [73..110] opcode_specific (OpcodeSpecificCols / JumpCols union, 38 cols)
//   [111] is_real
//   [112] branching
//   [113] not_branching
//   [114] ecall_mul_send_to_table
//   [115] ecall_range_check_operand
//   [116] is_sequential_instr

/// Reconstruct a synthetic clock value from its 16-bit and 8-bit limbs.
fn clk(row: &[AI]) -> AI {
    row[2].clone() + row[3].clone() * AI::from_i128(2_usize.pow(16) as i128)
}

/// Collect all register-file read/write operations visible in the trace.
///
/// Returns `(clk, addr, value, is_write)` tuples:
/// - op_a: write, skipped when op_a_0 = 1 (destination is x0).
/// - op_b: read,  skipped when imm_b = 1 (operand is an immediate).
/// - op_c: read,  skipped when imm_c = 1 (operand is an immediate).
fn get_memory(trace: &AbstractTrace, prime: u32) -> Vec<(AI, AI, AI, bool)> {
    let mut ops = vec![];
    for row in &trace.data {
        if MayBeFlag::True != row[111].is_zero(prime) {
            // op_a: write to dest register (skip if dest == x0)
            if MayBeFlag::True != row[19].is_non_zero(prime) {
                ops.push((clk(row), row[7].clone(), rec_word(row, 46, 4), true));
            }
            // op_b: read from source register (skip if immediate)
            if MayBeFlag::True != row[20].is_non_zero(prime) {
                ops.push((clk(row), row[11].clone(), rec_word(row, 55, 4), false));
            }
            // op_c: read from source register (skip if immediate)
            if MayBeFlag::True != row[21].is_non_zero(prime) {
                ops.push((clk(row), row[15].clone(), rec_word(row, 64, 4), false));
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
        if MayBeFlag::True != row[111].is_zero(prime) {
            record_reprs
                .insert(format!("\tins: (clk: {}, pc: {})", clk(row), row[4].clone()).to_string());
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
    let rw_ops_wo_clk: Vec<(AI, AI, bool)> = ops.into_iter().map(|x| (x.1, x.2, x.3)).collect();
    check_memory_consistency(&rw_ops_wo_clk)
}

// ############## Program Generation ##############################

fn opcode_from_index(value: u8) -> Option<Opcode> {
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

/// Build a minimal random program: one random ALU instruction followed by a halt.
///
/// Halt is performed by setting a0 (x10) = 0 and executing ECALL.
/// The HALT syscall code in pico is 0 (pico_vm::primitives::consts::HALT).
pub fn get_random_target_program(pc_start: u32, pc_base: u32, rng: &mut StdRng) -> Program {
    let v = vec![0, 1, 2, 3, 4, 8, 9, 10, 11, 12, 13, 14, 15];
    let mut instructions = vec![Instruction::new(
        opcode_from_index(*v.choose(rng).unwrap()).unwrap(),
        rng.random_range(0..32), // dest register
        rng.random(),
        rng.random(),
        true, // imm_b
        true, // imm_c
    )];
    // Load HALT syscall code (0) into a0 (x10), then ECALL.
    instructions.extend(vec![
        Instruction::new(Opcode::ADD, 5, 0, SyscallCode::HALT as u32, false, true),
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

// ############## Main ##############################

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    let args = Args::parse();
    let mut search_config = load_config(&args.config).unwrap();
    search_config.enable_heuristic = !args.no_heuristic;
    search_config.enable_interval_refinement = !args.no_refinement;

    // ######################## Prime and Column Settings ########################
    // KoalaBear: 2^31 - 2^24 + 1
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;

    // Instruction columns (opcode + 3 Word operands + op_a_0) — held fixed.
    let program_cols = (6..20).collect::<Vec<_>>();
    let num_extracted_rows = 8;

    // ######################## Extract CPU Constraints ##########################
    // NOTE: if the chip name printed below does not match "Cpu", update air_name.
    let air: CpuChip<KoalaBear> = CpuChip::default();
    let air_name = "Cpu";
    println!("CPU_COL_MAP: {:?}", CPU_COL_MAP);

    let (mut constraint_info, mut general_lookup_info) =
        extract_constraints_and_range::<KoalaBear, CpuChip<KoalaBear>>(&air, NUM_CPU_COLS, prime, args.method == "bb" && !args.no_simplify);
    constraint_info
        .refinable_cols
        .retain(|x| !program_cols.contains(x));
    // Allow is_real (col 111) to be refined.
    constraint_info.refinable_cols.push(111);
    general_lookup_info.is_real = vec![general_lookup_info.is_real[3].clone()];

    // ######################## Program Initialization ###########################
    let mut rng = StdRng::seed_from_u64(search_config.seed);
    for i in 0..args.num_trial {
        search_config.seed = i as u64;

        let pc_offset = prime - 4; //get_random_offset(prime, &mut rng);
        let program = get_random_target_program(pc_offset, pc_offset, &mut rng);
        println!("{}", get_program_str(&program));

        let base_abs_main_trace_data =
            generate_abstract_trace(&program, air_name.to_string(), num_extracted_rows);
        if base_abs_main_trace_data.is_empty() {
            continue;
        }

        let pad_fn = pad_dummy_rows_with_last_dummy(general_lookup_info.clone());
        let post_process = move |trace: &mut AbstractTrace, prime: u32| -> MayBeFlag {
            pad_fn(trace, prime);
            memory_check(trace, prime).1
        };

        // ######################## Public Values ####################################
        // PublicValues<Word<u8>, u8> layout (RISCV_NUM_PVS bytes):
        //   [0..31]  committed_value_digest ([Word<u8>; 8])
        //   [32..39] deferred_proofs_digest ([u8; 8])
        //   [40]     start_pc
        //   [41]     next_pc
        //   [42]     exit_code
        //   [43]     chunk
        //   [44]     execution_chunk
        //   [45..76] previous_initialize_addr_bits
        //   [77..108] last_initialize_addr_bits
        //   ...
        let mut public_vals = vec![AI::zero(); RISCV_NUM_PVS];
        public_vals[40] = AI::from_i128(pc_offset as i128); // start_pc
        public_vals[41] = AI::zero(); // next_pc (0 after halt)
        public_vals[44] = AI::one(); // execution_chunk = 1

        // ######################## Set Info ##########################################
        let program_info = ProgramInfo {
            program_str: get_program_str(&program),
            program_len: program.instructions.len(),
        };

        /*
        use zebra::constraint::eval_constraints;
        let at = AbstractTrace::new(base_abs_main_trace_data.clone());
        let e = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, prime);
        println!("{:?}", e);
        println!("{}", at);
        */

        // ######################## Solve ############################################
        let mut known_solution = HashSet::new();
        let result = experiment_harness(
            &program_info,
            &mut constraint_info,
            &search_config,
            &base_abs_main_trace_data,
            public_vals,
            &if args.blocking_closure && args.range_interval == 0 { vec![0usize] } else { vec![] },
            post_process,
            final_check,
            &args.method,
            &mut known_solution,
            args.turn_off_ui,
        );
        println!("{:?}", result);
        if known_solution.len() > 1 {
            break;
        }
    }

    Ok(())
}
