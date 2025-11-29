use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use p3_baby_bear::BabyBear;
use p3_field::{AbstractField, PrimeField32};
use p3_matrix::Matrix;

use valida_alu_u32::add::Add32Instruction;
use valida_basic_api::{BasicMachine, BasicMachineMetrics, ValidaRuntime};
use valida_cpu::{
    columns::{CPU_COL_MAP, NUM_CPU_COLS},
    BneInstruction, CpuChip, Imm32Instruction, MachineWithRegisters, StopInstruction,
};
use valida_machine::{
    Instruction, InstructionWord, Machine, Operands, ProgramROM, SegmentMachine, StarkField,
};
use valida_opcodes::BYTES_PER_INSTR;
use valida_program::{MachineWithProgramROM, ProgramTableType};

use latticevm::interval::AbstractInterval;
use latticevm::interval::MayBeFlag;
use latticevm::solver::run_solver;
use latticevm::solver::RangeType;
use latticevm::symbolic::AbstractTrace;
use latticevm::ui::UiState;
use latticevm::utils::create_or_clear_dir;

use latticevm_valida::alu_constraints::get_alu_constraints;
use latticevm_valida::alu_tables::{derive_add_table, derive_com_table, derive_sub_table};
use latticevm_valida::config::{get_machine_config, prover_options, MyConfig};
use latticevm_valida::p3_to_tv::get_converted_symbolicconstraints;
use latticevm_valida::state::valida_abstract_trace_to_abstract_state;
use latticevm_valida::utils::{
    generate_bootstrap_trace_from_program, make_pc_adjuster, refine_pc_interval,
};

// ############## Final Check Function ##############################
fn final_check(
    trace: &AbstractTrace,
    num_trial: usize,
    prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    let mut output = String::new();

    let mut recovered_states = vec![];
    for row in &trace.data {
        recovered_states.push(valida_abstract_trace_to_abstract_state(row, prime));
    }

    let mut string_representation = String::new();
    for state in &recovered_states {
        string_representation.push_str(&format!("{}\n", state));
        if state.is_done != MayBeFlag::False {
            break;
        }
    }

    if !known_reprt.contains(&string_representation) {
        known_reprt.insert(string_representation.clone());

        output.push_str(&format!("Trial ID: {}\n\n", num_trial));
        output.push_str("Malicious States:\n");
        output.push_str(&string_representation);
        output.push_str("-----------------\n\n");

        ui.recovered = output;

        fs::write("states.txt", ui.recovered.clone()).unwrap();
        fs::write("assignments.txt", ui.logs.clone()).unwrap();
    }
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

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(27) + 1;
    // Columns reserved for program counters / instructions
    let program_cols = (3..8).collect::<Vec<_>>();

    // ######################## Extract CPU Constraints ##########################
    let machine = BasicMachine::<BabyBear>::default();

    let cpu_air = CpuChip::default();
    let (cpu_constraints, mut cpu_potential_boolean_vars) =
        get_converted_symbolicconstraints::<BasicMachine<BabyBear>, MyConfig, _>(
            &machine, &cpu_air,
        );

    // Additional boolean columns
    cpu_potential_boolean_vars.push(18);
    cpu_potential_boolean_vars.push(19);
    cpu_potential_boolean_vars.push(22);
    cpu_potential_boolean_vars.push(24);
    let cpu_range_types = cpu_potential_boolean_vars
        .iter()
        .map(|k| (*k, RangeType::Bool))
        .collect();

    // Columns available for refinement (excluding reserved program columns)
    let mut cpu_target_cols = (0..NUM_CPU_COLS).collect::<Vec<_>>();
    cpu_target_cols.retain(|x| !program_cols.contains(x));
    println!("CPU AIR Column Mapping:\n {:?}", CPU_COL_MAP);

    // ######################## Auxiliary ALU Constraints #######################
    let alu_constraints = get_alu_constraints();
    let aux_objs = vec![
        alu_constraints["Add"].clone(),
        alu_constraints["Sub"].clone(),
        alu_constraints["Com"].clone(),
    ];
    let aux_tg_fns = vec![derive_add_table, derive_sub_table, derive_com_table];

    // ######################## Solver Parameters ###############################
    let max_iteration = 1000;
    let minimum_num_taregt_cols = 1;
    let min_row_id = 2;
    let max_row_id = 7;
    let seed = 41;

    // Public trace values (example: program start, memory base, initial step)
    let mut public_vals = vec![AbstractInterval::zero(); 3];
    public_vals[0] = AbstractInterval::from_i64(0);
    public_vals[1] = AbstractInterval::from_i64(4096);
    public_vals[2] = AbstractInterval::from_i64(1);
    let refinment_target_indicies_pv: Vec<usize> = vec![0, 1, 2];

    // ######################## Program Initialization ###########################
    let program = get_target_program::<BabyBear>();
    let program_len = program.len();

    // Convert program to string for UI display
    let program_str = program
        .iter()
        .map(|inst| format!("{}\n", inst))
        .collect::<String>();

    // Generate initial abstract main trace from program
    let base_abs_main_trace_data = generate_bootstrap_trace_from_program(&program, 0, 0, 0x1000);

    // Closure to adjust PC intervals to match program semantics
    let adjust_pc_program = make_pc_adjuster(program.clone());

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
    run_solver(
        &cpu_constraints,
        &cpu_target_cols,
        &cpu_range_types,
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
        refine_pc_interval,
        adjust_pc_program,
        final_check,
        prime,
        seed,
        &mut known_solution,
        &mut ui,
        &mut terminal,
    );

    // ######################## Cleanup #########################################
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
