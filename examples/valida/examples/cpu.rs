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
use valida_machine::{Instruction, InstructionWord as IW, Operands, StarkField};
use valida_opcodes::BYTES_PER_INSTR;

use valida_opcodes::Opcode;
use zebra::canonicalizer::save_repr_if_unique;
use zebra::interval::AbstractInterval as AI;
use zebra::interval::AbstractInterval;
use zebra::interval::MayBeFlag;
use zebra::memory::reconstruct_word;
use zebra::memory::IntervalMemory;
use zebra::memory::{check_memory_consistency, reconstruct_word as rec_word};
use zebra::quick::{experiment_harness, load_config, Args, ProgramInfo};
use zebra::solver::RangeType;
use zebra::state::AbstractState;
use zebra::trace::AbstractTrace;
use zebra::ui::{pad_dummy_rows_with_last_dummy, UiState};
use zebra::utils::create_or_clear_dir;
use zebra::utils::PrettySet;

use zebra_valida::config::MyConfig;
use zebra_valida::utils::{
    extract_constraints_and_range, generate_bootstrap_trace_from_program, make_pc_adjuster,
    refine_pc_interval,
};

fn get_memory(trace: &AbstractTrace, prime: u32) -> Vec<(AI, AI, AI, bool)> {
    let mut ops = vec![];
    for row in &trace.data {
        if MayBeFlag::True != row[58].is_zero(prime) {
            if MayBeFlag::False != row[30].is_non_zero(prime) {
                ops.push((row[0].clone(), row[31].clone(), rec_word(row, 32, 4), false));
            }
            if MayBeFlag::False != row[36].is_non_zero(prime) {
                ops.push((row[0].clone(), row[37].clone(), rec_word(row, 38, 4), false));
            }
            if MayBeFlag::False != row[42].is_non_zero(prime) {
                ops.push((row[0].clone(), row[43].clone(), rec_word(row, 44, 4), true));
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

            if (MayBeFlag::True != trace.data[i][20].is_zero(prime))
                || (MayBeFlag::True != trace.data[i][21].is_zero(prime))
            {
                if MayBeFlag::True != trace.data[i][42].is_zero(prime) {
                    let v = reconstruct_word(&trace.data[i], 44, 4);
                    if v.lo > prime as i128 {
                        bug_types.insert("OverFlowWord".to_string());
                    }
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

fn cpu_canonicalizer(trace: &AbstractTrace, prime: u32) -> HashSet<String> {
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

    record_reprs
}

// ############## Final Check Function ##############################
fn final_check(
    trace: &AbstractTrace,
    num_trial: usize,
    prime: u32,
    known_report: &mut HashSet<String>,
    ui: &mut UiState,
    _area: &mut i128,
) {
    let mut record_reprs = cpu_canonicalizer(trace, prime);

    let mut bug_types: HashSet<String> = HashSet::new();
    check_bug_type(&trace, &mut record_reprs, &mut bug_types, prime);

    let string_representation = format!("{}", PrettySet(record_reprs.clone()));
    if !known_report.contains(&string_representation) {
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
    }

    save_repr_if_unique(&PrettySet(record_reprs), known_report, ui);
}

fn imm_program<Val: StarkField>(rng: &mut StdRng) -> Vec<IW<i32>> {
    let x = rng.gen_range(-20..20) * 4;
    let a: i32 = rng.gen_range(-0x3C000000..0x3C000000);

    let ab = a.to_le_bytes();
    println!("{:?}", ab);
    vec![
        IW {
            opcode: Opcode::IMM32 as u32,
            operands: Operands([x, ab[0] as i32, ab[1] as i32, ab[2] as i32, ab[3] as i32]),
        },
        IW {
            opcode: Opcode::STOP as u32,
            operands: Operands::default(),
        },
    ]
}

fn alu_program<Val: StarkField>(rng: &mut StdRng) -> Vec<IW<i32>> {
    let x = rng.gen_range(-20..20) * 4;
    let y = rng.gen_range(-20..20) * 4;
    let a: i32 = rng.gen_range(-0x3C000000..0x3C000000);
    let ab = a.to_le_bytes();
    let b: i32 = rng.gen_range(-0x3C000000..0x3C000000);

    let alu_opcodes = [
        Opcode::ADD32,
        Opcode::SUB32,
        Opcode::MULHU32,
        Opcode::MULHS32,
        Opcode::DIV32,
        Opcode::EQ32,
        Opcode::NE32,
    ];
    let opcode = alu_opcodes.choose(rng).unwrap_or(&Opcode::STOP);

    let mut program = vec![];
    program.extend([
        IW {
            opcode: Opcode::IMM32 as u32,
            operands: Operands([y, ab[0] as i32, ab[1] as i32, ab[2] as i32, ab[3] as i32]),
        },
        IW {
            opcode: opcode.clone() as u32,
            operands: Operands([x, y, b, 0, 1]),
        },
        IW {
            opcode: Opcode::STOP as u32,
            operands: Operands::default(),
        },
    ]);

    program
}

fn jal_program<Val: StarkField>(rng: &mut StdRng) -> Vec<IW<i32>> {
    let x = rng.gen_range(-20..20) * 4;
    let y = rng.gen_range(-20..20) * 4;
    let a: i32 = rng.gen_range(2..4);
    let b: i32 = rng.gen_range(-0x3C000000..0x3C000000);
    let bb = b.to_le_bytes();

    let mut program = vec![];
    program.extend([
        IW {
            opcode: Opcode::IMM32 as u32,
            operands: Operands([x, 0, 0, 0, 0]),
        },
        IW {
            opcode: Opcode::JAL as u32,
            operands: Operands([x, a * (BYTES_PER_INSTR as i32), 0, 0, 0]),
        },
        IW {
            opcode: Opcode::IMM32 as u32,
            operands: Operands([y, bb[0] as i32, bb[1] as i32, bb[2] as i32, bb[3] as i32]),
        },
        IW {
            opcode: Opcode::STOP as u32,
            operands: Operands::default(),
        },
    ]);

    program
}

fn branch_program<Val: StarkField>(rng: &mut StdRng) -> Vec<IW<i32>> {
    let bytes_per_instr = BYTES_PER_INSTR as i32;
    let x = rng.gen_range(-20..1) * 4;
    let mut y = rng.gen_range(-20..1) * 4;
    if x == y {
        y = y - 4;
    }
    let a: i32 = rng.gen_range(1..5);

    let branch_opcodes = [Opcode::BEQ, Opcode::BNE];
    let opcode = branch_opcodes.choose(rng).unwrap_or(&Opcode::STOP);

    let mut program = vec![];
    program.extend([
        IW {
            opcode: Opcode::IMM32 as u32,
            operands: Operands([x, a as i32, 0, 0, 0]),
        },
        IW {
            opcode: Opcode::ADD32 as u32,
            operands: Operands([y, y, 1, 0, 1]),
        },
        IW {
            opcode: opcode.clone() as u32,
            operands: Operands([1 * bytes_per_instr, y, x, 0, 0]),
        },
        IW {
            opcode: Opcode::STOP as u32,
            operands: Operands::default(),
        },
    ]);

    program
}

pub fn generate_random_program(rng: &mut StdRng) -> Vec<IW<i32>> {
    let fs = vec![
        //imm_program::<BabyBear>,
        //alu_program::<BabyBear>,
        jal_program::<BabyBear>,
        //branch_program::<BabyBear>,
    ];
    let f = fs.choose(rng).unwrap();
    f(rng)
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    let args = Args::parse();
    let mut search_config = load_config(&args.config).unwrap();
    search_config.enable_heuristic = !args.no_heuristic;
    search_config.enable_interval_refinement = !args.no_refinement;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(27) + 1;
    let program_cols = (3..8).collect::<Vec<_>>();

    // ######################## Solver Parameters ###############################
    search_config.max_expansions = 3000;
    search_config.time_out_ms = 10000;
    search_config.seed = 41;

    println!("CPU AIR MAP");
    println!("  {:?}", CPU_COL_MAP);

    let air = CpuChip::default();
    let num_col = NUM_CPU_COLS;
    let chip_idx = 0;

    let machine = BasicMachine::<BabyBear>::default();
    let (mut constraint_info, general_lookup_info) =
        extract_constraints_and_range::<BasicMachine<BabyBear>, MyConfig, _>(
            &machine,
            &air,
            num_col,
            prime,
            args.method == "bb" && !args.no_simplify,
        );
    constraint_info
        .refinable_cols
        .retain(|x| !program_cols.contains(x));
    constraint_info.refinable_cols.push(58);

    let mut public_vals = vec![AI::zero(); 3];
    public_vals[0] = AI::from_i128(0);
    public_vals[1] = AI::from_i128(4096);
    public_vals[2] = AI::from_i128(1);

    search_config.minimum_num_taregt_cols = 3;

    // ######################## Program Initialization ###########################
    //    let program = get_target_program::<BabyBear>();

    let mut rng = StdRng::seed_from_u64(search_config.seed);
    let mut global_known_solution = HashSet::new();

    for i in 0..10 {
        //0..args.num_trial {
        println!("\n\n===========");
        let program = generate_random_program(&mut rng);
        let program_str = program
            .iter()
            .map(|inst| format!("{}\n", inst))
            .collect::<String>();
        println!("{}", program_str);

        let result = std::panic::catch_unwind(|| {
            generate_bootstrap_trace_from_program(&program, chip_idx, 0, 0x1000)
        });
        if result.is_err() {
            println!("=============\n\n");
            continue;
        }

        let program_table = result.as_ref().unwrap().0.clone();
        let base_abs_main_trace_data = &result.unwrap().1;

        /*
        println!("{:?}", program);
        use zebra::constraint::eval_constraints;
        let res = eval_constraints(
            &AbstractTrace::new(base_abs_main_trace_data.clone()),
            Some(&public_vals),
            &constraint_info.constraints,
            prime,
        );
        println!("{}", AbstractTrace::new(base_abs_main_trace_data.clone()));
        println!("{:?}", res);
        */

        search_config.seed = i as u64;
        search_config.min_row_id = 0;
        search_config.max_row_id = base_abs_main_trace_data.len() - 1;

        let adjust_pc_program = make_pc_adjuster(&program_table);
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
        let mut known_solution = global_known_solution.clone();
        let repr_sets =
            cpu_canonicalizer(&AbstractTrace::new(base_abs_main_trace_data.clone()), prime);
        known_solution.insert(format!("{}", PrettySet(repr_sets.clone())));

        let result = experiment_harness(
            &program_info,
            &mut constraint_info,
            &search_config,
            &base_abs_main_trace_data,
            public_vals.clone(),
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
        if known_solution.len() > 1 {
            println!("Honest Trace:\n   {}\n", PrettySet(repr_sets));
            println!("Malicious Traces");
            for k in &known_solution {
                println!("  {}\n-------", k);
            }

            for a in vec![
                "Empty",
                "UnassignedOpcodeFlags",
                "NonExclusiveOpcodeFlags",
                "ContinueAfterStop",
                "OverFlowWord",
                "TerminateBeforeStop",
            ] {
                if known_solution.contains(a) {
                    global_known_solution.insert(a.to_string());
                }
            }
        }
        println!("=============\n\n");
    }

    Ok(())
}
