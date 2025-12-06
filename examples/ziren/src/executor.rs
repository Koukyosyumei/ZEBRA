use std::borrow::Borrow;
use std::io;

use p3_air::BaseAir;
use p3_field::PrimeField32;

use zkm_core_executor::{ExecutionState, Executor, Program};
use zkm_core_machine::columns::CpuCols;
use zkm_core_machine::memory::MemoryInitCols;
use zkm_core_machine::memory::MemoryLocalCols;
use zkm_core_machine::mips::MipsAir;
use zkm_core_machine::utils::trace_checkpoint;
use zkm_core_machine::utils::ZKMCoreProverError;
use zkm_stark::koala_bear_poseidon2::KoalaBearPoseidon2;
use zkm_stark::ZKMCoreOpts;
use zkm_stark::{CpuProver, MachineProver};

use latticevm::interval::AbstractInterval;
use latticevm::state::AbstractState;

use crate::state::ziren_state_to_abstract_state;

pub fn run_ziren_program(
    program: &Program,
) -> (
    Vec<AbstractState>,
    Vec<(String, Vec<Vec<AbstractInterval>>)>,
) {
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
            let mut row = mt.1.row_mut(i);
            rows.push(
                row.iter()
                    .map(|v| AbstractInterval::from_i64(v.as_canonical_u32() as i64))
                    .collect(),
            );
        }
        true_abs_traces.push((mt.0.clone(), rows));
        //}

        /*
        println!("{}, {} - {}", mt.0, mt.1.values.len(), mt.1.width);
        if mt.0 == "MemoryLocal" {
            for i in 0..4 {
                let mut row = mt.1.row_mut(i);
                let local: &MemoryLocalCols<_> = (*row).borrow();
                println!("  row[{}]: {:?}", i, local);
            }
        } else if mt.0 == "MemoryGlobalInit" {
            for i in 0..4 {
                let mut row = mt.1.row_mut(i);
                let local: &MemoryInitCols<_> = (*row).borrow();
                println!("  row[{}]: {:?}", i, local);
            }
        } else if mt.0 == "Cpu" {
            for i in 0..4 {
                let mut row = mt.1.row_mut(i);
                let local: &CpuCols<_> = (*row).borrow();
                println!("  row[{}]: {:?}", i, local);
            }
        }
        */
    }

    (true_abstract_states, true_abs_traces)
}
