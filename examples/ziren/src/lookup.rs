use std::collections::HashSet;

use crate::p3_to_tv::convert_p3_virtual_pair_col;
use p3_air::Air;
use p3_air::AirBuilder;
use p3_air::PairCol;
use p3_air::VirtualPairCol;
use p3_field::PrimeField32;
use p3_koala_bear::KoalaBear;
use p3_uni_stark::{SymbolicExpression, SymbolicVariable};

use zkm_stark::LookupBuilder;
use zkm_stark::LookupKind;

use latticevm::alu::get_alu_constraint;
use latticevm::alu::OpALU;
use latticevm::interval::AbstractInterval;
use latticevm::symbolic::LatticeVMSymbolicExpr;

use crate::p3_to_tv::convert_p3_expr;

fn make_impl_constraint(
    opcode_val: i64,
    opcode_var: &LatticeVMSymbolicExpr,
    expr: LatticeVMSymbolicExpr,
    prime: u32,
) -> Option<LatticeVMSymbolicExpr> {
    if let LatticeVMSymbolicExpr::Constant(c) = opcode_var {
        if c.as_canonical_u32(prime) as i64 == opcode_val {
            Some(expr)
        } else {
            None
        }
    } else {
        Some(LatticeVMSymbolicExpr::Impl(
            Box::new(LatticeVMSymbolicExpr::Sub(
                Box::new(opcode_var.clone()),
                Box::new(LatticeVMSymbolicExpr::Constant(AbstractInterval::from_i64(
                    opcode_val,
                ))),
            )),
            Box::new(expr),
        ))
    }
}

pub fn add_u8_col_if_possible<F: PrimeField32>(b: &VirtualPairCol<F>, u8_cols: &mut Vec<usize>) {
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
) where
    F: p3_field::PrimeField32,
    A: Air<LookupBuilder<F>>,
{
    let mut builder = LookupBuilder::new(preprocessed_width, air.width());
    air.eval(&mut builder);
    let (sends, receives) = builder.lookups();

    for r in &receives {
        for (w, _) in &r.multiplicity.column_weights {
            if let p3_air::PairCol::Main(col_idx) = w {
                multiplicities.insert(*col_idx);
            }
        }
    }
    /*
    for s in &sends {
        for (w, _) in &s.multiplicity.column_weights {
            if let p3_air::PairCol::Main(col_idx) = w {
                multiplicities.insert(*col_idx);
            }
        }
    }*/

    for r in &receives {
        match r.kind {
            LookupKind::Instruction => {
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
                let next_next_pc = &r.values[4];
                let num_extra_cycles = &r.values[5];
                let opcode = &r.values[6];
                let a0 = &r.values[7];
                let a1 = &r.values[8];
                let a2 = &r.values[9];
                let a3 = &r.values[10];
                let b0 = &r.values[11];
                let b1 = &r.values[12];
                let b2 = &r.values[13];
                let b3 = &r.values[14];
                let c0 = &r.values[15];
                let c1 = &r.values[16];
                let c2 = &r.values[17];
                let c3 = &r.values[18];

                let hi0 = &r.values[19];
                let hi1 = &r.values[20];
                let hi2 = &r.values[21];
                let hi3 = &r.values[22];
                let op_a_immutable = &r.values[24];
                let is_rw_a = &r.values[25];
                let is_check_memory = &r.values[26];
                let is_halt = &r.values[27];
                let is_sequential = &r.values[28];
            }
            _ => {}
        }
    }

    for s in &sends {
        match s.kind {
            LookupKind::Instruction => {
                /*
                for rv in &s.values {
                    for c in &rv.column_weights {
                        if let PairCol::Main(index) = c.0 {
                            received_vars_from_cpu.insert(index);
                        }
                    }
                }*/
                let opcode = convert_p3_virtual_pair_col(&s.values[6]);
                let a0 = convert_p3_virtual_pair_col(&s.values[7]);
                let a1 = convert_p3_virtual_pair_col(&s.values[8]);
                let a2 = convert_p3_virtual_pair_col(&s.values[9]);
                let a3 = convert_p3_virtual_pair_col(&s.values[10]);
                let b0 = convert_p3_virtual_pair_col(&s.values[11]);
                let b1 = convert_p3_virtual_pair_col(&s.values[12]);
                let b2 = convert_p3_virtual_pair_col(&s.values[13]);
                let b3 = convert_p3_virtual_pair_col(&s.values[14]);
                let c0 = convert_p3_virtual_pair_col(&s.values[15]);
                let c1 = convert_p3_virtual_pair_col(&s.values[16]);
                let c2 = convert_p3_virtual_pair_col(&s.values[17]);
                let c3 = convert_p3_virtual_pair_col(&s.values[18]);

                let tmps = vec![
                    (0, OpALU::Add),
                    (1, OpALU::Sub),
                    (2, OpALU::Mul),
                    (15, OpALU::And),
                    (16, OpALU::Or),
                    (17, OpALU::Xor),
                ];
                for t in tmps {
                    let alu_constraint = get_alu_constraint(
                        &[a0.clone(), a1.clone(), a2.clone(), a3.clone()],
                        &[b0.clone(), b1.clone(), b2.clone(), b3.clone()],
                        &[c0.clone(), c1.clone(), c2.clone(), c3.clone()],
                        &t.1,
                    );
                    let impl_constraint = make_impl_constraint(t.0, &opcode, alu_constraint, prime);
                    if let Some(impl_constraint) = impl_constraint {
                        for i in 7..19 {
                            add_u8_col_if_possible(&s.values[i], u8_cols);
                        }

                        lookup_constraints.push(impl_constraint);
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
            LookupKind::Byte => {
                let opcode = &s.values[0];
                let a1 = &s.values[1];
                let a2 = &s.values[2];
                let b = &s.values[3];
                let c = &s.values[4];

                // Range U8
                add_u8_col_if_possible(&b, u8_cols);
                add_u8_col_if_possible(&c, u8_cols);
                add_u8_col_if_possible(&a1, u8_cols);

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
                        LatticeVMSymbolicExpr::Lt(
                            Box::new(b_expr.clone()),
                            Box::new(c_expr.clone()),
                        ),
                    ),
                ];

                for (opcode, op_expr) in ops {
                    let el_constraint = make_impl_constraint(
                        opcode,
                        &opcode_condition,
                        LatticeVMSymbolicExpr::Sub(Box::new(a1_expr.clone()), Box::new(op_expr)),
                        prime,
                    );
                    if let Some(el_constraint) = el_constraint {
                        lookup_constraints.push(el_constraint);
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn inspect_lookup(builder: LookupBuilder<KoalaBear>) {
    /*
    let mut builder = LookupBuilder::<KoalaBear>::new(0, NUM_CPU_COLS);
    air.eval(&mut builder);
     */
    ///////////////////////////////////////////////////////
    let mut main = builder.main();
    let (sends, receives) = builder.lookups();

    for lookup in receives {
        print!("Receive values: ");
        for value in lookup.values {
            let expr = value.apply::<SymbolicExpression<KoalaBear>, SymbolicVariable<KoalaBear>>(
                &[],
                main.row_mut(0),
            );
            print!("{}, ", convert_p3_expr::<KoalaBear>(&expr));
        }

        let multiplicity = lookup
            .multiplicity
            .apply::<SymbolicExpression<KoalaBear>, SymbolicVariable<KoalaBear>>(
                &[],
                main.row_mut(0),
            );

        println!(
            "   multiplicity: {}",
            convert_p3_expr::<KoalaBear>(&multiplicity)
        );

        println!("  scope: {:?}, kind: {:?}", lookup.scope, lookup.kind);
    }

    for lookup in sends {
        print!("Send values: ");
        for value in lookup.values {
            let expr = value.apply::<SymbolicExpression<KoalaBear>, SymbolicVariable<KoalaBear>>(
                &[],
                main.row_mut(0),
            );
            print!("{}, ", convert_p3_expr::<KoalaBear>(&expr));
        }

        let multiplicity = lookup
            .multiplicity
            .apply::<SymbolicExpression<KoalaBear>, SymbolicVariable<KoalaBear>>(
                &[],
                main.row_mut(0),
            );

        println!(
            "   multiplicity: {}",
            convert_p3_expr::<KoalaBear>(&multiplicity)
        );

        println!("  scope: {:?}, kind: {:?}", lookup.scope, lookup.kind);
    }
}
