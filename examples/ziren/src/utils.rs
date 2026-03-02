use std::collections::HashSet;
use std::io;

use p3_air::{Air, PairCol, VirtualPairCol};
use p3_field::PrimeField32;
use p3_uni_stark::{get_symbolic_constraints, SymbolicAirBuilder, SymbolicExpression};

use zkm_core_executor::{ExecutionState, Executor, Opcode, Program};
use zkm_core_machine::mips::MipsAir;
use zkm_core_machine::utils::{trace_checkpoint, ZKMCoreProverError};
use zkm_stark::koala_bear_poseidon2::KoalaBearPoseidon2;
use zkm_stark::{
    CpuProver, LookupBuilder, LookupKind, MachineProver, ZKMCoreOpts, ZKM_PROOF_NUM_PV_ELTS,
};

use latticevm::constraint::LatticeVMConstraints;
use latticevm::controlflowop::get_control_flow_constraint;
use latticevm::controlflowop::ControFLowOp;
use latticevm::interval::AbstractInterval;
use latticevm::solver::prepare_constraints_and_range_type;
use latticevm::solver::ConstraintInfo;
use latticevm::symbolic::GeneralLookupInfo;
use latticevm::symbolic::{
    make_impl_constraint, LatticeVMSymbolicEntry, LatticeVMSymbolicExpr as LVSExpr,
    LatticeVMSymbolicVal,
};
use latticevm::wordop::{get_alu_constraint, WordOp};

use crate::p3_to_tv::{convert_p3_expr, convert_p3_virtual_pair_col as cv};

pub fn get_pv_constraints() -> (Vec<LVSExpr>, Vec<LVSExpr>) {
    let pv_pos_constraints = vec![LVSExpr::Sub(
        Box::new(LVSExpr::Variable(LatticeVMSymbolicVal {
            entry: LatticeVMSymbolicEntry::Public,
            index: 41,
        })),
        Box::new(LVSExpr::Constant(AbstractInterval { lo: 0, hi: 0 })),
    )];
    let pv_neg_constraints = vec![LVSExpr::Sub(
        Box::new(LVSExpr::Variable(LatticeVMSymbolicVal {
            entry: LatticeVMSymbolicEntry::Public,
            index: 40,
        })),
        Box::new(LVSExpr::Constant(AbstractInterval { lo: 0, hi: 0 })),
    )];

    (pv_pos_constraints, pv_neg_constraints)
}

pub fn run_ziren_program(program: &Program) -> Vec<(String, Vec<Vec<AbstractInterval>>)> {
    // # Execute the Target Program
    let mut runtime = Executor::new(program.clone(), ZKMCoreOpts::default());
    let result = runtime.execute_state(false);
    if result.is_err() {
        return vec![];
    }
    let (checkpoint, _done) = result.unwrap();

    let mut checkpoint_file = tempfile::tempfile()
        .map_err(ZKMCoreProverError::IoError)
        .unwrap();
    checkpoint
        .save(&mut checkpoint_file)
        .map_err(ZKMCoreProverError::IoError)
        .unwrap();

    type SC = KoalaBearPoseidon2;
    let config = KoalaBearPoseidon2::new();
    let machine = MipsAir::machine(config);
    let prover = CpuProver::new(machine);

    let mut reader = io::BufReader::new(checkpoint_file);
    let execution_state: ExecutionState =
        bincode::deserialize_from(&mut reader).expect("failed to deserialize state");
    let (records, _report) = trace_checkpoint::<SC>(
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
        let nrows = mt.1.values.len() / mt.1.width;
        let mut rows = vec![];
        for i in 0..nrows {
            let row = mt.1.row_mut(i);
            rows.push(
                row.iter()
                    .map(|v| AbstractInterval::from_i128(v.as_canonical_u32() as i128))
                    .collect(),
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
    let true_abstract_traces = run_ziren_program(&program);
    let mut base_abs_main_trace_data = vec![];
    for st in &true_abstract_traces {
        if st.0 == key {
            base_abs_main_trace_data = st.1[..num_extracted_rows].to_vec();
        }
    }

    base_abs_main_trace_data
}

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
    air_constraints: &mut Vec<LVSExpr>,
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
                general_lookup_info.is_real.push(cv(&r.multiplicity));
                for rv in &r.values {
                    for c in &rv.column_weights {
                        if let PairCol::Main(index) = c.0 {
                            received_vars_from_cpu.insert(index);
                        }
                    }
                }
                for i in 7..11 {
                    try_add_single_var_col(&r.values[i], &mut general_lookup_info.op_a);
                }
                for i in 11..15 {
                    try_add_single_var_col(&r.values[i], &mut general_lookup_info.op_b);
                }
                for i in 15..19 {
                    try_add_single_var_col(&r.values[i], &mut general_lookup_info.op_c);
                }
            }
            _ => {}
        }
    }

    for s in &sends {
        let multiplicities = cv(&s.multiplicity);

        match s.kind {
            LookupKind::Program => {
                general_lookup_info.is_real.push(multiplicities);
            }
            LookupKind::Instruction => {
                let next_pc = cv(&s.values[3]);
                let next_next_pc = cv(&s.values[4]);
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

                // ALU constraints
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
                        make_impl_constraint(t.0 as i128, &opcode, alu_constraint, prime);

                    if let Some(impl_constraint) = impl_constraint {
                        for i in 7..19 {
                            try_add_single_var_col(&s.values[i], u8_cols);
                        }
                        lookup_constraints.push(LVSExpr::Mul(
                            Box::new(multiplicities.clone()),
                            Box::new(impl_constraint),
                        ));
                    }

                    let pc_constraint = LVSExpr::Sub(
                        Box::new(next_next_pc.clone()),
                        Box::new(LVSExpr::Add(
                            Box::new(next_pc.clone()),
                            Box::new(LVSExpr::Constant(AbstractInterval::from_i128(4))),
                        )),
                    );
                    let impl_pc_constraint =
                        make_impl_constraint(t.0 as i128, &opcode, pc_constraint, prime);
                    if let Some(impl_pc_constraint) = impl_pc_constraint {
                        air_constraints.push(LVSExpr::Mul(
                            Box::new(multiplicities.clone()),
                            Box::new(impl_pc_constraint),
                        ));
                    }
                }

                // Branch Constraints
                let tmps = vec![
                    (Opcode::BEQ as u8, ControFLowOp::BEQ),
                    (Opcode::BNE as u8, ControFLowOp::BNE),
                    (Opcode::BGEZ as u8, ControFLowOp::BGE),
                    (Opcode::BGTZ as u8, ControFLowOp::BGT),
                    (Opcode::BLEZ as u8, ControFLowOp::BLE),
                    (Opcode::BLTZ as u8, ControFLowOp::BLT),
                    (Opcode::Jump as u8, ControFLowOp::Jumpi),
                    (Opcode::Jumpi as u8, ControFLowOp::Jumpi),
                    (Opcode::JumpDirect as u8, ControFLowOp::JAL),
                ];
                for t in tmps {
                    let cf_constraints =
                        get_control_flow_constraint(&next_pc, &next_next_pc, &a, &b, &c, &t.1, 4);

                    for cfc in cf_constraints {
                        let impl_constraint =
                            make_impl_constraint(t.0 as i128, &opcode, cfc, prime);
                        if let Some(impl_constraint) = impl_constraint {
                            air_constraints.push(LVSExpr::Mul(
                                Box::new(multiplicities.clone()),
                                Box::new(impl_constraint),
                            ));
                        }
                    }
                }
            }
            LookupKind::Byte => {
                let s_opcode = &s.values[0];
                let a1 = &s.values[1];
                let a2 = &s.values[2];
                let b = &s.values[3];
                let c = &s.values[4];

                try_add_single_var_col(&b, u8_cols);
                try_add_single_var_col(&c, u8_cols);

                if s_opcode.column_weights.is_empty() {
                    if s_opcode.constant.as_canonical_u32() == 8 {
                        try_add_single_var_col(&a1, u16_cols);
                    }
                }

                let ops = [
                    (0, cv(&a1), LVSExpr::And(Box::new(cv(&b)), Box::new(cv(&c)))),
                    (1, cv(&a1), LVSExpr::Or(Box::new(cv(&b)), Box::new(cv(&c)))),
                    (2, cv(&a1), LVSExpr::Xor(Box::new(cv(&b)), Box::new(cv(&c)))),
                    (5, cv(&a1), LVSExpr::SRL(Box::new(cv(&b)), Box::new(cv(&c)))),
                    (
                        5,
                        cv(&a2),
                        LVSExpr::SRLCarry(Box::new(cv(&b)), Box::new(cv(&c))),
                    ),
                    (6, cv(&a1), LVSExpr::Lt(Box::new(cv(&b)), Box::new(cv(&c)))),
                    (7, cv(&a1), LVSExpr::Msb(Box::new(cv(&b)))),
                ];

                for (opcode, a_expr, op_expr) in ops {
                    let el_constraint = make_impl_constraint(
                        opcode,
                        &cv(&s_opcode),
                        LVSExpr::Sub(Box::new(a_expr), Box::new(op_expr)),
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

pub fn extract_constraints_and_range<F, A>(
    air: &A,
    num_cols: usize,
    prime: u32,
) -> (ConstraintInfo, GeneralLookupInfo)
where
    F: p3_field::PrimeField32,
    A: Air<LookupBuilder<F>> + Air<SymbolicAirBuilder<F>>,
{
    let mut u8_cols = vec![];
    let mut u16_cols = vec![];
    let mut multiplicities = HashSet::new();
    let mut lookup_constraints = Vec::new();
    let mut received_vars_from_cpu = HashSet::new();

    let symbolic_constraints: Vec<SymbolicExpression<F>> =
        get_symbolic_constraints(air, 0, ZKM_PROOF_NUM_PV_ELTS);
    let mut air_constraints = symbolic_constraints
        .iter()
        .map(|sc| convert_p3_expr::<F>(&sc))
        .collect::<Vec<_>>();
    let general_lookup_info = get_symbolic_lookup_constraints::<F, A>(
        air,
        0,
        ZKM_PROOF_NUM_PV_ELTS,
        &mut u8_cols,
        &mut u16_cols,
        &mut multiplicities,
        &mut air_constraints,
        &mut lookup_constraints,
        &mut received_vars_from_cpu,
        prime,
    );

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
