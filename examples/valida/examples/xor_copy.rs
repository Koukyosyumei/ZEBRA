use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::rc::Rc;
use std::time;
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
use valida_alu_u32::bitwise::columns::NUM_BITWISE_COLS;
use valida_alu_u32::bitwise::And32Instruction;
use valida_alu_u32::bitwise::Bitwise32Chip;
use valida_alu_u32::com::columns::COM_COL_MAP;
use valida_alu_u32::com::Com32Chip;
use valida_alu_u32::com::Eq32Instruction;
use valida_alu_u32::com::Ne32Instruction;
use valida_alu_u32::mul::Mul32Chip;
use valida_alu_u32::sub::columns::NUM_SUB_COLS;
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
    get_lookup_interactions, get_symbolic_constraints, get_symbolic_lookups, SymbolicAirBuilder,
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
use latticevm::interval::MayBeFlag;
use latticevm::solver::run_solver;
use latticevm::solver::RangeType;
use latticevm::symbolic::eval_air_constraints;
use latticevm::symbolic::eval_constraints;
use latticevm::symbolic::gather_boolean_variables;
use latticevm::symbolic::AbstractTrace;
use latticevm::symbolic::LatticeVMConstraints;
use latticevm::ui::UiState;
use latticevm::utils::create_or_clear_dir;

use latticevm_valida::alu_constraints::get_alu_constraint;
use latticevm_valida::alu_tables::{derive_add_table, derive_com_table, derive_sub_table};
use latticevm_valida::config::{get_machine_config, prover_options, MyConfig};
use latticevm_valida::p3_to_tv::get_converted_symbolicconstraints;
use latticevm_valida::state::valida_abstract_trace_to_abstract_state;
use latticevm_valida::utils::{
    generate_bootstrap_trace_from_program, make_pc_adjuster, refine_pc_interval,
};

fn program_counter_refine_fn(
    abs_main_trace_data: &mut Vec<Vec<AbstractInterval>>,
    program_len: usize,
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
    let string_representation = format!(
        "input0: [{}, {}, {}, {}], input1: [{}, {}, {}, {}], output: [{}, {}, {}, {}]",
        trace.data[0][64],
        trace.data[0][65],
        trace.data[0][66],
        trace.data[0][67],
        trace.data[0][68],
        trace.data[0][69],
        trace.data[0][70],
        trace.data[0][71],
        trace.data[0][72],
        trace.data[0][73],
        trace.data[0][74],
        trace.data[0][75],
    );

    if !known_reprt.contains(&string_representation) {
        known_reprt.insert(string_representation.clone());
        ui.recovered = string_representation;

        fs::write(
            format!("voutput/{}_states.txt", known_reprt.len()),
            ui.recovered.clone(),
        )
        .unwrap();
        fs::write(
            format!("voutput/{}_assignments.txt", known_reprt.len()),
            ui.logs.clone(),
        )
        .unwrap();
    }
}

fn get_target_program<Val: StarkField>(a: i32, b: i32) -> Vec<InstructionWord<i32>> {
    let bytes_per_instr = BYTES_PER_INSTR as i32;

    let mut program = vec![];
    program.extend([
        InstructionWord {
            opcode: <Imm32Instruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands([-4, a, 0, 0, 0]),
        },
        InstructionWord {
            opcode: <And32Instruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
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

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(27) + 1;

    // ######################## Extract Add Constraints ##########################
    println!("Bitwise AIR MAP");
    println!("  {:?}", COL_MAP);

    let bitwise_air = Bitwise32Chip::default();
    let machine = BasicMachine::<BabyBear>::default();

    let mut alu_constraint = get_alu_constraint::<BasicMachine<BabyBear>, MyConfig, _>(
        &machine,
        &bitwise_air,
        NUM_BITWISE_COLS,
    );
    alu_constraint.aux_range_types.insert(72, RangeType::Top);
    alu_constraint.aux_range_types.insert(73, RangeType::Top);
    alu_constraint.aux_range_types.insert(74, RangeType::Top);
    alu_constraint.aux_range_types.insert(75, RangeType::Top);

    alu_constraint.aux_refinement_plan.push(64);
    alu_constraint.aux_refinement_plan.push(68);
    alu_constraint.aux_range_types.insert(64, RangeType::U4);
    alu_constraint.aux_range_types.insert(68, RangeType::U4);

    let chip_idx = 10;
    let minimum_num_taregt_cols = alu_constraint.aux_refinement_plan.len();

    println!("  Target Columns: {:?}", alu_constraint.aux_refinement_plan);
    println!("  Range Types   : {:?}", alu_constraint.aux_range_types);

    let aux_objs = vec![];
    let aux_tg_fns = vec![derive_add_table, derive_sub_table, derive_com_table];

    // ######################## Solver Parameters ###############################
    let max_iteration = 1000000000;
    let min_row_id = 0;
    let max_row_id = 0;
    let seed = 41;

    // Public trace values (example: program start, memory base, initial step)
    let mut public_vals = vec![AbstractInterval::zero(); 3];
    public_vals[0] = AbstractInterval::from_i64(0);
    public_vals[1] = AbstractInterval::from_i64(4096);
    public_vals[2] = AbstractInterval::from_i64(1);
    let refinment_target_indicies_pv: Vec<usize> = vec![0, 1, 2];

    // ######################## Program Initialization ###########################
    let program = get_target_program::<BabyBear>(9, 11);
    let program_len = program.len();

    // Convert program to string for UI display
    let program_str = program
        .iter()
        .map(|inst| format!("{}\n", inst))
        .collect::<String>();

    // Generate initial abstract main trace from program
    let base_abs_main_trace_data =
        generate_bootstrap_trace_from_program(&program, chip_idx, 0, 0x1000);

    // ######################## UI Initialization ################################
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut ui = UiState::new();
    ui.program = program_str;

    // ######################## Run Solver ######################################
    let mut known_solution = HashSet::<String>::new();
    let start_time = time::Instant::now();
    run_solver(
        &alu_constraint.aux_constraints,
        &alu_constraint.aux_refinement_plan,
        &alu_constraint.aux_range_types,
        &aux_objs,
        &aux_tg_fns,
        &refinment_target_indicies_pv,
        &base_abs_main_trace_data,
        public_vals,
        max_iteration,
        minimum_num_taregt_cols,
        min_row_id,
        max_row_id,
        program_len,
        program_counter_refine_fn,
        adjust_pc_program,
        final_check,
        prime,
        seed,
        &mut known_solution,
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

    eprintln!("Execution Time    : {:?}", start_time.elapsed());
    eprintln!("#Unique Solution  : {}", known_solution.len());

    Ok(())
}
