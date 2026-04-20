use clap::Parser;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io;
use std::rc::Rc;

use p3_baby_bear::BabyBear;

use valida_alu_u32::sub::Sub32Instruction;
use valida_basic_api::BasicMachine;
use valida_cpu::Imm32Instruction;
use valida_cpu::StopInstruction;
use valida_machine::{Instruction, InstructionWord, Operands, StarkField};
use valida_memory::columns::NUM_MEM_COLS;
use valida_memory::MemoryChip;
use valida_opcodes::BYTES_PER_INSTR;

use zebra::canonicalizer::save_repr_if_unique;
use zebra::interval::{AbstractInterval, MayBeFlag};
use zebra::quick::{experiment_harness, load_config, Args, ProgramInfo};
use zebra::solver::nop_post_process;
use zebra::trace::AbstractTrace;
use zebra::ui::UiState;
use zebra::utils::{create_or_clear_dir, PrettySet};

use zebra_valida::config::MyConfig;
use zebra_valida::utils::{extract_constraints_and_range, generate_bootstrap_trace_from_program};

fn reconstruct_word(row: &[AbstractInterval], base: usize) -> AbstractInterval {
    let mut val = AbstractInterval::from_i128(0);
    let mut mul = 1_i128;
    for i in 0..4 {
        val = val + row[base + i].clone() * AbstractInterval::from_i128(mul);
        mul *= 256;
    }
    val
}

fn memory_canonicalizer(trace: &AbstractTrace, prime: u32) -> HashSet<String> {
    let mut record_reprs = HashSet::new();
    for row in &trace.data {
        let addr = row[12].clone();
        let clk = row[13].clone();
        let value = reconstruct_word(row, 4);
        let is_read = row[14].clone() + row[15].clone();
        let is_write = &row[16];
        if is_read.is_zero(prime) != MayBeFlag::True || is_write.is_zero(prime) != MayBeFlag::True {
            record_reprs.insert(format!(
                "clk: {}, addr: {}, value: {}, is_read: {}, is_write: {}",
                clk, addr, value, is_read, is_write
            ));
        }
    }
    record_reprs
}

fn check_bug_type(
    trace: &AbstractTrace,
    record_reprs: &mut HashSet<String>,
    bug_types: &mut HashSet<String>,
    prime: u32,
) -> bool {
    /*
    if record_reprs.is_empty() {
        record_reprs.insert("\tEmpty".to_string());
        bug_types.insert("Empty".to_string());
        return;
    }*/

    let def_interval = AbstractInterval::zero();
    let mut last_written: HashMap<i128, AbstractInterval> = HashMap::new();
    let mut ever_written: HashSet<i128> = HashSet::new();
    let mut is_crash = false;

    for row in &trace.data {
        let addr = row[12].clone();
        let value = reconstruct_word(row, 4);
        let is_read = row[14].clone() + row[15].clone();
        let is_write = &row[16];

        if is_read.is_zero(prime) != MayBeFlag::True {
            for a in addr.lo..(addr.hi + 1) {
                if !ever_written.contains(&a) {
                    // First access to this address is a read — memory should be zero-initialized
                    if value.is_zero(prime) != MayBeFlag::True {
                        bug_types.insert("InitialReadNonZero".to_string());
                        is_crash = true;
                    }
                } else {
                    let prev = last_written.get(&a).unwrap_or(&def_interval);
                    if (value.clone() - prev.clone()).is_zero(prime) != MayBeFlag::True {
                        bug_types.insert("MemoryInconsistency".to_string());
                        record_reprs.insert("crash".to_string());
                        is_crash = true;
                    }
                }
            }
        }

        if is_write.is_zero(prime) != MayBeFlag::True {
            for a in addr.lo..(addr.hi + 1) {
                last_written.insert(a, value.clone());
                ever_written.insert(a);
            }
        }
    }

    is_crash
}

const ALL_BUG_CLASSES: &[&str] = &["InitialReadNonZero", "MemoryInconsistency"];

/// Per-program comparison record produced by `--benchmark` mode.
#[derive(Debug, Default)]
struct BenchmarkRecord {
    program_id: usize,
    heuristic_found: bool,
    dfs_found: bool,
    heuristic_first_trial: Option<usize>,
    dfs_first_trial: Option<usize>,
    heuristic_total_trials: usize,
    dfs_total_trials: usize,
    heuristic_time_ms: u64,
    dfs_time_ms: u64,
    heuristic_bugs: usize,
    dfs_bugs: usize,
}

fn get_target_program<Val: StarkField>(rng: &mut StdRng) -> Vec<InstructionWord<i32>> {
    let _bytes_per_instr = BYTES_PER_INSTR as i32;
    let a: i32 = rng.gen_range(-0x3C000000..0x3C000000);
    let b: i32 = rng.gen_range(-0x3C000000..0x3C000000);
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

    search_config.max_expansions = 3000;
    search_config.time_out_ms = 10000;
    search_config.seed = 41;

    let air = MemoryChip::default();
    let num_col = NUM_MEM_COLS;
    let chip_idx = 2;

    let machine = BasicMachine::<BabyBear>::default();
    let (mut constraint_info, _general_lookup_info) =
        extract_constraints_and_range::<BasicMachine<BabyBear>, MyConfig, _>(
            &machine,
            &air,
            num_col,
            prime,
            args.method == "bb" && !args.no_simplify,
        );
    if search_config.minimum_num_taregt_cols == 0 {
        search_config.minimum_num_taregt_cols = constraint_info.refinable_cols.len();
    }

    let mut rng = StdRng::seed_from_u64(search_config.seed);
    // Bug classes confirmed across all programs
    let mut global_found_classes: HashSet<String> = HashSet::new();

    // Benchmark mode: accumulates one record per program for the final report.
    let mut benchmark_records: Vec<BenchmarkRecord> = Vec::new();

    if args.benchmark {
        println!(
            "\n{:<4}  {:^8} {:^8}  {:^8} {:^8}  {:^8} {:^8}  {:^7} {:^7}  {:^6} {:^6}",
            "Prog",
            "H.Found",
            "D.Found",
            "H.1stTr",
            "D.1stTr",
            "H.Trials",
            "D.Trials",
            "H.ms",
            "D.ms",
            "H.Bugs",
            "D.Bugs",
        );
        println!("{}", "-".repeat(90));
    }

    for i in 0..args.num_trial {
        println!("\n\n===========");
        let program = get_target_program::<BabyBear>(&mut rng);
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

        let base_abs_main_trace_data = result.unwrap().1;

        search_config.seed = i as u64;
        search_config.min_row_id = 0;
        search_config.max_row_id = base_abs_main_trace_data.len() - 1;

        let program_info = ProgramInfo {
            program_str: program_str,
            program_len: program.len(),
        };

        let honest_repr = format!(
            "{}",
            PrettySet(memory_canonicalizer(
                &AbstractTrace::new(base_abs_main_trace_data.clone()),
                prime
            ))
        );

        if args.benchmark {
            let mut record = BenchmarkRecord {
                program_id: i,
                ..Default::default()
            };

            for &use_heuristic in &[true, false] {
                let mut cfg = search_config.clone();
                cfg.enable_heuristic = use_heuristic;

                let mut ci = constraint_info.clone();

                let mut known: HashSet<String> = HashSet::new();
                known.insert(honest_repr.clone());
                let init_known_size = known.len();

                let first_trial: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));
                let ft = Rc::clone(&first_trial);

                let bug_class_map: Rc<RefCell<HashMap<String, HashSet<String>>>> =
                    Rc::new(RefCell::new(HashMap::new()));
                let bug_class_map_ref = Rc::clone(&bug_class_map);

                let bench_final_check =
                    move |trace: &AbstractTrace,
                          num_trial: usize,
                          prime: u32,
                          known_report: &mut HashSet<String>,
                          ui: &mut UiState,
                          _area: &mut i128| {
                        let mut record_reprs = memory_canonicalizer(trace, prime);
                        let mut bug_types: HashSet<String> = HashSet::new();
                        let is_crash =
                            check_bug_type(trace, &mut record_reprs, &mut bug_types, prime);

                        if !is_crash {
                            return;
                        }

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

                            if ft.borrow().is_none() {
                                *ft.borrow_mut() = Some(num_trial);
                            }
                        }

                        save_repr_if_unique(&PrettySet(record_reprs), known_report, ui);
                    };

                let res = experiment_harness(
                    &program_info,
                    &mut ci,
                    &cfg,
                    &base_abs_main_trace_data,
                    vec![],
                    &if args.blocking_closure && args.range_interval == 0 {
                        vec![0usize]
                    } else {
                        vec![]
                    },
                    nop_post_process,
                    bench_final_check,
                    &args.method,
                    &mut known,
                    true,
                )
                .unwrap();

                let mut bug_str_len = 0;
                for s in ALL_BUG_CLASSES {
                    if known.contains(*s) {
                        bug_str_len += 1;
                    }
                }

                let bugs_found = known.len().saturating_sub(init_known_size + bug_str_len);
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

            println!(
                "{:<4}  {:^8} {:^8}  {:^8} {:^8}  {:^8} {:^8}  {:^7} {:^7}  {:^6} {:^6}",
                record.program_id,
                if record.heuristic_found { "yes" } else { "no" },
                if record.dfs_found { "yes" } else { "no" },
                record
                    .heuristic_first_trial
                    .map_or("-".to_string(), |v| v.to_string()),
                record
                    .dfs_first_trial
                    .map_or("-".to_string(), |v| v.to_string()),
                record.heuristic_total_trials,
                record.dfs_total_trials,
                record.heuristic_time_ms,
                record.dfs_time_ms,
                record.heuristic_bugs,
                record.dfs_bugs,
            );

            benchmark_records.push(record);
        } else {
            // -------- Normal (non-benchmark) branch --------
            let bug_class_map: Rc<RefCell<HashMap<String, HashSet<String>>>> =
                Rc::new(RefCell::new(HashMap::new()));
            let bug_class_map_ref = Rc::clone(&bug_class_map);

            let mut known_solution: HashSet<String> =
                global_found_classes.iter().cloned().collect();
            known_solution.insert(honest_repr.clone());

            let final_check_fn = move |trace: &AbstractTrace,
                                       _num_trial: usize,
                                       prime: u32,
                                       known_report: &mut HashSet<String>,
                                       ui: &mut UiState,
                                       _area: &mut i128| {
                let mut record_reprs = memory_canonicalizer(trace, prime);
                let mut bug_types: HashSet<String> = HashSet::new();
                let is_crash = check_bug_type(trace, &mut record_reprs, &mut bug_types, prime);

                if !is_crash {
                    return;
                }

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
                vec![],
                &if args.blocking_closure && args.range_interval == 0 {
                    vec![0usize]
                } else {
                    vec![]
                },
                nop_post_process,
                final_check_fn,
                &args.method,
                &mut known_solution,
                args.turn_off_ui,
            );
            println!("{:?}", result);

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

        let h_success = benchmark_records
            .iter()
            .filter(|r| r.heuristic_found)
            .count();
        let d_success = benchmark_records.iter().filter(|r| r.dfs_found).count();

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

        let h_mean_trials: f64 = benchmark_records
            .iter()
            .map(|r| r.heuristic_total_trials as f64)
            .sum::<f64>()
            / n;
        let d_mean_trials: f64 = benchmark_records
            .iter()
            .map(|r| r.dfs_total_trials as f64)
            .sum::<f64>()
            / n;

        let h_mean_ms: f64 = benchmark_records
            .iter()
            .map(|r| r.heuristic_time_ms as f64)
            .sum::<f64>()
            / n;
        let d_mean_ms: f64 = benchmark_records
            .iter()
            .map(|r| r.dfs_time_ms as f64)
            .sum::<f64>()
            / n;

        let h_mean_bugs: f64 = benchmark_records
            .iter()
            .map(|r| r.heuristic_bugs as f64)
            .sum::<f64>()
            / n;
        let d_mean_bugs: f64 = benchmark_records
            .iter()
            .map(|r| r.dfs_bugs as f64)
            .sum::<f64>()
            / n;

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

        let h_wins = benchmark_records
            .iter()
            .filter(|r| match (r.heuristic_first_trial, r.dfs_first_trial) {
                (Some(h), Some(d)) => h < d,
                (Some(_), None) => true,
                _ => false,
            })
            .count();
        let d_wins = benchmark_records
            .iter()
            .filter(|r| match (r.heuristic_first_trial, r.dfs_first_trial) {
                (Some(h), Some(d)) => d < h,
                (None, Some(_)) => true,
                _ => false,
            })
            .count();
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
