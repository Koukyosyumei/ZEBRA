use std::collections::HashSet;
use std::sync::Arc;

use p3_air::{Air, PairCol, VirtualPairCol};
use p3_field::PrimeField32;
use p3_koala_bear::KoalaBear;
use p3_uni_stark::SymbolicExpression;

use pico_vm::{
    chips::chips::public_values::columns::NUM_PUBLIC_VALUES_COLS,
    compiler::riscv::{opcode::Opcode, program::Program},
    configs::config::StarkGenericConfig,
    emulator::opts::EmulatorOpts,
    emulator::riscv::emulator::RiscvEmulator,
    instances::{
        chiptype::riscv_chiptype::RiscvChipType,
        configs::embed_kb_bn254_poseidon2::KoalaBearBn254Poseidon2, machine::riscv::RiscvMachine,
    },
    machine::{
        folder::SymbolicConstraintFolder, lookup::LookupType, machine::MachineBehavior,
        utils::get_symbolic_constraints,
    },
    primitives::consts::RISCV_NUM_PVS,
};

use latticevm::alu::{get_alu_constraint, WordOp};
use latticevm::interval::AbstractInterval;
use latticevm::quick::ConstraintInfo;
use latticevm::solver::prepare_constraints_and_range_type;
use latticevm::symbolic::{
    make_impl_constraint, LatticeVMConstraints, LatticeVMSymbolicExpr as LVSExpr,
};
use latticevm::utils::GeneralLookupInfo;

use crate::p3_to_tv::convert_p3_expr;
use crate::p3_to_tv::convert_p3_virtual_pair_col as cv;

pub fn run_pico_program(program: &Program) -> Vec<(String, Vec<Vec<AbstractInterval>>)> {
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

    let mut chips_and_main_traces = riscv_machine
        .base_machine()
        .prover
        .generate_main(&riscv_machine.chips(), chunk);

    let mut true_abs_traces = vec![];
    for mt in &mut chips_and_main_traces {
        let nrows = mt.1.values.len() / mt.1.width;
        let mut rows: Vec<_> = vec![];
        for i in 0..nrows {
            let row = mt.1.row_mut(i);
            rows.push(
                row.iter()
                    .map(|v| AbstractInterval::from_i64(v.as_canonical_u32() as i64))
                    .collect::<Vec<_>>(),
            );
        }
        true_abs_traces.push((mt.0.clone(), rows));
    }

    true_abs_traces
}

pub fn get_program_str(program: &Program) -> String {
    program
        .instructions
        .iter()
        .map(|inst| format!("{:?}\n", inst))
        .collect::<String>()
}

pub fn generate_abstract_trace(
    program: &Program,
    key: String,
    num_extracted_rows: usize,
) -> Vec<Vec<AbstractInterval>> {
    let true_abstract_traces = run_pico_program(&program);
    let mut base_abs_main_trace_data = vec![];
    for st in &true_abstract_traces {
        if st.0 == key {
            base_abs_main_trace_data = st.1[..num_extracted_rows].to_vec();
        }
    }

    base_abs_main_trace_data
}

pub fn add_u8_col_if_possible<F: PrimeField32>(b: &VirtualPairCol<F>, u8_cols: &mut Vec<usize>) {
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
    _num_public_values: usize,
    u8_cols: &mut Vec<usize>,
    u16_cols: &mut Vec<usize>,
    multiplicities: &mut HashSet<usize>,
    lookup_constraints: &mut Vec<LVSExpr>,
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
            LookupType::Memory => {}
            LookupType::Alu => {
                for rv in &r.values {
                    for c in &rv.column_weights {
                        if let PairCol::Main(index) = c.0 {
                            received_vars_from_cpu.insert(index);
                        }
                    }
                }

                for i in 1..13 {
                    add_u8_col_if_possible(&r.values[i], u8_cols);
                }

                for i in 1..5 {
                    add_u8_col_if_possible(&r.values[i], &mut general_lookup_info.alu_output);
                }
                for i in 5..9 {
                    add_u8_col_if_possible(&r.values[i], &mut general_lookup_info.alu_input1);
                }
                for i in 9..13 {
                    add_u8_col_if_possible(&r.values[i], &mut general_lookup_info.alu_input2);
                }
            }
            _ => {}
        }
    }

    for s in &sends {
        match s.kind {
            LookupType::Memory => {}
            LookupType::Alu => {
                let multiplicities = cv(&s.mult);
                let opcode = cv(&s.values[0]);
                let a = [
                    cv(&s.values[1]),
                    cv(&s.values[2]),
                    cv(&s.values[3]),
                    cv(&s.values[4]),
                ];
                let b = [
                    cv(&s.values[5]),
                    cv(&s.values[6]),
                    cv(&s.values[7]),
                    cv(&s.values[8]),
                ];
                let c = [
                    cv(&s.values[9]),
                    cv(&s.values[10]),
                    cv(&s.values[11]),
                    cv(&s.values[12]),
                ];

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
                        make_impl_constraint(t.0 as i64, &opcode, alu_constraint, prime);

                    if let Some(impl_constraint) = impl_constraint {
                        lookup_constraints.push(LVSExpr::Mul(
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

                add_u8_col_if_possible(&b, u8_cols);
                add_u8_col_if_possible(&c, u8_cols);

                if opcode.column_weights.is_empty() {
                    if opcode.constant.as_canonical_u32() == 8 {
                        add_u16_col_if_possible(&a1, u16_cols);
                    }
                }

                let opcode_condition = cv(&opcode);

                let ops = [
                    (0, cv(&a1), LVSExpr::And(Box::new(cv(&b)), Box::new(cv(&c)))),
                    (1, cv(&a1), LVSExpr::Or(Box::new(cv(&b)), Box::new(cv(&c)))),
                    (2, cv(&a1), LVSExpr::Xor(Box::new(cv(&b)), Box::new(cv(&c)))),
                    (3, cv(&a1), LVSExpr::SRL(Box::new(cv(&b)), Box::new(cv(&c)))),
                    (
                        4,
                        cv(&a2),
                        LVSExpr::SRLCarry(Box::new(cv(&b)), Box::new(cv(&c))),
                    ),
                    (
                        5,
                        cv(&a1),
                        LVSExpr::Flip(Box::new(LVSExpr::Lt(Box::new(cv(&b)), Box::new(cv(&c))))), // lt(b, c) on abstractinterval returns 0 when b < c
                    ),
                    (6, cv(&a1), LVSExpr::Msb(Box::new(cv(&b)))),
                ];

                for (opcode, a_expr, op_expr) in ops {
                    let el_constraint = make_impl_constraint(
                        opcode,
                        &opcode_condition,
                        LVSExpr::Sub(Box::new(a_expr.clone()), Box::new(op_expr)),
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

pub fn extract_constraints_and_range<F, A>(
    air: &A,
    num_cols: usize,
    prime: u32,
) -> (ConstraintInfo, GeneralLookupInfo)
where
    F: p3_field::PrimeField32,
    A: Air<SymbolicConstraintFolder<F>>,
{
    let mut u8_cols = vec![];
    let mut u16_cols = vec![];
    let mut multiplicities = HashSet::new();
    let mut lookup_constraints = Vec::new();
    let mut received_vars_from_cpu = HashSet::new();

    let symbolic_constraints: Vec<SymbolicExpression<F>> = get_symbolic_constraints(air, 0);
    let general_lookup_info = get_symbolic_lookup_constraints::<F, A>(
        air,
        0,
        NUM_PUBLIC_VALUES_COLS,
        &mut u8_cols,
        &mut u16_cols,
        &mut multiplicities,
        &mut lookup_constraints,
        &mut received_vars_from_cpu,
        prime,
    );

    let mut air_constraints = symbolic_constraints
        .iter()
        .map(|sc| convert_p3_expr::<F>(&sc))
        .collect::<Vec<_>>();

    let (refinable_cols, range_types) = prepare_constraints_and_range_type(
        num_cols,
        &u8_cols,
        &u16_cols,
        &multiplicities,
        &received_vars_from_cpu,
        &mut air_constraints,
        &lookup_constraints,
        prime,
    );

    let constraints = LatticeVMConstraints::new(air_constraints, lookup_constraints);
    let constraint_info = ConstraintInfo {
        constraints: constraints,
        num_total_columns: num_cols,
        num_pv_columns: 0,
        output_columns: vec![],
        refinable_cols: refinable_cols,
        range_types: range_types,
        prime: prime,
    };

    (constraint_info, general_lookup_info)
}
