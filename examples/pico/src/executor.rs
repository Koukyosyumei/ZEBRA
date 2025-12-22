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

pub fn run_pico_program(
    program: &Program,
    //chips: &[MetaChip<SC::Val, dyn RecordBehavior>],
) {
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

    let chips = riscv_machine.chips();
    //let _ = riscv_machine.prover;
    //let prover = &riscv_machine.base_machine().prover;

    //let mut traces = chips
    //    .pico_iter()
    //    .map(|chip| chip.generate_main(chunk, EmulationRecord::default()));

    let chips_and_main_traces = riscv_machine
        .base_machine()
        .prover
        .generate_main(&riscv_machine.chips(), chunk);

    /*
    // BaseMachin
    // let chips_and_main_traces = self.prover.generate_main(&self.chips(), record);

    // # Execute the Target Program
    let mut runtime = Executor::new(program.clone(), ZKMCoreOpts::default());
    let (checkpoint, done) = runtime.execute_state(false).unwrap();

    let mut checkpoint_file = tempfile::tempfile()
        .map_err(ZKMCoreProverError::IoError)
        .unwrap();
    checkpoint
        .save(&mut checkpoint_file)
        .map_err(ZKMCoreProverError::IoError)
        .unwrap();

    let true_abstract_states = runtime
        .state_history
        .iter()
        .map(|s| ziren_state_to_abstract_state(s))
        .collect::<Vec<_>>();

    type SC = KoalaBearPoseidon2;
    let config = KoalaBearPoseidon2::new();
    let machine = MipsAir::machine(config);
    let prover = CpuProver::new(machine);

    let mut reader = io::BufReader::new(checkpoint_file);
    let execution_state: ExecutionState =
        bincode::deserialize_from(&mut reader).expect("failed to deserialize state");
    let (records, report) = trace_checkpoint::<SC>(
        program.clone(),
        execution_state,
        ZKMCoreOpts::default(),
        None,
    );
    let mut main_traces = records
        .iter()
        .map(|record| prover.generate_traces(record))
        .collect::<Vec<_>>();

    let mut true_abs_traces = vec![];
    for mt in &mut main_traces[0] {
        //if mt.0 == "Cpu" {
        let nrows = mt.1.values.len() / mt.1.width;
        let mut rows = vec![];
        for i in 0..nrows {
            let row = mt.1.row_mut(i);
            rows.push(
                row.iter()
                    .map(|v| AbstractInterval::from_i64(v.as_canonical_u32() as i64))
                    .collect(),
            );
        }
        true_abs_traces.push((mt.0.clone(), rows));
        //}
    }

    (true_abstract_states, true_abs_traces)
    */
}
