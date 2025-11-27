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

pub fn get_adjust_pc_clausuer(
    program: Vec<InstructionWord<i32>>,
) -> impl Fn(&mut AbstractTrace, u32) {
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

pub fn program_counter_refine_fn(
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
