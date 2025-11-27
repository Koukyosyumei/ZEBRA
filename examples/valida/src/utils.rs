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

use crate::config::{get_machine_config, prover_options};

pub fn make_pc_adjuster(program: Vec<InstructionWord<i32>>) -> impl Fn(&mut AbstractTrace, u32) {
    move |main_trace: &mut AbstractTrace, prime: u32| {
        for row in &mut main_trace.data {
            if row[1].is_singleton() {
                let pc = row[1].as_canonical_u32(prime) as usize;
                if pc < program.len() {
                    let instr = program[pc];
                    row[3] = AbstractInterval::from_i64(instr.opcode.into());
                    row[4] = AbstractInterval::from_i64(instr.operands.0[0].into());
                    row[5] = AbstractInterval::from_i64(instr.operands.0[1].into());
                    row[6] = AbstractInterval::from_i64(instr.operands.0[2].into());
                    row[7] = AbstractInterval::from_i64(instr.operands.0[3].into());
                    row[8] = AbstractInterval::from_i64(instr.operands.0[4].into());

                    if row[3].as_canonical_u32(prime) == 8 {
                        for i in 4..57 {
                            row[i] = AbstractInterval::zero();
                        }
                        row[24] = AbstractInterval::from_i64(1);
                    }
                }
            } else {
                row[3] = AbstractInterval::i4();
                row[4] = AbstractInterval::i4();
                row[5] = AbstractInterval::i4();
                row[6] = AbstractInterval::i4();
                row[7] = AbstractInterval::i4();
                row[8] = AbstractInterval::i4();
            }
        }
    }
}

pub fn refine_pc_interval(
    abs_main_trace_data: &mut Vec<Vec<AbstractInterval>>,
    program_len: usize,
    i: usize,
    j: usize,
) {
    if j == 1 {
        abs_main_trace_data[i][j] = AbstractInterval {
            lo: 0,
            hi: (program_len - 1) as i64,
        };
    }
}

pub fn generate_bootstrap_trace_from_program(
    program: &Vec<InstructionWord<i32>>,
    i: usize,
    pc: u32,
    fp: u32,
) -> Vec<Vec<AbstractInterval>> {
    let config = get_machine_config();
    let (prover_opts, show_preprocessed, show_preprocessed_dims, show_public_verifier) =
        prover_options();

    let rom = ProgramROM::new(program.clone());
    let mut machine = BasicMachine::<BabyBear>::default();
    machine.set_segment_number(0);
    machine.set_max_trace_height(65536);
    machine.set_program_rom(rom, ProgramTableType::Public);
    machine.set_initial_register_values(valida_cpu::Registers { pc: pc, fp: fp });

    let mut runtime = ValidaRuntime::default_for_field::<BabyBear>();
    let mut state = machine.start(&mut runtime);
    let mut metrics = BasicMachineMetrics::initialize();
    let (instance_data, _output) = BasicMachine::run(&mut state, &mut metrics);

    let mut traces = state.machine.generate_traces(&config, prover_opts);

    // ############# Obtain the inital solution ############################
    let mut rows = vec![];
    if let Some(traces) = &mut traces.1[i] {
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
    rows
}
