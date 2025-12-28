use std::io;
use std::sync::Arc;

use p3_air::BaseAir;
use p3_field::PrimeField32;
use p3_koala_bear::KoalaBear;

use pico_vm::compiler::riscv::program::Program;
use pico_vm::configs::config::StarkGenericConfig;
use pico_vm::emulator::opts::EmulatorOpts;
use pico_vm::emulator::record::RecordBehavior;
use pico_vm::emulator::riscv::emulator::RiscvEmulator;
use pico_vm::emulator::riscv::record::EmulationRecord;
use pico_vm::instances::chiptype::riscv_chiptype::RiscvChipType;
use pico_vm::instances::configs::embed_kb_bn254_poseidon2::KoalaBearBn254Poseidon2;
use pico_vm::instances::machine::riscv::RiscvMachine;
use pico_vm::iter::IntoPicoRefIterator;
use pico_vm::iter::PicoIterator;
use pico_vm::machine::chip::{ChipBehavior, MetaChip};
use pico_vm::machine::machine::MachineBehavior;
use pico_vm::primitives::consts::RISCV_NUM_PVS;
use pico_vm::primitives::Poseidon2Init;

use latticevm::interval::AbstractInterval;
use latticevm::state::AbstractState;

pub fn run_pico_program(program: &Program) -> Vec<(String, Vec<Vec<AbstractInterval>>)> {
    let config = KoalaBearBn254Poseidon2::new();
    let riscv_machine = RiscvMachine::new(config, RiscvChipType::all_chips(), RISCV_NUM_PVS);

    let mut runtime = RiscvEmulator::new_single::<KoalaBear>(
        Arc::new(program.clone()),
        EmulatorOpts::default(),
        None,
    );
    // runtime.state.input_stream.push(vec![2, 0, 0, 0]);
    let batch_records = runtime.run(None).unwrap().0;
    let chunk = &batch_records[0];

    let mut chips_and_main_traces = riscv_machine
        .base_machine()
        .prover
        .generate_main(&riscv_machine.chips(), chunk);

    let mut true_abs_traces = vec![];
    for mt in &mut chips_and_main_traces {
        let nrows = mt.1.values.len() / mt.1.width;
        let mut rows: Vec<_> = vec![];
        for i in 0..nrows {
            let row = mt.1.row_mut(i);
            rows.push(
                row.iter()
                    .map(|v| AbstractInterval::from_i64(v.as_canonical_u32() as i64))
                    .collect::<Vec<_>>(),
            );
        }
        true_abs_traces.push((mt.0.clone(), rows));
    }

    true_abs_traces
}
