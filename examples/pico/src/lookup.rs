use std::collections::HashSet;

use crate::p3_to_tv::convert_p3_virtual_pair_col;
use p3_air::Air;
use p3_air::PairCol;
use p3_air::VirtualPairCol;
use p3_field::PrimeField32;

use pico_vm::compiler::riscv::opcode::Opcode;
use pico_vm::machine::folder::SymbolicConstraintFolder;
use pico_vm::machine::lookup::LookupType;

use latticevm::alu::get_alu_constraint;
use latticevm::alu::WordOp;
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

pub fn add_u16_col_if_possible<F: PrimeField32>(b: &VirtualPairCol<F>, u8_cols: &mut Vec<usize>) {
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
    A: Air<SymbolicConstraintFolder<F>>,
{
    let mut general_lookup_info = GeneralLookupInfo::default();
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
            LookupType::Memory => {
                println!("RRRRRRRRRR: {:?}", r);
            }
            LookupType::Alu => {
                for rv in &r.values {
                    for c in &rv.column_weights {
                        if let PairCol::Main(index) = c.0 {
                            received_vars_from_cpu.insert(index);
                        }
                    }
                }

                let _opcode = &r.values[0];
                let _a0 = &r.values[1];
                let _a1 = &r.values[2];
                let _a2 = &r.values[3];
                let _a3 = &r.values[4];
                let _b0 = &r.values[5];
                let _b1 = &r.values[6];
                let _b2 = &r.values[7];
                let _b3 = &r.values[8];
                let _c0 = &r.values[9];
                let _c1 = &r.values[10];
                let _c2 = &r.values[11];
                let _c3 = &r.values[12];

                for i in 1..13 {
                    add_single_var_col_if_possible(&r.values[i], u8_cols);
                }

                for i in 1..5 {
                    add_single_var_col_if_possible(
                        &r.values[i],
                        &mut general_lookup_info.alu_output,
                    );
                }
                for i in 5..9 {
                    add_single_var_col_if_possible(
                        &r.values[i],
                        &mut general_lookup_info.alu_input1,
                    );
                }
                for i in 9..13 {
                    add_single_var_col_if_possible(
                        &r.values[i],
                        &mut general_lookup_info.alu_input2,
                    );
                }
            }
            _ => {}
        }
    }

    for s in &sends {
        match s.kind {
            LookupType::Memory => {
                println!("SSSSSSSSSSSSS: {:?}", s);
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

                let tmps = vec![
                    (Opcode::ADD as u8, WordOp::Add),
                    (Opcode::SUB as u8, WordOp::SubU),
                    (Opcode::MUL as u8, WordOp::Mul),
                    (Opcode::MULH as u8, WordOp::MulH),
                    (Opcode::MULHU as u8, WordOp::MulHU),
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
                        lookup_constraints.push(LatticeVMSymbolicExpr::Mul(
                            Box::new(multiplicities.clone()),
                            Box::new(impl_constraint),
                        ));
                    }
                }
            }
            LookupType::Byte => {
                let opcode = &s.values[0];
                let a1 = &s.values[1];
                let a2 = &s.values[2];
                let b = &s.values[3];
                let c = &s.values[4];

                if opcode.column_weights.is_empty() {
                    if opcode.constant.as_canonical_u32() == 7 {
                        add_single_var_col_if_possible(&b, u8_cols);
                        add_single_var_col_if_possible(&c, u8_cols);
                    }

                    if opcode.constant.as_canonical_u32() == 8 {
                        add_u16_col_if_possible(&a1, u8_cols);
                    }
                }

                /*
                println!("send: opcode: {:?}", opcode);
                println!("send: a: {:?} {:?} {:?} {:?}", a1, a2, b, c);
                */

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
                        5,
                        LatticeVMSymbolicExpr::Flip(Box::new(LatticeVMSymbolicExpr::Lt(
                            Box::new(b_expr.clone()),
                            Box::new(c_expr.clone()),
                        ))), // lt(b, c) on abstractinterval returns 0 when b < c
                    ),
                    (6, LatticeVMSymbolicExpr::Msb(Box::new(b_expr.clone()))),
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

    general_lookup_info
}
