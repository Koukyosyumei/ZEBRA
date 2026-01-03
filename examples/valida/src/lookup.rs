use std::collections::HashMap;
use std::collections::HashSet;

use p3_air::Air;
use p3_baby_bear::BabyBear;
use p3_field::Field;
use p3_field::{AbstractField, PrimeField32};
use p3_matrix::Matrix;

use valida_basic_api::BasicMachine;
use valida_basic_api::BasicMachineMetrics;
use valida_basic_api::ValidaRuntime;
use valida_cpu::MachineWithRegisters;
use valida_machine::symbolic::symbolic_builder::get_lookup_interactions;
use valida_machine::symbolic::symbolic_builder::get_symbolic_constraints;
use valida_machine::BusArgument;
use valida_machine::{InstructionWord, ProgramROM, SegmentMachine};
use valida_program::MachineWithProgramROM;
use valida_program::ProgramTableType;

use valida_machine::{
    columns::{PermutationColsView, MAX_PERMUTATION_CONSTRAINT_DEGREE},
    permutation::MAX_PERMUTATION_HEIGHT,
    ChipWithPersistence, Interaction, InteractionType, Machine, StarkConfig, ValidaAirBuilder,
};

pub fn inspect_lookup_interactions<M, C, SC, AB>(
    chip: &C,
    builder: &mut AB,
    range_u8_cols: &mut Vec<usize>,
    pc_cols: &mut Vec<Vec<usize>>,
    counter_cols: &mut Vec<usize>,
) where
    M: Machine<SC::Val>,
    C: ChipWithPersistence<M, SC> + Air<AB>,
    SC: StarkConfig,
    AB: ValidaAirBuilder<Machine = M, F = SC::Val, EF = SC::Challenge>,
{
    let machine = builder.machine();
    let ephemeral_interactions = chip.ephemeral_interactions(machine);

    for (e_interaction, interaction_type) in &ephemeral_interactions {
        println!("e_interaction: {:?}", e_interaction);

        match interaction_type {
            InteractionType::LocalSend => {}
            InteractionType::LocalReceive => {}
            InteractionType::GlobalSend => match e_interaction.argument_index {
                BusArgument::Local(_) => {}
                BusArgument::Global(id) => {
                    // Lookup with Range8
                    if id == 5 {
                        for pair in &e_interaction.fields {
                            if pair.constant.is_zero() {
                                for (col, _weight) in &pair.column_weights {
                                    match col {
                                        p3_air::PairCol::Preprocessed(_) => {}
                                        p3_air::PairCol::Public(_) => {}
                                        p3_air::PairCol::Main(col_idx) => {
                                            range_u8_cols.push(col_idx.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                BusArgument::Persistent(_) => {}
            },
            InteractionType::GlobalReceive => match e_interaction.argument_index {
                BusArgument::Local(_) => {}
                BusArgument::Global(id) => {
                    // Lookup with CPU
                    if id == 0 {
                        for pair in e_interaction.fields.iter() {
                            let mut sub_cols = Vec::new();
                            for (col, _weight) in &pair.column_weights {
                                if let p3_air::PairCol::Main(col_idx) = col {
                                    sub_cols.push(*col_idx);
                                }
                            }
                            pc_cols.push(sub_cols);
                        }
                        for (col, _weight) in &e_interaction.count.column_weights {
                            if let p3_air::PairCol::Main(col_idx) = col {
                                counter_cols.push(*col_idx);
                            }
                        }
                    }
                }
                BusArgument::Persistent(_) => {}
            },
            InteractionType::PersistentSend => {}
            InteractionType::PersistentReceive => {}
        }
    }
}
