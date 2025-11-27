use std::collections::HashMap;
use std::collections::HashSet;
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
use valida_machine::{
    Instruction, InstructionWord, Machine, Operands, ProgramROM, SegmentMachine, StarkField,
};
use valida_opcodes::BYTES_PER_INSTR;
use valida_program::MachineWithProgramROM;
use valida_program::ProgramTableType;

use latticevm::interval::AbstractInterval;
use latticevm::solver::run_solver;
use latticevm::symbolic::AbstractTrace;
use latticevm::ui::UiState;

use latticevm::interval::MayBeFlag;
use latticevm_valida::alu_constraints::get_alu_constraints;
use latticevm_valida::alu_tables::{derive_add_table, derive_com_table, derive_sub_table};
use latticevm_valida::config::{get_machine_config, prover_options, MyConfig};
use latticevm_valida::p3_to_tv::get_converted_symbolicconstraints;
use latticevm_valida::state::valida_abstract_trace_to_abstract_state;
use latticevm_valida::utils::{get_adjust_pc_clausuer, program_counter_refine_fn};

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

    let alu_constraints = get_alu_constraints();
    let aux_objs = vec![
        alu_constraints["Add"].clone(),
        alu_constraints["Sub"].clone(),
        alu_constraints["Com"].clone(),
    ];
    let aux_tg_fns = vec![derive_add_table, derive_sub_table, derive_com_table];

    // ############### Parameters of Solver #############################
    let max_iteration = 1000;
    let minimum_num_taregt_cols = 1;
    let min_row_id = 2;
    let max_row_id = 7;

    // ############### Prepare Public Values ############################
    let mut public_vals = vec![AbstractInterval::zero(); 3];
    public_vals[0] = AbstractInterval::from_i64(0);
    public_vals[1] = AbstractInterval::from_i64(4096);
    public_vals[2] = AbstractInterval::from_i64(1);
    let refinment_target_indicies_pv: Vec<usize> = vec![0, 1, 2];

    // ############### Dry-Run Machine ##################################
    let program = add_program::<BabyBear>();
    let program_len = program.len();
    let mut program_str = String::new();
    for inst in &program {
        program_str.push_str(&format!("{}\n", inst));
    }

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
    if let Some(traces) = &mut traces.1[0] {
        let nrows = traces.values.len() / traces.width();
        for i in 0..nrows {
            let row = traces.row_mut(i);
            rows.push(
                row.iter()
                    .map(|v| AbstractInterval::from_i64(v.as_canonical_u32() as i64))
                    .collect(),
            );
        }
    }
    let base_abs_main_trace_data = rows.clone();
    let abs_main_trace = AbstractTrace::new(base_abs_main_trace_data.clone());

    let adjust_pc_program = get_adjust_pc_clausuer(program.clone());

    // ############## Final Check Function ##############################
    fn final_check(
        trace: &AbstractTrace,
        num_trial: usize,
        prime: u32,
        known_reprt: &mut HashSet<String>,
        ui: &mut UiState,
    ) {
        let mut output = String::new();

        let mut memory = HashMap::new();
        let mut recovered_states = vec![];
        for row in &trace.data {
            recovered_states.push(valida_abstract_trace_to_abstract_state(row, &memory, prime));
            memory = recovered_states.last().unwrap().memory.clone();
        }

        let rs_len = recovered_states.len();
        for i in 0..rs_len {
            if i == rs_len - 1 {
                recovered_states[rs_len - 1 - i].memory = HashMap::new();
            } else {
                recovered_states[rs_len - 1 - i].memory =
                    recovered_states[rs_len - 2 - i].memory.clone();
            }
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

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut ui = UiState::new();

    ui.program = program_str;

    run_solver(
        &cpu_constraints,
        &cpu_target_cols,
        &cpu_potential_boolean_vars,
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
