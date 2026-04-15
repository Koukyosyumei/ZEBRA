use clap::Parser;
use core::mem::transmute;
use rand::seq::SliceRandom;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io;
use std::rc::Rc;

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

const ALL_BUG_CLASSES: &[&str] = &[
    "Empty",
    "UnassignedOpcodeFlags",
    "NonExclusiveOpcodeFlags",
    "ContinueAfterStop",
    "OverFlowWord",
    "TerminateBeforeStop",
];

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
        imm_program::<BabyBear>,
        alu_program::<BabyBear>,
        jal_program::<BabyBear>,
        branch_program::<BabyBear>,
    ];
    let f = fs.choose(rng).unwrap();
    f(rng)
}

/// Per-program comparison record produced by `--benchmark` mode.
#[derive(Debug, Default)]
struct BenchmarkRecord {
    program_id: usize,
    heuristic_found: bool,
    dfs_found: bool,
    /// Trial ID (global expansion count) when the first bug was found, if any.
    heuristic_first_trial: Option<usize>,
    dfs_first_trial: Option<usize>,
    heuristic_total_trials: usize,
    dfs_total_trials: usize,
    heuristic_time_ms: u64,
    dfs_time_ms: u64,
    heuristic_bugs: usize,
    dfs_bugs: usize,
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
    // Bug classes confirmed across all programs
    let mut global_found_classes: HashSet<String> = HashSet::new();

    // Benchmark mode: accumulates one record per program for the final report.
    let mut benchmark_records: Vec<BenchmarkRecord> = Vec::new();

    // Print benchmark table header before the loop so progress is visible immediately.
    if args.benchmark {
        println!(
            "\n{:<4}  {:^8} {:^8}  {:^8} {:^8}  {:^8} {:^8}  {:^7} {:^7}  {:^6} {:^6}",
            "Prog", "H.Found", "D.Found",
            "H.1stTr", "D.1stTr",
            "H.Trials", "D.Trials",
            "H.ms", "D.ms",
            "H.Bugs", "D.Bugs",
        );
        println!("{}", "-".repeat(90));
    }

    for i in 0..args.num_trial {
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

        search_config.seed = i as u64;
        search_config.min_row_id = 0;
        search_config.max_row_id = base_abs_main_trace_data.len() - 1;

        // ######################## Set Info ##########################################
        let program_info = ProgramInfo {
            program_str: program_str,
            program_len: program.len(),
        };
        // Apply the per-program PC range to constraint_info once; variants clone from it.
        constraint_info
            .range_types
            .insert(1, RangeType::Any(0, program.len() as i128 - 1));

        let repr_sets =
            cpu_canonicalizer(&AbstractTrace::new(base_abs_main_trace_data.clone()), prime);
        let honest_repr = format!("{}", PrettySet(repr_sets.clone()));

        if args.benchmark {
            // -------- Benchmark branch: run both variants and collect metrics --------
            let mut record = BenchmarkRecord { program_id: i, ..Default::default() };

            for &use_heuristic in &[true, false] {
                let mut cfg = search_config.clone();
                cfg.enable_heuristic = use_heuristic;

                // Clone constraint_info so each variant starts from identical state.
                let mut ci = constraint_info.clone();

                let mut known: HashSet<String> = HashSet::new();
                known.insert(honest_repr.clone());
                let init_known_size = known.len();

                // Capture the trial ID (global expansion count) of the first bug found.
                let first_trial: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));
                let ft = Rc::clone(&first_trial);

                let bench_final_check =
                    move |trace: &AbstractTrace, num_trial: usize, prime: u32,
                          known_report: &mut HashSet<String>, _ui: &mut UiState,
                          _area: &mut i128| {
                        let mut reprs = cpu_canonicalizer(trace, prime);
                        let mut types: HashSet<String> = HashSet::new();
                        check_bug_type(trace, &mut reprs, &mut types, prime);
                        let repr = format!("{}", PrettySet(reprs));
                        if !known_report.contains(&repr) {
                            known_report.insert(repr);
                            if !types.is_empty() {
                                let mut g = ft.borrow_mut();
                                if g.is_none() {
                                    *g = Some(num_trial);
                                }
                            }
                        }
                    };

                let adj = make_pc_adjuster(&program_table);
                let post = move |trace: &mut AbstractTrace, p: u32| -> MayBeFlag {
                    adj(trace, p);
                    memory_check(trace, p).1
                };

                let res = experiment_harness(
                    &program_info,
                    &mut ci,
                    &cfg,
                    base_abs_main_trace_data,
                    public_vals.clone(),
                    &vec![],
                    post,
                    bench_final_check,
                    &args.method,
                    &mut known,
                    true, // always suppress TUI in benchmark mode
                )
                .unwrap();

                let bugs_found = known.len().saturating_sub(init_known_size);
                let first = *first_trial.borrow();

                if use_heuristic {
                    record.heuristic_found = bugs_found > 0;
                    record.heuristic_first_trial = first;
                    record.heuristic_total_trials = res.num_total_trials;
                    record.heuristic_time_ms = res.execution_time.as_millis() as u64;
                    record.heuristic_bugs = bugs_found;
                } else {
                    record.dfs_found = bugs_found > 0;
                    record.dfs_first_trial = first;
                    record.dfs_total_trials = res.num_total_trials;
                    record.dfs_time_ms = res.execution_time.as_millis() as u64;
                    record.dfs_bugs = bugs_found;
                }
            }

            // Print one row per program.
            println!(
                "{:<4}  {:^8} {:^8}  {:^8} {:^8}  {:^8} {:^8}  {:^7} {:^7}  {:^6} {:^6}",
                record.program_id,
                if record.heuristic_found { "yes" } else { "no" },
                if record.dfs_found { "yes" } else { "no" },
                record.heuristic_first_trial.map_or("-".to_string(), |v| v.to_string()),
                record.dfs_first_trial.map_or("-".to_string(), |v| v.to_string()),
                record.heuristic_total_trials,
                record.dfs_total_trials,
                record.heuristic_time_ms,
                record.dfs_time_ms,
                record.heuristic_bugs,
                record.dfs_bugs,
            );

            benchmark_records.push(record);
        } else {
            // -------- Normal (non-benchmark) branch: existing behaviour --------
            let adjust_pc_program = make_pc_adjuster(&program_table);
            let post_process = move |trace: &mut AbstractTrace, prime: u32| -> MayBeFlag {
                adjust_pc_program(trace, prime);
                memory_check(trace, prime).1
            };

            // Per-program bug tracker: bug class -> list of malicious trace representations
            let bug_class_map: Rc<RefCell<HashMap<String, HashSet<String>>>> =
                Rc::new(RefCell::new(HashMap::new()));
            let bug_class_map_ref = Rc::clone(&bug_class_map);

            // Seed known_solution with the honest trace so it is not reported as malicious
            let mut known_solution: HashSet<String> = global_found_classes
                .iter()
                .cloned()
                .collect();
            known_solution.insert(honest_repr.clone());

            let final_check_fn =
                move |trace: &AbstractTrace, _num_trial: usize, prime: u32,
                      known_report: &mut HashSet<String>, ui: &mut UiState, _area: &mut i128| {
                    let mut record_reprs = cpu_canonicalizer(trace, prime);
                    let mut bug_types: HashSet<String> = HashSet::new();
                    check_bug_type(trace, &mut record_reprs, &mut bug_types, prime);

                    let trace_repr = format!("{}", PrettySet(record_reprs.clone()));
                    if !known_report.contains(&trace_repr) {
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

                        // Associate this trace with its bug classes
                        let mut map = bug_class_map_ref.borrow_mut();
                        if bug_types.is_empty() {
                            map.entry("Unknown".to_string())
                                .or_default()
                                .insert(trace_repr.clone());
                        } else {
                            for bt in &bug_types {
                                map.entry(bt.clone())
                                    .or_default()
                                    .insert(trace_repr.clone());
                            }
                        }
                        drop(map);
                    }

                    save_repr_if_unique(&PrettySet(record_reprs), known_report, ui);
                };

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
                final_check_fn,
                &args.method,
                &mut known_solution,
                args.turn_off_ui,
            );
            println!("{:?}", result);

            // Per-Program Report
            let map = bug_class_map.borrow();
            if map.is_empty() {
                println!("No malicious traces found for this program.");
            } else {
                println!("\nHonest Trace:\n{}\n", honest_repr);
                let total_traces: usize = map.values().map(|v| v.len()).sum();
                println!(
                    "=== Malicious Traces: {} trace(s) across {} bug class(es) ===\n",
                    total_traces,
                    map.len()
                );
                let ordered_classes: Vec<&str> = ALL_BUG_CLASSES
                    .iter()
                    .copied()
                    .chain(std::iter::once("Unknown"))
                    .collect();
                for class in &ordered_classes {
                    if let Some(traces) = map.get(*class) {
                        println!(
                            "[{}]  ({} trace{})",
                            class,
                            traces.len(),
                            if traces.len() == 1 { "" } else { "s" }
                        );
                        for (j, t) in traces.iter().enumerate() {
                            println!("  -- Trace {} --\n{}", j + 1, t);
                        }
                        println!();
                        global_found_classes.insert(class.to_string());
                    }
                }
            }
            println!("=============\n\n");
        }
    }

    // ######################## Benchmark Summary ##################################
    if args.benchmark && !benchmark_records.is_empty() {
        let n = benchmark_records.len() as f64;

        let h_success = benchmark_records.iter().filter(|r| r.heuristic_found).count();
        let d_success = benchmark_records.iter().filter(|r| r.dfs_found).count();

        // Mean first-trial over programs where that variant found at least one bug.
        let h_first_trials: Vec<f64> = benchmark_records
            .iter()
            .filter_map(|r| r.heuristic_first_trial.map(|t| t as f64))
            .collect();
        let d_first_trials: Vec<f64> = benchmark_records
            .iter()
            .filter_map(|r| r.dfs_first_trial.map(|t| t as f64))
            .collect();
        let mean_or_na = |v: &[f64]| {
            if v.is_empty() {
                "N/A".to_string()
            } else {
                format!("{:.1}", v.iter().sum::<f64>() / v.len() as f64)
            }
        };

        let h_mean_trials: f64 =
            benchmark_records.iter().map(|r| r.heuristic_total_trials as f64).sum::<f64>() / n;
        let d_mean_trials: f64 =
            benchmark_records.iter().map(|r| r.dfs_total_trials as f64).sum::<f64>() / n;

        let h_mean_ms: f64 =
            benchmark_records.iter().map(|r| r.heuristic_time_ms as f64).sum::<f64>() / n;
        let d_mean_ms: f64 =
            benchmark_records.iter().map(|r| r.dfs_time_ms as f64).sum::<f64>() / n;

        let h_mean_bugs: f64 =
            benchmark_records.iter().map(|r| r.heuristic_bugs as f64).sum::<f64>() / n;
        let d_mean_bugs: f64 =
            benchmark_records.iter().map(|r| r.dfs_bugs as f64).sum::<f64>() / n;

        println!("\n{}", "=".repeat(60));
        println!("Benchmark Summary  ({} programs)", benchmark_records.len());
        println!("{}", "=".repeat(60));
        println!("{:<22}  {:>12}  {:>12}", "Metric", "Heuristic", "Blind-DFS");
        println!("{}", "-".repeat(60));
        println!(
            "{:<22}  {:>12}  {:>12}",
            "Success rate",
            format!("{}/{}", h_success, benchmark_records.len()),
            format!("{}/{}", d_success, benchmark_records.len()),
        );
        println!(
            "{:<22}  {:>12}  {:>12}",
            "Mean 1st-bug trial",
            mean_or_na(&h_first_trials),
            mean_or_na(&d_first_trials),
        );
        println!(
            "{:<22}  {:>12.1}  {:>12.1}",
            "Mean total trials", h_mean_trials, d_mean_trials
        );
        println!(
            "{:<22}  {:>12.1}  {:>12.1}",
            "Mean wall time (ms)", h_mean_ms, d_mean_ms
        );
        println!(
            "{:<22}  {:>12.2}  {:>12.2}",
            "Mean bugs found", h_mean_bugs, d_mean_bugs
        );
        println!("{}", "=".repeat(60));

        // Win/tie/lose counts
        let h_wins = benchmark_records.iter().filter(|r| {
            match (r.heuristic_first_trial, r.dfs_first_trial) {
                (Some(h), Some(d)) => h < d,
                (Some(_), None) => true,
                _ => false,
            }
        }).count();
        let d_wins = benchmark_records.iter().filter(|r| {
            match (r.heuristic_first_trial, r.dfs_first_trial) {
                (Some(h), Some(d)) => d < h,
                (None, Some(_)) => true,
                _ => false,
            }
        }).count();
        let ties = benchmark_records.len() - h_wins - d_wins;
        println!(
            "First-bug race: Heuristic wins {}, Blind-DFS wins {}, Ties/N/A {}",
            h_wins, d_wins, ties
        );
        println!("{}", "=".repeat(60));
    }

    // ######################## Global Bug Class Summary ##########################
    println!("{}", "=".repeat(55));
    println!("Global Bug Class Summary");
    println!("{}", "=".repeat(55));
    if global_found_classes.is_empty() {
        println!("  Found   : none");
    } else {
        let mut found: Vec<&str> = ALL_BUG_CLASSES
            .iter()
            .copied()
            .filter(|c| global_found_classes.contains(*c))
            .collect();
        if global_found_classes.contains("Unknown") {
            found.push("Unknown");
        }
        println!("  Found   : {}", found.join(", "));
    }
    let not_found: Vec<&str> = ALL_BUG_CLASSES
        .iter()
        .copied()
        .filter(|c| !global_found_classes.contains(*c))
        .collect();
    println!(
        "  Missing : {}",
        if not_found.is_empty() {
            "none".to_string()
        } else {
            not_found.join(", ")
        }
    );
    println!("{}", "=".repeat(55));

    Ok(())
}
