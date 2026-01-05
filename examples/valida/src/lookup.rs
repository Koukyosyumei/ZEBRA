use p3_air::Air;
use p3_air::VirtualPairCol;
use p3_field::Field;
use p3_field::PrimeField32;

use valida_machine::symbolic::symbolic_builder::SymbolicAirBuilder;
use valida_machine::BusArgument;
use valida_machine::{
    ChipWithPersistence, InteractionType, Machine, StarkConfig, ValidaAirBuilder,
};
use valida_opcodes::Opcode;

use latticevm::alu::get_alu_constraint;
use latticevm::alu::WordOp;
use latticevm::symbolic::make_impl_constraint;
use latticevm::symbolic::LatticeVMSymbolicExpr;

use crate::p3_to_tv::convert_p3_virtual_pair_col;

pub fn add_single_var_col_if_possible<F: PrimeField32>(
    b: &VirtualPairCol<F>,
    u8_cols: &mut Vec<usize>,
) {
    if !b.column_weights.is_empty() {
        if let p3_air::PairCol::Main(col_idx) = b.column_weights[0].0 {
            u8_cols.push(col_idx);
        }
    }
}

pub fn inspect_lookup_interactions<M, C, SC, AB>(
    chip: &C,
    builder: &mut AB,
    range_u8_cols: &mut Vec<usize>,
    pc_cols: &mut Vec<Vec<usize>>,
    counter_cols: &mut Vec<usize>,
    lookup_constraints: &mut Vec<LatticeVMSymbolicExpr>,
    prime: u32,
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
                    // Lookup with General ALU
                    if id == 0 {
                        let opcode = convert_p3_virtual_pair_col(&e_interaction.fields[0]);

                        let b0 = convert_p3_virtual_pair_col(&e_interaction.fields[1]);
                        let b1 = convert_p3_virtual_pair_col(&e_interaction.fields[2]);
                        let b2 = convert_p3_virtual_pair_col(&e_interaction.fields[3]);
                        let b3 = convert_p3_virtual_pair_col(&e_interaction.fields[4]);

                        let c0 = convert_p3_virtual_pair_col(&e_interaction.fields[5]);
                        let c1 = convert_p3_virtual_pair_col(&e_interaction.fields[6]);
                        let c2 = convert_p3_virtual_pair_col(&e_interaction.fields[7]);
                        let c3 = convert_p3_virtual_pair_col(&e_interaction.fields[8]);

                        let a0 = convert_p3_virtual_pair_col(&e_interaction.fields[9]);
                        let a1 = convert_p3_virtual_pair_col(&e_interaction.fields[10]);
                        let a2 = convert_p3_virtual_pair_col(&e_interaction.fields[11]);
                        let a3 = convert_p3_virtual_pair_col(&e_interaction.fields[12]);

                        let multiplicities = convert_p3_virtual_pair_col(&e_interaction.count);

                        let tmps = vec![
                            (Opcode::ADD32 as u8, WordOp::Add),
                            (Opcode::SUB32 as u8, WordOp::Sub),
                            (Opcode::MUL32 as u8, WordOp::Mul),
                            (Opcode::LT32 as u8, WordOp::Lt),
                            (Opcode::SLT32 as u8, WordOp::SLt),
                            (Opcode::MULHU32 as u8, WordOp::MulHU),
                            (Opcode::MULHS32 as u8, WordOp::MulHS),
                            (Opcode::EQ32 as u8, WordOp::Eq),
                            (Opcode::NE32 as u8, WordOp::NEq),
                            (Opcode::DIV32 as u8, WordOp::Div),
                            (Opcode::SDIV32 as u8, WordOp::SDiv),
                        ];

                        for t in tmps {
                            let alu_constraint = get_alu_constraint(
                                &[a0.clone(), a1.clone(), a2.clone(), a3.clone()],
                                &[b0.clone(), b1.clone(), b2.clone(), b3.clone()],
                                &[c0.clone(), c1.clone(), c2.clone(), c3.clone()],
                                &t.1,
                            );
                            let impl_constraint =
                                make_impl_constraint(t.0 as i64, &opcode, alu_constraint, prime);

                            if let Some(impl_constraint) = impl_constraint {
                                for i in 1..13 {
                                    add_single_var_col_if_possible(
                                        &e_interaction.fields[i],
                                        range_u8_cols,
                                    );
                                }

                                lookup_constraints.push(LatticeVMSymbolicExpr::Mul(
                                    Box::new(multiplicities.clone()),
                                    Box::new(impl_constraint),
                                ));
                            }
                        }
                    }

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

                    // Lookup with other byte instructions
                    if id == 3 {
                        let opcode_condition =
                            convert_p3_virtual_pair_col(&e_interaction.fields[0]);
                        let input = convert_p3_virtual_pair_col(&e_interaction.fields[1]);
                        let output = convert_p3_virtual_pair_col(&e_interaction.fields[2]);
                        let multiplicities = convert_p3_virtual_pair_col(&e_interaction.count);

                        let ops = [(1, LatticeVMSymbolicExpr::Msb(Box::new(input.clone())))];
                        for (opcode, op_expr) in ops {
                            let el_constraint = make_impl_constraint(
                                opcode,
                                &opcode_condition,
                                LatticeVMSymbolicExpr::Sub(
                                    Box::new(output.clone()),
                                    Box::new(op_expr),
                                ),
                                prime,
                            );
                            if let Some(el_constraint) = el_constraint {
                                lookup_constraints.push(LatticeVMSymbolicExpr::Mul(
                                    Box::new(multiplicities.clone()),
                                    Box::new(el_constraint),
                                ));
                            }
                        }
                    }

                    for (col, _weight) in &e_interaction.count.column_weights {
                        if let p3_air::PairCol::Main(col_idx) = col {
                            counter_cols.push(*col_idx);
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

pub fn get_lookup_interactions<M, SC, C>(
    machine: &M,
    chip: &C,
    range_u8_cols: &mut Vec<usize>,
    pc_cols: &mut Vec<Vec<usize>>,
    counter_cols: &mut Vec<usize>,
    lookup_constraints: &mut Vec<LatticeVMSymbolicExpr>,
    prime: u32,
) where
    M: Machine<SC::Val>,
    SC: StarkConfig,
    C: ChipWithPersistence<M, SC>,
{
    let mut builder = SymbolicAirBuilder::new(
        machine,
        chip.main_width(),
        chip.preprocessed_width(),
        chip.public_width(),
        chip.permutation_width(machine),
    );

    inspect_lookup_interactions(
        chip,
        &mut builder,
        range_u8_cols,
        pc_cols,
        counter_cols,
        lookup_constraints,
        prime,
    );
}
