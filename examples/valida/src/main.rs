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

use valida_alu_u32::add::{Add32Instruction, MachineWithAdd32Chip};
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
use valida_machine::StarkConfigImpl;
use valida_machine::{
    Instruction, InstructionWord, Machine, MachineProof, MachineRuntime, MemoryBackendTrait,
    MultiSegmentMachineProof, Operands, ProgramROM, ProverOptions, SegmentMachine, StarkField,
    ValidaMemoryBackend, Word,
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

use latticevm_valida::p3_to_tv::convert_p3_expr;
use latticevm_valida::state::valida_abstract_trace_to_abstract_state;

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

fn add_program<Val: StarkField>() -> Vec<InstructionWord<i32>> {
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
        /*
        InstructionWord {
            opcode: <BneInstruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands([0 * bytes_per_instr, -8, -4, 0, 0]),
        },
        */
        InstructionWord {
            opcode: <StopInstruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands::default(),
        },
    ]);

    program
}

fn main() -> Result<(), io::Error> {
    println!("{:?}", CPU_COL_MAP);
    // 2^31 - 2^27 + 1
    let prime = 2_u32.pow(31) - 2_u32.pow(27) + 1;
    let program_cols = (3..8).collect::<Vec<_>>();

    let air = CpuChip::default();
    let machine = BasicMachine::<BabyBear>::default();
    let symbolic_constraints =
        get_symbolic_constraints::<BasicMachine<BabyBear>, MyConfig, CpuChip>(&machine, &air);

    //let lookup_constraints =
    //    get_symbolic_lookups::<BasicMachine<BabyBear>, MyConfig, CpuChip>(&machine, &air);

    let mut tv_constraints = symbolic_constraints
        .iter()
        .map(|sc| convert_p3_expr::<BabyBear>(&sc))
        .collect::<Vec<_>>();
    for tv in &tv_constraints {
        println!("{}", tv);
    }
    println!("=====================\n\n");

    /*
    let mut lv_constraints = lookup_constraints
        .iter()
        .map(|sc| convert_p3_expr::<BabyBear>(&sc))
        .collect::<Vec<_>>();
    for tv in &lv_constraints {
        println!("{}", tv);
    }
    println!("=====================\n\n");
    */

    //println!("{:?}", air.ephemeral_interactions(&machine));

    let mut rng = StdRng::seed_from_u64(42);
    let max_row_id = 2;
    let num_extracted_rows = 2;
    let potential_boolean_vars = gather_boolean_variables(&tv_constraints);

    let constraints = LatticeVMConstraints {
        air_constraints: tv_constraints.clone(),
        pv_pos_constraints: vec![],
        pv_neg_constraints: vec![],
    };

    // ############### Prepare Public Values ############################
    let mut public_vals = vec![AbstractInterval::zero(); 3];
    public_vals[0] = AbstractInterval::from_i64(0);
    public_vals[1] = AbstractInterval::from_i64(4096);
    public_vals[2] = AbstractInterval::from_i64(1);
    let refinment_target_indicies_pv: Vec<usize> = vec![0, 1, 2];

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

    for s in &state.machine.state_history {
        println!("{:?}", s);
    }

    let config = get_machine_config();
    let (prover_opts, show_preprocessed, show_preprocessed_dims, show_public_verifier) =
        prover_options();

    let mut traces = state.machine.generate_traces(&config, prover_opts);

    let mut rows = vec![];
    if let Some(traces) = &mut traces.1[0] {
        let nrows = traces.values.len() / traces.width();
        for i in 0..nrows {
            println!("{}: {:?}", i, traces.row_mut(i));
            let mut row = traces.row_mut(i);
            rows.push(
                row.iter()
                    .map(|v| AbstractInterval::from_i64(v.as_canonical_u32() as i64))
                    .collect(),
            );
        }
    }
    let base_abs_main_trace_data = rows.clone();

    // ############## Final Check Function ##############################
    fn final_check(trace: &AbstractTrace, prime: u32, ui: &mut UiState) {
        let mut output = String::new();

        let recovered_states = trace
            .data
            .iter()
            .map(|row| valida_abstract_trace_to_abstract_state(row, prime))
            .collect::<Vec<_>>();

        output.push_str("Malicious States:\n");
        for rs in &recovered_states {
            output.push_str(&format!("\t{}\n", rs));
        }
        output.push_str("-----------------\n");

        ui.recovered = output;
    }

    let mut target_cols = (0..NUM_CPU_COLS).collect::<Vec<_>>();
    target_cols.retain(|x| !program_cols.contains(x));

    let abs_main_trace = AbstractTrace::new(base_abs_main_trace_data.clone());

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
        &constraints,
        &target_cols,
        &potential_boolean_vars,
        &refinment_target_indicies_pv,
        &base_abs_main_trace_data,
        public_vals,
        max_row_id,
        final_check,
        prime,
        42,
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

    //let cpu = &state.machine.cpu;

    //let dot = exprs_to_dependency_dot(&tv_constraints);
    //println!("{}", dot);
    //let symbolic_constraints: Vec<SymbolicExpression<BabyBear>> =
    //    get_symbolic_constraints(&air, 0, ZKM_PROOF_NUM_PV_ELTS);

    Ok(())
}
