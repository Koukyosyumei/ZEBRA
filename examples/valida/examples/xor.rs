use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::rc::Rc;
use std::{io, thread, time::Duration};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::rngs::StdRng;
use rand::thread_rng;
use rand::SeedableRng;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

use p3_baby_bear::BabyBear;
use p3_challenger::DuplexChallenger;
use p3_commit::ExtensionMmcs;
use p3_dft::Radix2Bowers;
use p3_field::extension::BinomialExtensionField;
use p3_field::{AbstractField, Field, PrimeField32, TwoAdicField};
use p3_fri::FriConfig;
use p3_fri::{TwoAdicFriPcs, TwoAdicFriPcsConfig};
use p3_keccak::Keccak256Hash;
use p3_matrix::dense::RowMajorMatrix;
use p3_matrix::Matrix;
use p3_mds::coset_mds::CosetMds;
use p3_merkle_tree::FieldMerkleTreeMmcs;
use p3_poseidon::Poseidon;
use p3_symmetric::{CompressionFunctionFromHasher, SerializingHasher32};
//use p3_uni_stark::symbolic_builder::get_symbolic_constraints;

use valida_alu_u32::add::columns::ADD_COL_MAP;
use valida_alu_u32::add::Add32Chip;
use valida_alu_u32::add::{columns::NUM_ADD_COLS, Add32Instruction, MachineWithAdd32Chip};
use valida_alu_u32::bitwise::columns::COL_MAP;
use valida_alu_u32::bitwise::And32Instruction;
use valida_alu_u32::bitwise::Bitwise32Chip;
use valida_alu_u32::com::columns::COM_COL_MAP;
use valida_alu_u32::com::Com32Chip;
use valida_alu_u32::com::Eq32Instruction;
use valida_alu_u32::com::Ne32Instruction;
use valida_alu_u32::mul::Mul32Chip;
use valida_alu_u32::sub::columns::SUB_COL_MAP;
use valida_alu_u32::sub::Sub32Chip;
use valida_alu_u32::sub::Sub32Instruction;
use valida_basic_api::BasicMachine;
use valida_basic_api::BasicMachineMetrics;
use valida_basic_api::ValidaRuntime;
use valida_cpu::BneInstruction;
use valida_cpu::Imm32Instruction;
use valida_cpu::MachineWithRegisters;
use valida_cpu::StopInstruction;
use valida_cpu::{
    columns::{CPU_COL_MAP, NUM_CPU_COLS},
    CpuChip,
};
use valida_machine::symbolic::symbolic_builder::{
    get_symbolic_constraints, get_symbolic_lookups, SymbolicAirBuilder,
};
use valida_machine::symbolic::symbolic_expression::SymbolicExpression;
use valida_machine::Chip;
use valida_machine::ChipWithPersistence;
use valida_machine::StarkConfig;
use valida_machine::StarkConfigImpl;
use valida_machine::{
    check_constraints::display_interaction, Instruction, InstructionWord, Machine, MachineProof,
    MachineRuntime, MemoryBackendTrait, MultiSegmentMachineProof, Operands, ProgramROM,
    ProverOptions, SegmentMachine, StarkField, ValidaMemoryBackend, Word,
};
use valida_opcodes::BYTES_PER_INSTR;
use valida_program::MachineWithProgramROM;
use valida_program::ProgramTableType;

use latticevm::interval::AbstractInterval;
use latticevm::solver::run_solver;
use latticevm::symbolic::eval_air_constraints;
use latticevm::symbolic::eval_constraints;
use latticevm::symbolic::gather_boolean_variables;
use latticevm::symbolic::AbstractTrace;
use latticevm::symbolic::LatticeVMConstraints;
use latticevm::ui::UiState;

use latticevm::interval::MayBeFlag;
use latticevm::solver::AbsConstraintObj;
use latticevm_valida::p3_to_tv::convert_p3_expr;
use latticevm_valida::p3_to_tv::get_converted_symbolicconstraints;
//use latticevm_valida::state::check_eq_states;
use latticevm_valida::state::valida_abstract_trace_to_abstract_state;
//use latticevm_valida::state::valida_state_to_abstract_state;

pub type Val = BabyBear;
pub type Challenge = BinomialExtensionField<Val, 5>;
pub type PackedChallenge = BinomialExtensionField<<Val as Field>::Packing, 5>;
pub type Mds16 = CosetMds<Val, 16>;
pub type Perm16 = Poseidon<Val, Mds16, 16, 5>;
pub type MyHash = SerializingHasher32<Keccak256Hash>;
pub type MyCompress = CompressionFunctionFromHasher<Val, MyHash, 2, 8>;
pub type ValMmcs = FieldMerkleTreeMmcs<Val, MyHash, MyCompress, 8>;
pub type ChallengeMmcs = ExtensionMmcs<Val, Challenge, ValMmcs>;
pub type Dft = Radix2Bowers;
pub type Challenger = DuplexChallenger<Val, Perm16, 16>;
pub type MyFriConfig = TwoAdicFriPcsConfig<Val, Challenge, Challenger, Dft, ValMmcs, ChallengeMmcs>;
pub type Pcs = TwoAdicFriPcs<MyFriConfig>;
pub type MyConfig = StarkConfigImpl<Val, Challenge, PackedChallenge, Pcs, Challenger>;

pub fn get_machine_config() -> MyConfig {
    let mds16 = Mds16::default();
    let perm16 = Perm16::new_from_rng(4, 22, mds16, &mut thread_rng()); // TODO: Use deterministic RNG
    let hash = MyHash::new(Keccak256Hash {});
    let compress = MyCompress::new(hash);
    let val_mmcs = ValMmcs::new(hash, compress);
    let challenge_mmcs = ChallengeMmcs::new(val_mmcs.clone());
    let dft = Dft::default();
    let fri_config = FriConfig {
        log_blowup: 1,
        num_queries: 40,
        proof_of_work_bits: 8,
        mmcs: challenge_mmcs,
    };

    let pcs = Pcs::new(fri_config, dft, val_mmcs);

    let challenger = Challenger::new(perm16);
    let config = MyConfig::new(pcs, challenger);
    config
}

/// Returns the prover options used in all the tests as well as the a vector for
/// `show_preprocessed`, bool for `show_preprocessed_dims` and vector for
/// `show_public_verifier`.
pub fn prover_options() -> (ProverOptions, Vec<bool>, bool, Vec<bool>) {
    // Have each trace print once: only shown if the test fails anyway.
    // skip preprocessed, which includes some long traces
    let show_preprocessed = vec![false; BasicMachine::<Val>::NUM_CHIPS];
    let show_public_prover = vec![false; BasicMachine::<Val>::NUM_CHIPS];
    let show_main = vec![false; BasicMachine::<Val>::NUM_CHIPS];
    let show_interactions = vec![false; BasicMachine::<Val>::NUM_CHIPS];
    let show_public_verifier = vec![false; BasicMachine::<Val>::NUM_CHIPS];
    let show_public_dims = false;
    let show_main_dims = false;
    let show_permutation_dims = false;
    let show_preprocessed_dims = false;

    let prover_opts = ProverOptions {
        show_main,
        show_public: show_public_prover,
        show_interactions,
        show_public_dims,
        show_main_dims,
        show_permutation_dims,
    };

    (
        prover_opts,
        show_preprocessed,
        show_preprocessed_dims,
        show_public_verifier,
    )
}

fn derive_add_table(
    cpu_main_trace: &Vec<Vec<AbstractInterval>>,
    potential_boolean_vars: &Vec<usize>,
    prime: u32,
) -> Vec<Vec<AbstractInterval>> {
    let mut out = vec![];
    for row in cpu_main_trace {
        if row[58].as_canonical_u32(prime) != 0 && row[3].as_canonical_u32(prime) == 100 {
            let mut r: Vec<_> = (0..16).map(|_| AbstractInterval::top(prime)).collect();
            for i in potential_boolean_vars {
                r[*i] = AbstractInterval::bool();
            }

            let cpu_columns = vec![32, 33, 34, 35, 38, 39, 40, 41, 44, 45, 46, 47];
            let add_columns = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
            for i in 0..(cpu_columns.len()) {
                r[add_columns[i]] = row[cpu_columns[i]].clone();
            }
            r[15] = AbstractInterval::one();
            out.push(r);
        }
    }

    out
}

fn derive_sub_table(
    cpu_main_trace: &Vec<Vec<AbstractInterval>>,
    potential_boolean_vars: &Vec<usize>,
    prime: u32,
) -> Vec<Vec<AbstractInterval>> {
    let mut out = vec![];
    for row in cpu_main_trace {
        if row[58].as_canonical_u32(prime) != 0 && row[3].as_canonical_u32(prime) == 101 {
            let mut r: Vec<_> = (0..17).map(|_| AbstractInterval::top(prime)).collect();
            for i in potential_boolean_vars {
                r[*i] = AbstractInterval::bool();
            }

            let cpu_columns = vec![32, 33, 34, 35, 38, 39, 40, 41, 44, 45, 46, 47];
            let sub_columns = vec![0, 1, 2, 3, 4, 5, 6, 7, 12, 13, 14, 15];
            for i in 0..(cpu_columns.len()) {
                r[sub_columns[i]] = row[cpu_columns[i]].clone();
            }
            r[16] = AbstractInterval::one();
            out.push(r);
        }
    }

    out
}

fn derive_bitwise_table(
    cpu_main_trace: &Vec<Vec<AbstractInterval>>,
    potential_boolean_vars: &Vec<usize>,
    prime: u32,
) -> Vec<Vec<AbstractInterval>> {
    let mut out = vec![];
    for row in cpu_main_trace {
        if row[58].as_canonical_u32(prime) != 0
            && (row[3].as_canonical_u32(prime) == 107
                || row[3].as_canonical_u32(prime) == 108
                || row[3].as_canonical_u32(prime) == 109)
        {
            let mut r: Vec<_> = (0..79).map(|_| AbstractInterval::top(prime)).collect();
            for i in potential_boolean_vars {
                r[*i] = AbstractInterval::bool();
            }

            let cpu_columns = vec![32, 33, 34, 35, 38, 39, 40, 41, 44, 45, 46, 47];
            let bws_columns = vec![64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75];
            for i in 0..(cpu_columns.len()) {
                r[bws_columns[i]] = row[cpu_columns[i]].clone();
            }
            if row[3].as_canonical_u32(prime) == 107 {
                r[76] = AbstractInterval::one();
                r[77] = AbstractInterval::zero();
                r[78] = AbstractInterval::zero();
            } else if row[3].as_canonical_u32(prime) == 108 {
                r[76] = AbstractInterval::zero();
                r[77] = AbstractInterval::one();
                r[78] = AbstractInterval::zero();
            } else if row[3].as_canonical_u32(prime) == 109 {
                r[76] = AbstractInterval::zero();
                r[77] = AbstractInterval::zero();
                r[78] = AbstractInterval::one();
            }
            out.push(r);
        }
    }

    out
}

fn derive_com_table(
    cpu_main_trace: &Vec<Vec<AbstractInterval>>,
    potential_boolean_vars: &Vec<usize>,
    prime: u32,
) -> Vec<Vec<AbstractInterval>> {
    let mut out = vec![];
    for row in cpu_main_trace {
        if row[58].as_canonical_u32(prime) != 0
            && (row[3].as_canonical_u32(prime) == 116 || row[3].as_canonical_u32(prime) == 111)
        {
            let mut r: Vec<_> = (0..14).map(|_| AbstractInterval::i4()).collect();
            for i in potential_boolean_vars {
                r[*i] = AbstractInterval::bool();
            }

            let cpu_columns = vec![32, 33, 34, 35, 38, 39, 40, 41, 44];
            let com_columns = vec![0, 1, 2, 3, 4, 5, 6, 7, 11];
            for i in 0..(cpu_columns.len()) {
                r[com_columns[i]] = row[cpu_columns[i]].clone();
            }
            if row[3].as_canonical_u32(prime) == 116 {
                r[13] = AbstractInterval::one();
                r[12] = AbstractInterval::zero();
            } else {
                r[13] = AbstractInterval::zero();
                r[12] = AbstractInterval::one();
            }
            out.push(r);
        }
    }

    out
}

fn add_program<Val: StarkField>() -> Vec<InstructionWord<i32>> {
    let bytes_per_instr = BYTES_PER_INSTR as i32;

    let mut program = vec![];
    program.extend([
        InstructionWord {
            opcode: <Imm32Instruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands([-4, 2, 0, 0, 0]),
        },
        InstructionWord {
            opcode: <And32Instruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands([-8, -8, 1, 0, 1]),
        },
        //InstructionWord {
        //    opcode: <BneInstruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
        //    operands: Operands([1 * bytes_per_instr, -8, -4, 0, 0]),
        //},
        InstructionWord {
            opcode: <StopInstruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands::default(),
        },
    ]);

    program
}

fn main() -> Result<(), io::Error> {
    // 2^31 - 2^27 + 1
    let prime = 2_u32.pow(31) - 2_u32.pow(27) + 1;
    let program_cols = (3..8).collect::<Vec<_>>();

    // ############### Config #########################################
    let config = get_machine_config();
    let (prover_opts, show_preprocessed, show_preprocessed_dims, show_public_verifier) =
        prover_options();

    // ############### Extract Constraints ############################
    let machine = BasicMachine::<BabyBear>::default();

    let cpu_air = CpuChip::default();
    let (cpu_constraints, mut cpu_potential_boolean_vars) =
        get_converted_symbolicconstraints::<BasicMachine<BabyBear>, MyConfig, _>(
            &machine, &cpu_air,
        );
    cpu_potential_boolean_vars.push(18);
    cpu_potential_boolean_vars.push(19);
    cpu_potential_boolean_vars.push(22);
    cpu_potential_boolean_vars.push(24);
    let mut cpu_target_cols = (0..NUM_CPU_COLS).collect::<Vec<_>>();
    cpu_target_cols.retain(|x| !program_cols.contains(x));
    println!("CPU AIR MAP");
    println!("  {:?}", CPU_COL_MAP);

    let add_air = Add32Chip::default();
    let (add_constraints, add_potential_boolean_vars) =
        get_converted_symbolicconstraints::<BasicMachine<BabyBear>, MyConfig, _>(
            &machine, &add_air,
        );
    let add_target_cols = vec![8, 9, 10, 11, 12, 13, 14];
    let aux_add_obj = AbsConstraintObj {
        name: "Add".to_string(),
        aux_constraints: add_constraints.clone(),
        aux_target_cols: add_target_cols.clone(),
        aux_potential_boolean_vars: add_potential_boolean_vars.clone(),
    };
    println!("ADD AIR MAP");
    println!("  {:?}", ADD_COL_MAP);

    let sub_air = Sub32Chip::default();
    let (sub_constraints, sub_potential_boolean_vars) =
        get_converted_symbolicconstraints::<BasicMachine<BabyBear>, MyConfig, _>(
            &machine, &sub_air,
        );
    let sub_target_cols = vec![8, 9, 10, 11, 12, 13, 14, 15];
    let aux_sub_obj = AbsConstraintObj {
        name: "Sub".to_string(),
        aux_constraints: sub_constraints,
        aux_target_cols: sub_target_cols,
        aux_potential_boolean_vars: sub_potential_boolean_vars,
    };
    println!("SUB AIR MAP");
    println!("  {:?}", SUB_COL_MAP);

    let bitwise_air = Bitwise32Chip::default();
    let (bitwise_constraints, bitwise_potential_boolean_vars) = get_converted_symbolicconstraints::<
        BasicMachine<BabyBear>,
        MyConfig,
        _,
    >(&machine, &bitwise_air);
    let bitwise_target_cols = (0..64).collect::<Vec<usize>>();
    println!("BITWISE AIR MAP");
    println!("  {:?}", COL_MAP);

    let com_air = Com32Chip::default();
    let (com_constraints, com_potential_boolean_vars) =
        get_converted_symbolicconstraints::<BasicMachine<BabyBear>, MyConfig, _>(
            &machine, &com_air,
        );
    let com_target_cols = vec![8, 9, 10];
    let aux_com_obj = AbsConstraintObj {
        name: "Com".to_string(),
        aux_constraints: com_constraints,
        aux_target_cols: com_target_cols,
        aux_potential_boolean_vars: com_potential_boolean_vars,
    };
    println!("COM AIR MAP");
    println!("  {:?}", COM_COL_MAP);

    let aux_objs = vec![];
    let aux_tg_fns = vec![derive_add_table, derive_sub_table, derive_com_table];

    let min_row_id = 0;
    let max_row_id = 0;

    // ############### Prepare Public Values ############################
    let mut public_vals = vec![AbstractInterval::zero(); 3];
    public_vals[0] = AbstractInterval::from_i64(0);
    public_vals[1] = AbstractInterval::from_i64(4096);
    public_vals[2] = AbstractInterval::from_i64(1);
    let refinment_target_indicies_pv: Vec<usize> = vec![0, 1, 2];

    // ############### Dry-Run Machine ##################################
    let program = add_program::<BabyBear>();
    let rom = ProgramROM::new(program.clone());

    let mut machine = BasicMachine::<BabyBear>::default();
    machine.set_segment_number(0);
    machine.set_max_trace_height(65536);
    machine.set_program_rom(rom, ProgramTableType::Public);
    machine.set_initial_register_values(valida_cpu::Registers { pc: 0, fp: 0x1000 });

    let mut runtime = ValidaRuntime::default_for_field::<BabyBear>();
    let mut state = machine.start(&mut runtime);
    let mut metrics = BasicMachineMetrics::initialize();
    let (instance_data, _output) = BasicMachine::run(&mut state, &mut metrics);

    let mut traces = state.machine.generate_traces(&config, prover_opts);

    // ############# Obtain the inital solution ############################
    let mut rows = vec![];
    if let Some(traces) = &mut traces.1[10] {
        let nrows = traces.values.len() / traces.width();
        for i in 0..nrows {
            let mut row = traces.row_mut(i);
            rows.push(
                row.iter()
                    .map(|v| AbstractInterval::from_i64(v.as_canonical_u32() as i64))
                    .collect(),
            );
        }
    }
    let base_abs_main_trace_data = rows.clone();
    let abs_main_trace = AbstractTrace::new(base_abs_main_trace_data.clone());

    fn program_counter_refine_fn(
        abs_main_trace_data: &mut Vec<Vec<AbstractInterval>>,
        i: usize,
        j: usize,
    ) {
    }

    fn adjust_pc_program(main_trace: &mut AbstractTrace, prime: u32) {}

    // ############## Final Check Function ##############################
    fn final_check(
        trace: &AbstractTrace,
        num_trial: usize,
        prime: u32,
        known_reprt: &mut HashSet<String>,
        ui: &mut UiState,
    ) {
        let mut output = String::new();
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut ui = UiState::new();

    let mut program_str = String::new();
    for inst in program {
        program_str.push_str(&format!("{}\n", inst));
    }
    ui.program = program_str;

    run_solver(
        &bitwise_constraints,
        &bitwise_target_cols,
        &bitwise_potential_boolean_vars,
        &aux_objs,
        &aux_tg_fns,
        &refinment_target_indicies_pv,
        &base_abs_main_trace_data,
        public_vals,
        min_row_id,
        max_row_id,
        program_counter_refine_fn,
        adjust_pc_program,
        final_check,
        prime,
        41,
        &mut ui,
        &mut terminal,
    );

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
