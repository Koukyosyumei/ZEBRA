use std::collections::HashSet;

use crate::p3_to_tv::convert_p3_virtual_pair_col;
use p3_air::Air;
use p3_air::AirBuilder;
use p3_air::PairCol;
use p3_air::VirtualPairCol;
use p3_field::PrimeField32;
use p3_koala_bear::KoalaBear;
use p3_uni_stark::{SymbolicExpression, SymbolicVariable};

use pico_vm::compiler::riscv::opcode::Opcode;
use pico_vm::machine::builder::{ChipBuilder, ChipLookupBuilder, LookupBuilder};
use pico_vm::machine::folder::SymbolicConstraintFolder;
use pico_vm::machine::lookup::LookupType;

use latticevm::alu::get_alu_constraint;
use latticevm::alu::OpALU;
use latticevm::interval::AbstractInterval;
use latticevm::symbolic::make_impl_constraint;
use latticevm::symbolic::LatticeVMSymbolicExpr;

use crate::p3_to_tv::convert_p3_expr;

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
    A: Air<SymbolicConstraintFolder<F>>,
{
    let mut builder = SymbolicConstraintFolder::new(preprocessed_width, air.width());
    air.eval(&mut builder);

    let (sends, receives) = builder.lookups();

    for r in &receives {
        for (w, _) in &r.mult.column_weights {
            if let p3_air::PairCol::Main(col_idx) = w {
                multiplicities.insert(*col_idx);
            }
        }
    }

    for r in &receives {
        match r.kind {
            LookupType::Alu => {
                for rv in &r.values {
                    for c in &rv.column_weights {
                        if let PairCol::Main(index) = c.0 {
                            received_vars_from_cpu.insert(index);
                        }
                    }
                }

                let opcode = &r.values[0];
                let a0 = &r.values[1];
                let a1 = &r.values[2];
                let a2 = &r.values[3];
                let a3 = &r.values[4];
                let b0 = &r.values[5];
                let b1 = &r.values[6];
                let b2 = &r.values[7];
                let b3 = &r.values[8];
                let c0 = &r.values[9];
                let c1 = &r.values[10];
                let c2 = &r.values[11];
                let c3 = &r.values[12];

                println!("receive opcode: {:?}", opcode);
                println!("receive a: {:?} {:?} {:?} {:?}", a0, a1, a2, a3);
                println!("receive b: {:?} {:?} {:?} {:?}", b0, b1, b2, b3);
                println!("receive c: {:?} {:?} {:?} {:?}", c0, c1, c2, c3);
            }
            _ => {}
        }
    }

    for s in &sends {
        match s.kind {
            LookupType::Byte => {
                let opcode = &s.values[0];
                let a1 = &s.values[1];
                let a2 = &s.values[2];
                let b = &s.values[3];
                let c = &s.values[4];

                if opcode.column_weights.is_empty() {
                    if opcode.constant.as_canonical_u32() == 7 {
                        add_u8_col_if_possible(&b, u8_cols);
                        add_u8_col_if_possible(&c, u8_cols);
                    }
                }
            }
            LookupType::Alu => {
                let multiplicities = convert_p3_virtual_pair_col(&s.mult);
                let opcode = convert_p3_virtual_pair_col(&s.values[0]);
                let a0 = convert_p3_virtual_pair_col(&s.values[1]);
                let a1 = convert_p3_virtual_pair_col(&s.values[2]);
                let a2 = convert_p3_virtual_pair_col(&s.values[3]);
                let a3 = convert_p3_virtual_pair_col(&s.values[4]);
                let b0 = convert_p3_virtual_pair_col(&s.values[5]);
                let b1 = convert_p3_virtual_pair_col(&s.values[6]);
                let b2 = convert_p3_virtual_pair_col(&s.values[7]);
                let b3 = convert_p3_virtual_pair_col(&s.values[8]);
                let c0 = convert_p3_virtual_pair_col(&s.values[9]);
                let c1 = convert_p3_virtual_pair_col(&s.values[10]);
                let c2 = convert_p3_virtual_pair_col(&s.values[11]);
                let c3 = convert_p3_virtual_pair_col(&s.values[12]);

                println!("send: opcode: {:?}", opcode);
                println!("send: a: {:?} {:?} {:?} {:?}", a0, a1, a2, a3);
                println!("send: b: {:?} {:?} {:?} {:?}", b0, b1, b2, b3);
                println!("send: c: {:?} {:?} {:?} {:?}", c0, c1, c2, c3);

                let tmps = vec![
                    (Opcode::ADD as u8, OpALU::Add),
                    (Opcode::SUB as u8, OpALU::Sub),
                    (Opcode::MUL as u8, OpALU::Mul),
                    (Opcode::SLT as u8, OpALU::Lt),
                    (Opcode::AND as u8, OpALU::And),
                    (Opcode::OR as u8, OpALU::Or),
                    (Opcode::XOR as u8, OpALU::Xor),
                    (Opcode::SRL as u8, OpALU::SRL),
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
                        /*
                        for i in 7..19 {
                            add_u8_col_if_possible(&s.values[i], u8_cols);
                        }*/

                        lookup_constraints.push(LatticeVMSymbolicExpr::Mul(
                            Box::new(multiplicities.clone()),
                            Box::new(impl_constraint),
                        ));
                    }
                }
            }
            _ => {}
        }
    }
}
