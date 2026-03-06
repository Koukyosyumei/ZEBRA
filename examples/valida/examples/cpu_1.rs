use clap::Parser;
use core::mem::transmute;
use rand::seq::SliceRandom;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashSet;
use std::io;

use p3_baby_bear::BabyBear;
use p3_field::AbstractField;
//use p3_uni_stark::symbolic_builder::get_symbolic_constraints;

use valida_alu_u32::add::Add32Instruction;
use valida_basic_api::BasicMachine;
use valida_cpu::BneInstruction;
use valida_cpu::Imm32Instruction;
use valida_cpu::StopInstruction;
use valida_cpu::{
    columns::{CPU_COL_MAP, NUM_CPU_COLS},
    CpuChip,
};
use valida_machine::{Instruction, InstructionWord, Operands, StarkField};
use valida_opcodes::BYTES_PER_INSTR;

use latticevm::canonicalizer::save_repr_if_unique;
use latticevm::interval::AbstractInterval as AI;
use latticevm::interval::AbstractInterval;
use latticevm::interval::MayBeFlag;
use latticevm::memory::IntervalMemory;
use latticevm::memory::{check_memory_consistency, reconstruct_word as rec_word};
use latticevm::quick::{experiment_harness, load_config, Args, ProgramInfo};
use latticevm::solver::RangeType;
use latticevm::state::AbstractState;
use latticevm::trace::AbstractTrace;
use latticevm::ui::{pad_dummy_rows_with_last_dummy, UiState};
use latticevm::utils::create_or_clear_dir;
use latticevm::utils::PrettySet;
use valida_opcodes::Opcode;

use latticevm_valida::config::MyConfig;
use latticevm_valida::utils::{
    extract_constraints_and_range, generate_bootstrap_trace_from_program, make_pc_adjuster,
    refine_pc_interval,
};

fn get_memory(trace: &AbstractTrace, prime: u32) -> Vec<(AI, AI, AI, bool)> {
    let mut ops = vec![];
    for row in &trace.data {
        if MayBeFlag::True != row[58].is_zero(prime) {
            if MayBeFlag::False != row[42].is_non_zero(prime) {
                ops.push((row[0].clone(), row[43].clone(), rec_word(row, 44, 4), true));
            }
            if MayBeFlag::False != row[30].is_non_zero(prime) {
                ops.push((row[0].clone(), row[31].clone(), rec_word(row, 32, 4), false));
            }
            if MayBeFlag::False != row[36].is_non_zero(prime) {
                ops.push((row[0].clone(), row[37].clone(), rec_word(row, 38, 4), false));
            }
        }
    }

    ops
}

fn memory_check(trace: &AbstractTrace, prime: u32) -> (IntervalMemory, MayBeFlag) {
    let ops = get_memory(trace, prime);
    let rw_ops_wo_clk: Vec<(AI, AI, bool)> =
        ops.into_iter().map(|x| (x.1, x.2, x.3)).clone().collect();

    check_memory_consistency(&rw_ops_wo_clk)
}

fn check_bug_type(
    trace: &AbstractTrace,
    record_reprs: &mut HashSet<String>,
    bug_types: &mut HashSet<String>,
    prime: u32,
) {
    if record_reprs.is_empty() {
        record_reprs.insert("\tEmpty".to_string());
        bug_types.insert("Empty".to_string());
    }

    for i in 0..trace.data.len() {
        let mut ai = AbstractInterval::zero();
        for j in 9..27 {
            ai = ai.clone() + trace.data[i][j].clone();
        }
        if MayBeFlag::True != trace.data[i][58].is_zero(prime) {
            if MayBeFlag::True == ai.is_zero(prime) {
                bug_types.insert("UnassignedOpcodeFlags".to_string());
            } else if MayBeFlag::False == (ai - AbstractInterval::one()).is_zero(prime) {
                bug_types.insert("NonExclusiveOpcodeFlags".to_string());
            }

            if i > 0 {
                if MayBeFlag::False == trace.data[i - 1][24].is_zero(prime) {
                    bug_types.insert("ContinueAfterStop".to_string());
                }
            }

            if MayBeFlag::True != trace.data[i][42].is_zero(prime) {
                let v = trace.data[i][44].clone()
                    + trace.data[i][45].clone()
                    + trace.data[i][46].clone()
                    + trace.data[i][47].clone();
                if v.lo > prime as i128 {
                    bug_types.insert("OverFlowWord".to_string());
                }
            }
        }

        if MayBeFlag::True == trace.data[i][58].is_zero(prime) {
            if i > 0 {
                if MayBeFlag::True == trace.data[i - 1][24].is_zero(prime) {
                    bug_types.insert("TerminateBeforeStop".to_string());
                }
            }
        }
    }
}

// ############## Final Check Function ##############################
fn final_check(
    trace: &AbstractTrace,
    num_trial: usize,
    prime: u32,
    known_report: &mut HashSet<String>,
    ui: &mut UiState,
) {
    let mut record_reprs = HashSet::new();
    for row in &trace.data {
        if MayBeFlag::True != row[58].is_zero(prime) {
            record_reprs
                .insert(format!("\tins: (clk: {}, pc: {})", row[0], row[1].clone()).to_string());
        }
    }
    for ms in &get_memory(trace, prime) {
        if ms.3 {
            record_reprs.insert(
                format!("\tmem: (clk: {}, addr: {}, val: {})", ms.0, ms.1, ms.2).to_string(),
            );
        }
    }

    let mut bug_types: HashSet<String> = HashSet::new();
    check_bug_type(&trace, &mut record_reprs, &mut bug_types, prime);

    let mut is_new = bug_types.is_empty();
    for bt in &bug_types {
        if !known_report.contains(bt) {
            is_new = true;
            known_report.insert(bt.clone());
        }
    }
    if !is_new {
        return;
    }

    save_repr_if_unique(&PrettySet(record_reprs), known_report, ui);
}

fn get_target_program<Val: StarkField>() -> Vec<InstructionWord<i32>> {
    let bytes_per_instr = BYTES_PER_INSTR as i32;

    let mut program = vec![];
    program.extend([
        InstructionWord {
            opcode: <Imm32Instruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands([-4, 2, 0, 0, 0]),
        },
        InstructionWord {
            opcode: <StopInstruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands::default(),
        },
    ]);

    program
}

pub fn generate_random_program(length: usize, rng: &mut StdRng) -> Vec<InstructionWord<i32>> {
    let mut program = Vec::with_capacity(length);

    // Subset of opcodes to use for the random program
    let available_opcodes = [
        Opcode::ADD32,
        Opcode::SUB32,
        Opcode::MUL32,
        Opcode::DIV32,
        Opcode::IMM32,
        Opcode::LOAD32,
        Opcode::STORE32,
        Opcode::JAL,
        Opcode::BEQ,
        Opcode::BNE,
        Opcode::LOADFP,
        Opcode::STOP,
    ];

    for i in 0..length {
        // Typically, a program should terminate with a STOP opcode
        let opcode = if i == length - 1 {
            &Opcode::STOP
        } else {
            available_opcodes.choose(rng).unwrap_or(&Opcode::STOP)
        };

        let mut ops = [0i32; 5]; // Each instruction has 5 operands

        match opcode {
            Opcode::IMM32 => {
                // IMM32 format: [dest_offset, byte0, byte1, byte2, byte3]
                ops[0] = rng.gen_range(-20..20) * 4; // Stack offsets are usually multiples of 4
                for b in 1..5 {
                    ops[b] = rng.gen_range(0..256); // Individual bytes of the immediate value
                }
            }
            Opcode::ADD32 | Opcode::SUB32 | Opcode::MUL32 | Opcode::DIV32 => {
                // ALU format: [dest, src1, src2/imm, ?, is_imm_flag]
                ops[0] = rng.gen_range(-20..20) * 4; // Destination stack offset
                ops[1] = rng.gen_range(-20..20) * 4; // Source 1 stack offset
                ops[4] = rng.gen_range(0..2); // is_imm flag (0 or 1)

                if ops[4] == 1 {
                    ops[2] = rng.gen_range(0..1000); // Direct immediate value
                } else {
                    ops[2] = rng.gen_range(-20..20) * 4; // Source 2 stack offset
                }
            }
            Opcode::STOP => {
                // STOP usually takes no operands
            }
            _ => {
                // Default: fill with small random offsets
                for op in ops.iter_mut() {
                    *op = rng.gen_range(-40..40);
                }
            }
        }

        program.push(InstructionWord {
            opcode: opcode.clone() as u32,
            operands: Operands(ops),
        });
    }

    program
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    let args = Args::parse();
    let mut search_config = load_config(&args.config).unwrap();

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(27) + 1;
    let program_cols = (3..8).collect::<Vec<_>>();

    // ######################## Solver Parameters ###############################
    search_config.max_expansions = 3000;
    search_config.min_row_id = 0;
    search_config.max_row_id = 1;
    search_config.time_out_ms = 100000;
    search_config.seed = 41;

    println!("CPU AIR MAP");
    println!("  {:?}", CPU_COL_MAP);

    let air = CpuChip::default();
    let num_col = NUM_CPU_COLS;
    let chip_idx = 0;

    let machine = BasicMachine::<BabyBear>::default();
    let (mut constraint_info, general_lookup_info) =
        extract_constraints_and_range::<BasicMachine<BabyBear>, MyConfig, _>(
            &machine, &air, num_col, prime,
        );
    constraint_info
        .refinable_cols
        .retain(|x| !program_cols.contains(x));
    constraint_info.refinable_cols.push(58);
    //constraint_info.range_types.insert(19, RangeType::Bool);
    println!("{:?}", constraint_info.refinable_cols);

    let mut public_vals = vec![AI::zero(); 3];
    public_vals[0] = AI::from_i128(0);
    public_vals[1] = AI::from_i128(4096);
    public_vals[2] = AI::from_i128(1);

    search_config.minimum_num_taregt_cols = 3;

    // ######################## Program Initialization ###########################
    //    let program = get_target_program::<BabyBear>();

    let mut rng = StdRng::seed_from_u64(search_config.seed);

    for _ in 0..10 {
        let program = generate_random_program(2, &mut rng);
        let program_str = program
            .iter()
            .map(|inst| format!("{}\n", inst))
            .collect::<String>();

        let result = std::panic::catch_unwind(|| {
            generate_bootstrap_trace_from_program(&program, chip_idx, 0, 0x1000)
        });
        if result.is_err() {
            continue;
        }

        let base_abs_main_trace_data = result.unwrap();
        let adjust_pc_program = make_pc_adjuster(program.clone());

        let post_process = move |trace: &mut AbstractTrace, prime: u32| -> MayBeFlag {
            adjust_pc_program(trace, prime);
            memory_check(trace, prime).1
        };

        // ######################## Set Info ##########################################
        let program_info = ProgramInfo {
            program_str: program_str,
            program_len: program.len(),
        };
        constraint_info
            .range_types
            .insert(1, RangeType::Any(0, program.len() as i128 - 1));

        // ######################## Solve ############################################
        let mut known_solution = HashSet::new();
        let result = experiment_harness(
            &program_info,
            &mut constraint_info,
            &search_config,
            &base_abs_main_trace_data,
            public_vals.clone(),
            &vec![], // vec![0],
            post_process,
            final_check,
            &args.method,
            &mut known_solution,
        );
        println!("{:?}", result);
    }

    Ok(())
}
