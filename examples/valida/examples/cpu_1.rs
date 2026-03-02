use clap::Parser;
use core::mem::transmute;
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

    save_repr_if_unique(&PrettySet(record_reprs), known_reprt, ui);
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
            opcode: <Add32Instruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands([-8, -8, 1, 0, 1]),
        },
        InstructionWord {
            opcode: <BneInstruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands([1 * bytes_per_instr, -8, -4, 0, 0]),
        },
        InstructionWord {
            opcode: <StopInstruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands::default(),
        },
    ]);

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
    search_config.max_expansions = 30000;
    search_config.min_row_id = 0;
    search_config.max_row_id = 5;
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
    constraint_info.range_types.insert(19, RangeType::Bool);

    let mut public_vals = vec![AI::zero(); 3];
    public_vals[0] = AI::from_i128(0);
    public_vals[1] = AI::from_i128(4096);
    public_vals[2] = AI::from_i128(1);

    search_config.minimum_num_taregt_cols = 1;

    // ######################## Program Initialization ###########################
    let program = get_target_program::<BabyBear>();
    let program_str = program
        .iter()
        .map(|inst| format!("{}\n", inst))
        .collect::<String>();

    let base_abs_main_trace_data =
        generate_bootstrap_trace_from_program(&program, chip_idx, 0, 0x1000);
    let adjust_pc_program = make_pc_adjuster(program.clone());

    let post_process = move |trace: &mut AbstractTrace, prime: u32| -> MayBeFlag {
        adjust_pc_program(trace, prime);
        //memory_check(trace, prime).1
        use latticevm::interval::MayBeFlag::True;
        MayBeFlag::True
    };

    // ######################## Set Info ##########################################
    let program_info = ProgramInfo {
        program_str: program_str,
        program_len: program.len(),
    };

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
