use std::collections::HashSet;

use p3_air::{Air, VirtualPairCol};
use p3_baby_bear::BabyBear;
use p3_field::{Field, PrimeField32};
use p3_matrix::Matrix;

use valida_basic_api::{BasicMachine, BasicMachineMetrics, ValidaRuntime};
use valida_cpu::MachineWithRegisters;
use valida_machine::{
    symbolic::symbolic_builder::{get_symbolic_constraints, SymbolicAirBuilder},
    BusArgument, ChipWithPersistence, InstructionWord, InteractionType, Machine, ProgramROM,
    SegmentMachine, StarkConfig, ValidaAirBuilder,
};
use valida_opcodes::Opcode;
use valida_program::{MachineWithProgramROM, ProgramTableType};

use latticevm::constraint::LatticeVMConstraints;
use latticevm::interval::AbstractInterval;
use latticevm::solver::prepare_constraints_and_range_type;
use latticevm::solver::ConstraintInfo;
use latticevm::symbolic::GeneralLookupInfo;
use latticevm::symbolic::{make_impl_constraint, LatticeVMSymbolicExpr as LVSExpr};
use latticevm::trace::AbstractTrace;
use latticevm::wordop::{get_alu_constraint, WordOp};

use crate::config::{get_machine_config, prover_options};
use crate::p3_to_tv::{convert_p3_expr, convert_p3_virtual_pair_col as cv};

pub fn try_add_single_var_col<F: PrimeField32>(b: &VirtualPairCol<F>, u8_cols: &mut Vec<usize>) {
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
    lookup_constraints: &mut Vec<LVSExpr>,
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
        match interaction_type {
            InteractionType::LocalSend => {}
            InteractionType::LocalReceive => {}
            InteractionType::GlobalSend => match e_interaction.argument_index {
                BusArgument::Local(_) => {}
                BusArgument::Global(id) => {
                    if id == 0 {
                        let opcode = cv(&e_interaction.fields[0]);
                        let b = [
                            cv(&e_interaction.fields[1]),
                            cv(&e_interaction.fields[2]),
                            cv(&e_interaction.fields[3]),
                            cv(&e_interaction.fields[4]),
                        ];
                        let c = [
                            cv(&e_interaction.fields[5]),
                            cv(&e_interaction.fields[6]),
                            cv(&e_interaction.fields[7]),
                            cv(&e_interaction.fields[8]),
                        ];
                        let a = [
                            cv(&e_interaction.fields[9]),
                            cv(&e_interaction.fields[10]),
                            cv(&e_interaction.fields[11]),
                            cv(&e_interaction.fields[12]),
                        ];

                        let multiplicities = cv(&e_interaction.count);

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
                            let alu_constraint = get_alu_constraint(&a, &b, &c, &a, &t.1);
                            let impl_constraint =
                                make_impl_constraint(t.0 as i64, &opcode, alu_constraint, prime);

                            if let Some(impl_constraint) = impl_constraint {
                                for i in 1..13 {
                                    try_add_single_var_col(&e_interaction.fields[i], range_u8_cols);
                                }
                                lookup_constraints.push(LVSExpr::Mul(
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
                        let opcode_condition = cv(&e_interaction.fields[0]);
                        let input = cv(&e_interaction.fields[1]);
                        let output = cv(&e_interaction.fields[2]);
                        let multiplicities = cv(&e_interaction.count);

                        let ops = [(1, LVSExpr::Msb(Box::new(input.clone())))];
                        for (opcode, op_expr) in ops {
                            let el_constraint = make_impl_constraint(
                                opcode,
                                &opcode_condition,
                                LVSExpr::Sub(Box::new(output.clone()), Box::new(op_expr)),
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
    lookup_constraints: &mut Vec<LVSExpr>,
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

pub fn generate_bootstrap_trace_from_program(
    program: &Vec<InstructionWord<i32>>,
    chip_id: usize,
    pc: u32,
    fp: u32,
) -> Vec<Vec<AbstractInterval>> {
    let config = get_machine_config();
    let (prover_opts, _show_preprocessed, _show_preprocessed_dims, _show_public_verifier) =
        prover_options();

    let rom = ProgramROM::new(program.clone());
    let mut machine = BasicMachine::<BabyBear>::default();
    machine.set_segment_number(0);
    machine.set_max_trace_height(65536);
    machine.set_program_rom(rom, ProgramTableType::Public);
    machine.set_initial_register_values(valida_cpu::Registers { pc: pc, fp: fp });

    let mut runtime = ValidaRuntime::default_for_field::<BabyBear>();
    let mut state = machine.start(&mut runtime);
    let mut metrics = BasicMachineMetrics::initialize();
    let (_instance_data, _output) = BasicMachine::run(&mut state, &mut metrics);

    let mut traces = state.machine.generate_traces(&config, prover_opts);

    // ############# Obtain the inital solution ############################
    let mut rows = vec![];
    if let Some(traces) = &mut traces.1[chip_id] {
        let nrows = traces.values.len() / traces.width();
        for i in 0..nrows {
            let row = traces.row_mut(i);
            rows.push(
                row.iter()
                    .map(|v| AbstractInterval::from_i64(v.as_canonical_u32() as i64))
                    .collect(),
            );
        }
    }
    rows
}

pub fn extract_constraints_and_range<M, SC, C>(
    machine: &M,
    chip: &C,
    num_cols: usize,
    prime: u32,
) -> (ConstraintInfo, GeneralLookupInfo)
where
    M: Machine<SC::Val>,
    SC: StarkConfig,
    C: ChipWithPersistence<M, SC>,
{
    let mut u8_cols = vec![];
    let mut u16_cols = vec![];
    let mut multiplicities = Vec::new();
    let mut lookup_constraints = Vec::new();
    let mut nested_received_vars_from_cpu = Vec::new();
    let mut general_lookup_info = GeneralLookupInfo::default();

    let symbolic_constraints = get_symbolic_constraints::<M, SC, C>(&machine, &chip);
    get_lookup_interactions(
        machine,
        chip,
        &mut u8_cols,
        &mut nested_received_vars_from_cpu,
        &mut multiplicities,
        &mut lookup_constraints,
        prime,
    );

    let received_vars_from_cpu: Vec<_> = nested_received_vars_from_cpu
        .iter()
        .skip(1)
        .flatten()
        .cloned()
        .collect();

    if let [a, b, c, d, e, f, g, h, i, j, k, el] = received_vars_from_cpu.as_slice() {
        general_lookup_info.op_b.extend([*a, *b, *c, *d]);
        general_lookup_info.op_c.extend([*e, *f, *g, *h]);
        general_lookup_info.op_a.extend([*i, *j, *k, *el]);
    }

    if let [a, b, c, d, e, f, g, h, i] = received_vars_from_cpu.as_slice() {
        general_lookup_info.op_b.extend([*a, *b, *c, *d]);
        general_lookup_info.op_c.extend([*e, *f, *g, *h]);
        general_lookup_info.op_a.extend([*i]);
    }

    let mut refinable_cols: Vec<usize> = (0..num_cols).collect();
    refinable_cols.retain(|c| !multiplicities.contains(c));
    refinable_cols.retain(|c| !received_vars_from_cpu.contains(c));

    let mut air_constraints = symbolic_constraints
        .iter()
        .map(|sc| convert_p3_expr::<SC::Val>(&sc))
        .collect::<Vec<_>>();
    let multiplicities: HashSet<_> = multiplicities.iter().map(|v| *v).collect();
    let received_vars_from_cpu: HashSet<_> = received_vars_from_cpu.iter().map(|v| *v).collect();

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

pub fn make_pc_adjuster(
    program: Vec<InstructionWord<i32>>,
) -> impl Fn(&mut AbstractTrace, u32) + Clone {
    move |main_trace: &mut AbstractTrace, prime: u32| {
        for row in &mut main_trace.data {
            if row[1].is_singleton() {
                let pc = row[1].as_canonical_u32(prime) as usize;
                if pc < program.len() {
                    let instr = program[pc];
                    row[3] = AbstractInterval::from_i64(instr.opcode.into());
                    row[4] = AbstractInterval::from_i64(instr.operands.0[0].into());
                    row[5] = AbstractInterval::from_i64(instr.operands.0[1].into());
                    row[6] = AbstractInterval::from_i64(instr.operands.0[2].into());
                    row[7] = AbstractInterval::from_i64(instr.operands.0[3].into());
                    row[8] = AbstractInterval::from_i64(instr.operands.0[4].into());

                    if row[3].as_canonical_u32(prime) == 8 {
                        for i in 4..57 {
                            row[i] = AbstractInterval::zero();
                        }
                        row[24] = AbstractInterval::from_i64(1);
                    }
                }
            } else {
                for i in 3..9 {
                    row[i] = AbstractInterval::i4();
                }
            }
        }
    }
}

pub fn refine_pc_interval(
    abs_main_trace_data: &mut Vec<Vec<AbstractInterval>>,
    program_len: usize,
    i: usize,
    j: usize,
) {
    if j == 1 {
        abs_main_trace_data[i][j] = AbstractInterval {
            lo: 0,
            hi: (program_len - 1) as i64,
        };
    }
}
