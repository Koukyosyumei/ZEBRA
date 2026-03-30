use clap::Parser;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashSet;
use std::io;

use p3_baby_bear::BabyBear;

use valida_alu_u32::sub::Sub32Instruction;
use valida_basic_api::BasicMachine;
use valida_cpu::Imm32Instruction;
use valida_cpu::StopInstruction;
use valida_machine::{Instruction, InstructionWord, Operands, StarkField};
use valida_memory::columns::{MEM_COL_MAP, NUM_MEM_COLS};
use valida_memory::MemoryChip;
use valida_opcodes::BYTES_PER_INSTR;

use zebra::canonicalizer::save_repr_if_unique;
use zebra::interval::{AbstractInterval, MayBeFlag};
use zebra::quick::{
    experiment_harness, generate_report, load_config, write_output, Args, ProgramInfo,
};
use zebra::solver::nop_post_process;
use zebra::trace::AbstractTrace;
use zebra::ui::UiState;
use zebra::utils::{create_or_clear_dir, PrettySet};

use zebra_valida::config::MyConfig;
use zebra_valida::utils::{extract_constraints_and_range, generate_bootstrap_trace_from_program};

fn reconstruct_word(row: &[AbstractInterval], base: usize) -> AbstractInterval {
    let mut val = AbstractInterval::from_i64(0);
    let mut mul = 1_i64;
    for i in 0..4 {
        val = val + row[base + i].clone() * AbstractInterval::from_i64(mul);
        mul *= 256;
    }
    val
}

fn final_check(
    trace: &AbstractTrace,
    _num_trial: usize,
    prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
    _area: &mut i128,
) {
    let num_row = trace.data.len();
    let def_interval = AbstractInterval::zero();
    let mut memory = std::collections::HashMap::<i64, AbstractInterval>::new();
    let mut record_reprs = HashSet::new();

    for i in 0..num_row {
        let addr = trace.data[i][12].clone();
        let value = reconstruct_word(&trace.data[i], 4);
        let is_read = trace.data[i][14].clone() + trace.data[i][15].clone();
        let is_write = &trace.data[i][16];

        if is_read.is_zero(prime) != MayBeFlag::True {
            for a in addr.lo..(addr.hi + 1) {
                let prev_value = memory.get(&a).unwrap_or(&def_interval);
                if (value.clone() - prev_value.clone()).is_zero(prime) != MayBeFlag::True {
                    record_reprs.insert(format!(
                        "row {}: addr: {}, expected: {}, got: {}, is_read: {}, is_write: {}",
                        i, addr, prev_value, value, is_read, is_write
                    ));
                }
            }
        }

        if is_write.is_zero(prime) != MayBeFlag::True {
            for a in addr.lo..(addr.hi + 1) {
                memory.insert(a, value.clone());
            }
        }
    }

    if !record_reprs.is_empty() {
        save_repr_if_unique(&PrettySet(record_reprs), known_reprt, ui);
    }
}

fn get_target_program<Val: StarkField>(a: i32, b: i32) -> Vec<InstructionWord<i32>> {
    let _bytes_per_instr = BYTES_PER_INSTR as i32;
    let a_bytes = a.to_le_bytes();

    let mut program = vec![];
    program.extend([
        InstructionWord {
            opcode: <Imm32Instruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands([
                -4,
                a_bytes[0] as i32,
                a_bytes[1] as i32,
                a_bytes[2] as i32,
                a_bytes[3] as i32,
            ]),
        },
        InstructionWord {
            opcode: <Sub32Instruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands([-8, -4, b, 0, 1]),
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
    search_config.enable_heuristic = !args.no_heuristic;
    search_config.enable_interval_refinement = !args.no_refinement;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(27) + 1;

    // ######################## Extract Memory Constraints #######################
    println!("MEM AIR MAP");
    println!("  {:?}", MEM_COL_MAP);

    let air = MemoryChip::default();
    let num_col = NUM_MEM_COLS;
    let chip_idx = 2;

    let machine = BasicMachine::<BabyBear>::default();
    let (mut constraint_info, _general_lookup_info) =
        extract_constraints_and_range::<BasicMachine<BabyBear>, MyConfig, _>(
            &machine, &air, num_col, prime, args.method == "bb",
        );
    if search_config.minimum_num_taregt_cols == 0 {
        search_config.minimum_num_taregt_cols = constraint_info.refinable_cols.len();
    }

    let mut rng = StdRng::seed_from_u64(search_config.seed);
    let mut ds = vec![];
    for _ in 0..args.num_trial {
        let mut new_constraint_info = constraint_info.clone();

        let x: i32 = rng.r#gen_range(-0x3C000000..0x3C000000);
        let y: i32 = rng.r#gen_range(-0x3C000000..0x3C000000);

        // ######################## Program Initialization ###########################
        let program = get_target_program::<BabyBear>(x, y);
        let program_str = program
            .iter()
            .map(|inst| format!("{}\n", inst))
            .collect::<String>();
        let base_abs_main_trace_data =
            generate_bootstrap_trace_from_program(&program, chip_idx, 0, 0x1000).1;

        // ######################## Set Info ##########################################
        let program_info = ProgramInfo {
            program_str: program_str,
            program_len: program.len(),
        };

        // ######################## Solve ############################################
        let mut known_solution = HashSet::new();
        let result = experiment_harness(
            &program_info,
            &mut new_constraint_info,
            &search_config,
            &base_abs_main_trace_data,
            vec![],
            &if args.blocking_closure && args.range_interval == 0 { vec![0usize] } else { vec![] },
            nop_post_process,
            final_check,
            &args.method,
            &mut known_solution,
        );
        println!("({} {}), {:?}", x, y, result);
        ds.push(result.unwrap());
    }
    let report = generate_report(&ds);
    println!("{:?}", report);
    let _ = write_output(args, search_config, report);

    Ok(())
}
