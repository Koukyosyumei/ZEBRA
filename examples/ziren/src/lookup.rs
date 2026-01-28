use std::collections::HashSet;

use crate::p3_to_tv::convert_p3_virtual_pair_col as cv;
use p3_air::Air;
use p3_air::PairCol;
use p3_air::VirtualPairCol;
use p3_field::PrimeField32;

use zkm_core_executor::Opcode;
use zkm_stark::LookupBuilder;
use zkm_stark::LookupKind;

use latticevm::alu::get_alu_constraint;
use latticevm::alu::WordOp;
use latticevm::symbolic::make_impl_constraint;
use latticevm::symbolic::LatticeVMSymbolicExpr as LVSExpr;
use latticevm::utils::GeneralLookupInfo;

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
    num_public_values: usize,
    u8_cols: &mut Vec<usize>,
    multiplicities: &mut HashSet<usize>,
    lookup_constraints: &mut Vec<LVSExpr>,
    received_vars_from_cpu: &mut HashSet<usize>,
    prime: u32,
) -> GeneralLookupInfo
where
    F: p3_field::PrimeField32,
    A: Air<LookupBuilder<F>>,
{
    let mut general_lookup_info = GeneralLookupInfo::default();
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
                for i in 7..11 {
                    try_add_single_var_col(&r.values[i], &mut general_lookup_info.alu_output);
                }
                for i in 11..15 {
                    try_add_single_var_col(&r.values[i], &mut general_lookup_info.alu_input1);
                }
                for i in 15..19 {
                    try_add_single_var_col(&r.values[i], &mut general_lookup_info.alu_input2);
                }
            }
            _ => {}
        }
    }

    for s in &sends {
        let multiplicities = cv(&s.multiplicity);

        match s.kind {
            LookupKind::Instruction => {
                let opcode = cv(&s.values[6]);
                let a = [
                    cv(&s.values[7]),
                    cv(&s.values[8]),
                    cv(&s.values[9]),
                    cv(&s.values[10]),
                ];
                let b = [
                    cv(&s.values[11]),
                    cv(&s.values[12]),
                    cv(&s.values[13]),
                    cv(&s.values[14]),
                ];
                let c = [
                    cv(&s.values[15]),
                    cv(&s.values[16]),
                    cv(&s.values[17]),
                    cv(&s.values[18]),
                ];
                let hi = [
                    cv(&s.values[19]),
                    cv(&s.values[20]),
                    cv(&s.values[21]),
                    cv(&s.values[22]),
                ];

                for t in [
                    (Opcode::ADD as u8, WordOp::Add),
                    (Opcode::SUB as u8, WordOp::SubU),
                    (Opcode::MUL as u8, WordOp::Mul),
                    (Opcode::MULT as u8, WordOp::MulTL),
                    (Opcode::MULT as u8, WordOp::MulTH),
                    (Opcode::MULTU as u8, WordOp::MulTUL),
                    (Opcode::MULTU as u8, WordOp::MulTUH),
                    (Opcode::SLT as u8, WordOp::SLt),
                    (Opcode::AND as u8, WordOp::And),
                    (Opcode::OR as u8, WordOp::Or),
                    (Opcode::XOR as u8, WordOp::Xor),
                    (Opcode::SRL as u8, WordOp::SRL),
                ] {
                    let alu_constraint = get_alu_constraint(&a, &b, &c, &hi, &t.1);
                    let impl_constraint =
                        make_impl_constraint(t.0 as i64, &opcode, alu_constraint, prime);

                    if let Some(impl_constraint) = impl_constraint {
                        for i in 7..19 {
                            try_add_single_var_col(&s.values[i], u8_cols);
                        }
                        lookup_constraints.push(LVSExpr::Mul(
                            Box::new(multiplicities.clone()),
                            Box::new(impl_constraint),
                        ));
                    }
                }
            }
            LookupKind::Byte => {
                let s_opcode = &s.values[0];
                let a1 = &s.values[1];
                let a2 = &s.values[2];
                let b = &s.values[3];
                let c = &s.values[4];

                // Range U8
                try_add_single_var_col(&b, u8_cols);
                try_add_single_var_col(&c, u8_cols);
                try_add_single_var_col(&a1, u8_cols);

                let ops = [
                    (0, LVSExpr::And(Box::new(cv(&b)), Box::new(cv(&c)))),
                    (1, LVSExpr::Or(Box::new(cv(&b)), Box::new(cv(&c)))),
                    (2, LVSExpr::Xor(Box::new(cv(&b)), Box::new(cv(&c)))),
                    (
                        6,
                        LVSExpr::Flip(Box::new(LVSExpr::Lt(Box::new(cv(&b)), Box::new(cv(&c))))),
                    ),
                ];

                for (opcode, op_expr) in ops {
                    let el_constraint = make_impl_constraint(
                        opcode,
                        &cv(&s_opcode),
                        LVSExpr::Sub(Box::new(cv(&a1)), Box::new(op_expr)),
                        prime,
                    );
                    if let Some(el_constraint) = el_constraint {
                        lookup_constraints.push(LVSExpr::Mul(
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
