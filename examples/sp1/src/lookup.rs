use std::collections::HashSet;

use crate::p3_to_tv::convert_p3_virtual_pair_col;
use p3_air::Air;
use p3_air::PairCol;
use p3_air::VirtualPairCol;
use p3_field::PrimeField32;

use sp1_core_executor::Opcode;
use sp1_stark::InteractionBuilder;
use sp1_stark::InteractionKind;

use latticevm::alu::get_alu_constraint;
use latticevm::alu::WordOp;
use latticevm::interval::AbstractInterval;
use latticevm::symbolic::make_impl_constraint;
use latticevm::symbolic::LatticeVMSymbolicExpr;
use latticevm::utils::GeneralLookupInfo;

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

pub fn get_symbolic_lookup_constraints<F, A>(
    air: &A,
    preprocessed_width: usize,
    num_public_values: usize,
    u8_cols: &mut Vec<usize>,
    multiplicities: &mut HashSet<usize>,
    lookup_constraints: &mut Vec<LatticeVMSymbolicExpr>,
    received_vars_from_cpu: &mut HashSet<usize>,
    prime: u32,
) -> GeneralLookupInfo
where
    F: p3_field::PrimeField32,
    A: Air<InteractionBuilder<F>>,
{
    let mut general_lookup_info = GeneralLookupInfo::default();
    let mut builder = InteractionBuilder::new(preprocessed_width, air.width());
    air.eval(&mut builder);
    let (sends, receives) = builder.interactions();

    for r in &receives {
        for (w, _) in &r.multiplicity.column_weights {
            if let p3_air::PairCol::Main(col_idx) = w {
                multiplicities.insert(*col_idx);
            }
        }
    }

    for r in &receives {
        match r.kind {
            InteractionKind::Instruction => {
                for rv in &r.values {
                    for c in &rv.column_weights {
                        if let PairCol::Main(index) = c.0 {
                            received_vars_from_cpu.insert(index);
                        }
                    }
                }
                let shard = &r.values[0];
                let clk = &r.values[1];
                let pc = &r.values[2];
                let next_pc = &r.values[3];
                let num_extra_cycles = &r.values[4];
                let opcode = &r.values[5];
                let a0 = &r.values[6];
                let a1 = &r.values[7];
                let a2 = &r.values[8];
                let a3 = &r.values[9];
                let b0 = &r.values[10];
                let b1 = &r.values[11];
                let b2 = &r.values[12];
                let b3 = &r.values[13];
                let c0 = &r.values[14];
                let c1 = &r.values[15];
                let c2 = &r.values[16];
                let c3 = &r.values[17];

                for i in 7..11 {
                    add_single_var_col_if_possible(
                        &r.values[i],
                        &mut general_lookup_info.alu_output,
                    );
                }
                for i in 11..15 {
                    add_single_var_col_if_possible(
                        &r.values[i],
                        &mut general_lookup_info.alu_input1,
                    );
                }
                for i in 15..19 {
                    add_single_var_col_if_possible(
                        &r.values[i],
                        &mut general_lookup_info.alu_input2,
                    );
                }

                let op_a_0 = &r.values[18];
                let op_a_immutable = &r.values[19];
                let is_memory = &r.values[20];
                let is_syscall = &r.values[21];
                let is_halt = &r.values[22];
            }
            _ => {}
        }
    }

    for s in &sends {
        let multiplicities = convert_p3_virtual_pair_col(&s.multiplicity);

        match s.kind {
            InteractionKind::Program => {
                general_lookup_info.pc_table_is_real = multiplicities.clone();
            }
            InteractionKind::Instruction => {
                let _shard = &s.values[0];
                let _clk = &s.values[1];
                let pc = convert_p3_virtual_pair_col(&s.values[2]);
                let next_pc = convert_p3_virtual_pair_col(&s.values[3]);

                let opcode = convert_p3_virtual_pair_col(&s.values[5]);
                let a0 = convert_p3_virtual_pair_col(&s.values[6]);
                let a1 = convert_p3_virtual_pair_col(&s.values[7]);
                let a2 = convert_p3_virtual_pair_col(&s.values[8]);
                let a3 = convert_p3_virtual_pair_col(&s.values[9]);
                let b0 = convert_p3_virtual_pair_col(&s.values[10]);
                let b1 = convert_p3_virtual_pair_col(&s.values[11]);
                let b2 = convert_p3_virtual_pair_col(&s.values[12]);
                let b3 = convert_p3_virtual_pair_col(&s.values[13]);
                let c0 = convert_p3_virtual_pair_col(&s.values[14]);
                let c1 = convert_p3_virtual_pair_col(&s.values[15]);
                let c2 = convert_p3_virtual_pair_col(&s.values[16]);
                let c3 = convert_p3_virtual_pair_col(&s.values[17]);

                let tmps = vec![
                    (Opcode::ADD as u8, WordOp::Add),
                    (Opcode::SUB as u8, WordOp::SubU),
                    (Opcode::MUL as u8, WordOp::Mul),
                    (Opcode::SLT as u8, WordOp::SLt),
                    (Opcode::AND as u8, WordOp::And),
                    (Opcode::OR as u8, WordOp::Or),
                    (Opcode::XOR as u8, WordOp::Xor),
                    (Opcode::SRL as u8, WordOp::SRL),
                ];
                for t in tmps {
                    let alu_constraint = get_alu_constraint(
                        &[a0.clone(), a1.clone(), a2.clone(), a3.clone()],
                        &[b0.clone(), b1.clone(), b2.clone(), b3.clone()],
                        &[c0.clone(), c1.clone(), c2.clone(), c3.clone()],
                        &[a0.clone(), a1.clone(), a2.clone(), a3.clone()],
                        &t.1,
                    );
                    let impl_constraint =
                        make_impl_constraint(t.0 as i64, &opcode, alu_constraint, prime);
                    if let Some(impl_constraint) = impl_constraint {
                        for i in 7..19 {
                            add_single_var_col_if_possible(&s.values[i], u8_cols);
                        }

                        lookup_constraints.push(LatticeVMSymbolicExpr::Mul(
                            Box::new(multiplicities.clone()),
                            Box::new(impl_constraint),
                        ));
                    }

                    let pc_constraint = LatticeVMSymbolicExpr::Sub(
                        Box::new(next_pc.clone()),
                        Box::new(LatticeVMSymbolicExpr::Add(
                            Box::new(pc.clone()),
                            Box::new(LatticeVMSymbolicExpr::Constant(AbstractInterval::from_i64(
                                4,
                            ))),
                        )),
                    );
                    let impl_pc_constraint =
                        make_impl_constraint(t.0 as i64, &opcode, pc_constraint, prime);
                    if let Some(impl_pc_constraint) = impl_pc_constraint {
                        lookup_constraints.push(LatticeVMSymbolicExpr::Mul(
                            Box::new(multiplicities.clone()),
                            Box::new(impl_pc_constraint),
                        ));
                    }
                }

                /*
                let shard = &r.values[0];
                let clk = &r.values[1];
                let pc = &r.values[2];
                let next_pc = &r.values[3];
                let next_next_pc = &r.values[4];
                let num_extra_cycles = &r.values[5];

                let hi0 = &r.values[19];
                let hi1 = &r.values[20];
                let hi2 = &r.values[21];
                let hi3 = &r.values[22];
                let op_a_immutable = &r.values[24];
                let is_rw_a = &r.values[25];
                let is_check_memory = &r.values[26];
                let is_halt = &r.values[27];
                let is_sequential = &r.values[28];
                */
            }
            InteractionKind::Byte => {
                let opcode = &s.values[0];
                let a1 = &s.values[1];
                let a2 = &s.values[2];
                let b = &s.values[3];
                let c = &s.values[4];

                // Range U8
                add_single_var_col_if_possible(&b, u8_cols);
                add_single_var_col_if_possible(&c, u8_cols);
                add_single_var_col_if_possible(&a1, u8_cols);

                let a1_expr = convert_p3_virtual_pair_col(&a1);
                let b_expr = convert_p3_virtual_pair_col(&b);
                let c_expr = convert_p3_virtual_pair_col(&c);
                let opcode_condition = convert_p3_virtual_pair_col(&opcode);

                let ops = [
                    (
                        0,
                        LatticeVMSymbolicExpr::And(
                            Box::new(b_expr.clone()),
                            Box::new(c_expr.clone()),
                        ),
                    ),
                    (
                        1,
                        LatticeVMSymbolicExpr::Or(
                            Box::new(b_expr.clone()),
                            Box::new(c_expr.clone()),
                        ),
                    ),
                    (
                        2,
                        LatticeVMSymbolicExpr::Xor(
                            Box::new(b_expr.clone()),
                            Box::new(c_expr.clone()),
                        ),
                    ),
                    (
                        6,
                        LatticeVMSymbolicExpr::Flip(Box::new(LatticeVMSymbolicExpr::Lt(
                            Box::new(b_expr.clone()),
                            Box::new(c_expr.clone()),
                        ))),
                    ),
                    (7, LatticeVMSymbolicExpr::Msb(Box::new(b_expr.clone()))),
                ];

                for (opcode, op_expr) in ops {
                    let el_constraint = make_impl_constraint(
                        opcode,
                        &opcode_condition,
                        LatticeVMSymbolicExpr::Sub(Box::new(a1_expr.clone()), Box::new(op_expr)),
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
            _ => {}
        }
    }

    general_lookup_info
}
