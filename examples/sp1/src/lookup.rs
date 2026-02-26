use std::collections::HashSet;

use p3_air::Air;
use p3_air::PairCol;
use p3_air::VirtualPairCol;
use p3_field::PrimeField32;

use sp1_core_executor::Opcode;
use sp1_stark::InteractionBuilder;
use sp1_stark::InteractionKind;

use latticevm::controlflowop::{get_control_flow_constraint, ControFLowOp};
use latticevm::interval::AbstractInterval;
use latticevm::symbolic::make_impl_constraint;
use latticevm::symbolic::GeneralLookupInfo;
use latticevm::symbolic::LatticeVMSymbolicExpr as LExpr;
use latticevm::wordop::get_alu_constraint;
use latticevm::wordop::WordOp;

use crate::p3_to_tv::convert_p3_virtual_pair_col as cv;

pub fn try_add_single_var_col<F: PrimeField32>(b: &VirtualPairCol<F>, u8_cols: &mut Vec<usize>) {
    if !b.column_weights.is_empty() {
        if let p3_air::PairCol::Main(col_idx) = b.column_weights[0].0 {
            u8_cols.push(col_idx);
        }
    }
}

pub fn get_symbolic_lookup_constraints<F, A>(
    air: &A,
    preprocessed_width: usize,
    _num_public_values: usize,
    u8_cols: &mut Vec<usize>,
    u16_cols: &mut Vec<usize>,
    multiplicities: &mut HashSet<usize>,
    air_constraints: &mut Vec<LExpr>,
    lookup_constraints: &mut Vec<LExpr>,
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
                general_lookup_info.is_real.push(cv(&r.multiplicity));
                for rv in &r.values {
                    for c in &rv.column_weights {
                        if let PairCol::Main(index) = c.0 {
                            received_vars_from_cpu.insert(index);
                        }
                    }
                }
                for i in 6..10 {
                    try_add_single_var_col(&r.values[i], &mut general_lookup_info.op_a);
                }
                for i in 10..14 {
                    try_add_single_var_col(&r.values[i], &mut general_lookup_info.op_b);
                }
                for i in 14..18 {
                    try_add_single_var_col(&r.values[i], &mut general_lookup_info.op_c);
                }
            }
            _ => {}
        }
    }

    for s in &sends {
        let multiplicities = cv(&s.multiplicity);

        match s.kind {
            InteractionKind::Program => {
                general_lookup_info.is_real.push(multiplicities.clone());
            }
            InteractionKind::Instruction => {
                let _shard = &s.values[0];
                let _clk = &s.values[1];
                let pc = cv(&s.values[2]);
                let next_pc = cv(&s.values[3]);
                let opcode = cv(&s.values[5]);

                let a = [
                    cv(&s.values[6]),
                    cv(&s.values[7]),
                    cv(&s.values[8]),
                    cv(&s.values[9]),
                ];
                let b = [
                    cv(&s.values[10]),
                    cv(&s.values[11]),
                    cv(&s.values[12]),
                    cv(&s.values[13]),
                ];
                let c = [
                    cv(&s.values[14]),
                    cv(&s.values[15]),
                    cv(&s.values[16]),
                    cv(&s.values[17]),
                ];

                // ALU Constraints
                let tmps = vec![
                    (Opcode::ADD as u8, WordOp::Add),
                    (Opcode::SUB as u8, WordOp::SubU),
                    (Opcode::MUL as u8, WordOp::Mul),
                    (Opcode::MULH as u8, WordOp::MulH),
                    (Opcode::MULHU as u8, WordOp::MulHU),
                    (Opcode::SLT as u8, WordOp::SLt),
                    (Opcode::SLTU as u8, WordOp::SLt),
                    (Opcode::AND as u8, WordOp::And),
                    (Opcode::OR as u8, WordOp::Or),
                    (Opcode::XOR as u8, WordOp::Xor),
                    (Opcode::SRL as u8, WordOp::SRL),
                ];
                for t in tmps {
                    let alu_constraint = get_alu_constraint(&a, &b, &c, &a, &t.1);
                    let impl_constraint =
                        make_impl_constraint(t.0 as i128, &opcode, alu_constraint, prime);
                    if let Some(impl_constraint) = impl_constraint {
                        for i in 6..18 {
                            try_add_single_var_col(&s.values[i], u8_cols);
                        }

                        lookup_constraints.push(LExpr::Mul(
                            Box::new(multiplicities.clone()),
                            Box::new(impl_constraint),
                        ));
                    }

                    let pc_constraint = LExpr::Sub(
                        Box::new(next_pc.clone()),
                        Box::new(LExpr::Add(
                            Box::new(pc.clone()),
                            Box::new(LExpr::Constant(AbstractInterval::from_i128(4))),
                        )),
                    );
                    let impl_pc_constraint =
                        make_impl_constraint(t.0 as i128, &opcode, pc_constraint, prime);
                    if let Some(impl_pc_constraint) = impl_pc_constraint {
                        air_constraints.push(LExpr::Mul(
                            Box::new(multiplicities.clone()),
                            Box::new(impl_pc_constraint),
                        ));
                    }
                }

                // Branch Constraints
                let tmps = vec![
                    (Opcode::BEQ as u8, ControFLowOp::BEQ),
                    (Opcode::BNE as u8, ControFLowOp::BNE),
                    (Opcode::BGE as u8, ControFLowOp::BGE),
                    (Opcode::BLT as u8, ControFLowOp::BLT),
                    (Opcode::BGEU as u8, ControFLowOp::BGEU),
                    (Opcode::BLTU as u8, ControFLowOp::BLTU),
                    (Opcode::JAL as u8, ControFLowOp::JAL),
                    (Opcode::JALR as u8, ControFLowOp::JALR),
                ];
                for t in tmps {
                    let cf_constraints =
                        get_control_flow_constraint(&pc, &next_pc, &a, &b, &c, &t.1, 4);

                    for cfc in cf_constraints {
                        let impl_constraint =
                            make_impl_constraint(t.0 as i128, &opcode, cfc, prime);
                        if let Some(impl_constraint) = impl_constraint {
                            air_constraints.push(LExpr::Mul(
                                Box::new(multiplicities.clone()),
                                Box::new(impl_constraint),
                            ));
                        }
                    }
                }
            }
            InteractionKind::Byte => {
                let opcode = &s.values[0];
                let a1 = &s.values[1];
                let a2 = &s.values[2];
                let b = &s.values[3];
                let c = &s.values[4];

                try_add_single_var_col(&b, u8_cols);
                try_add_single_var_col(&c, u8_cols);

                if opcode.column_weights.is_empty() {
                    if opcode.constant.as_canonical_u32() == 8 {
                        try_add_single_var_col(&a1, u16_cols);
                    }
                }

                let a1_expr = cv(&a1);
                let a2_expr = cv(&a2);
                let opcode_condition = cv(&opcode);

                let ops = [
                    (0, &a1_expr, LExpr::And(Box::new(cv(&b)), Box::new(cv(&c)))),
                    (1, &a1_expr, LExpr::Or(Box::new(cv(&b)), Box::new(cv(&c)))),
                    (2, &a1_expr, LExpr::Xor(Box::new(cv(&b)), Box::new(cv(&c)))),
                    (5, &a1_expr, LExpr::SRL(Box::new(cv(&b)), Box::new(cv(&c)))),
                    (
                        5,
                        &a2_expr,
                        LExpr::SRLCarry(Box::new(cv(&b)), Box::new(cv(&c))),
                    ),
                    (6, &a1_expr, LExpr::Lt(Box::new(cv(&b)), Box::new(cv(&c)))),
                    (7, &a1_expr, LExpr::Msb(Box::new(cv(&b)))),
                ];

                for (opcode, a_expr, op_expr) in ops {
                    let el_constraint = make_impl_constraint(
                        opcode,
                        &opcode_condition,
                        LExpr::Sub(Box::new(a_expr.clone()), Box::new(op_expr)),
                        prime,
                    );
                    if let Some(el_constraint) = el_constraint {
                        lookup_constraints.push(LExpr::Mul(
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
