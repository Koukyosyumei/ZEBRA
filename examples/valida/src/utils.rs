use std::collections::HashMap;
use std::collections::HashSet;


use p3_baby_bear::BabyBear;
use p3_field::{AbstractField, PrimeField32};
use p3_matrix::Matrix;

use valida_basic_api::BasicMachine;
use valida_basic_api::BasicMachineMetrics;
use valida_basic_api::ValidaRuntime;
use valida_cpu::MachineWithRegisters;
use valida_machine::symbolic::symbolic_builder::get_lookup_interactions;
use valida_machine::symbolic::symbolic_builder::get_symbolic_constraints;
use valida_machine::ChipWithPersistence;
use valida_machine::StarkConfig;
use valida_machine::{
    InstructionWord, Machine, ProgramROM, SegmentMachine,
};
use valida_program::MachineWithProgramROM;
use valida_program::ProgramTableType;

use latticevm::interval::AbstractInterval;
use latticevm::solver::RangeType;
use latticevm::symbolic::gather_boolean_variables;
use latticevm::symbolic::gather_vars;
use latticevm::symbolic::AbstractTrace;
use latticevm::symbolic::LatticeVMSymbolicExpr;
use latticevm::utils::GeneralLookupInfo;

use crate::config::{get_machine_config, prover_options};
use crate::p3_to_tv::convert_p3_expr;

pub fn make_pc_adjuster(program: Vec<InstructionWord<i32>>) -> impl Fn(&mut AbstractTrace, u32) {
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
                row[3] = AbstractInterval::i4();
                row[4] = AbstractInterval::i4();
                row[5] = AbstractInterval::i4();
                row[6] = AbstractInterval::i4();
                row[7] = AbstractInterval::i4();
                row[8] = AbstractInterval::i4();
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

pub fn generate_bootstrap_trace_from_program(
    program: &Vec<InstructionWord<i32>>,
    chip_id: usize,
    pc: u32,
    fp: u32,
) -> Vec<Vec<AbstractInterval>> {
    let config = get_machine_config();
    let (prover_opts, show_preprocessed, show_preprocessed_dims, show_public_verifier) =
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
    let (instance_data, _output) = BasicMachine::run(&mut state, &mut metrics);

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
) -> (
    Vec<LatticeVMSymbolicExpr>,
    Vec<usize>,
    HashMap<usize, RangeType>,
    GeneralLookupInfo,
)
where
    M: Machine<SC::Val>,
    SC: StarkConfig,
    C: ChipWithPersistence<M, SC>,
{
    let mut u8_cols = vec![];
    let mut multiplicities = Vec::new();
    //let mut lookup_symbolic_constraints = Vec::new();
    let mut nested_received_vars_from_cpu = Vec::new();
    let mut general_lookup_info = GeneralLookupInfo::default();

    let symbolic_constraints = get_symbolic_constraints::<M, SC, C>(&machine, &chip);
    get_lookup_interactions(
        machine,
        chip,
        &mut u8_cols,
        &mut nested_received_vars_from_cpu,
        &mut multiplicities,
    );
    let received_vars_from_cpu: Vec<_> = nested_received_vars_from_cpu
        .iter()
        .flatten()
        .cloned()
        .collect();
    if let [a, b, c, d, e, f, g, h, i, j, k, el] = received_vars_from_cpu.as_slice() {
        general_lookup_info.alu_input1.extend([*a, *b, *c, *d]);
        general_lookup_info.alu_input2.extend([*e, *f, *g, *h]);
        general_lookup_info.alu_output.extend([*i, *j, *k, *el]);
    }

    let mut refinable_cols: Vec<usize> = (0..num_cols).collect();
    refinable_cols.retain(|c| !multiplicities.contains(c));
    refinable_cols.retain(|c| !received_vars_from_cpu.contains(c));

    let tv_constraints = symbolic_constraints
        .iter()
        .map(|sc| convert_p3_expr::<SC::Val>(&sc))
        .collect::<Vec<_>>();

    let mut used_vars = HashSet::new();
    for t in &tv_constraints {
        gather_vars(0, t, &mut used_vars);
    }
    let used_var_ids: HashSet<usize> = used_vars.iter().map(|x| x.1).collect();
    refinable_cols.retain(|c| used_var_ids.contains(c));

    let multiplicities: HashSet<_> = multiplicities.iter().map(|v| *v).collect();
    let potential_boolean_vars = gather_boolean_variables(&tv_constraints, &multiplicities);

    let mut range_types: HashMap<usize, RangeType> = potential_boolean_vars
        .iter()
        .map(|k| (*k, RangeType::Bool))
        .collect();
    for c in &u8_cols {
        range_types.insert(*c, RangeType::U8);
    }

    (
        tv_constraints,
        refinable_cols,
        range_types,
        general_lookup_info,
    )
}

pub fn dummy_program_counter_refine_fn(
    abs_main_trace_data: &mut Vec<Vec<AbstractInterval>>,
    program_len: usize,
    i: usize,
    j: usize,
) {
}

pub fn dummy_adjust_pc_program(main_trace: &mut AbstractTrace, prime: u32) {}

pub fn dummy_table_deriver(
    cpu_main_trace: &Vec<Vec<AbstractInterval>>,
    range_types: &HashMap<usize, RangeType>,
    prime: u32,
) -> Vec<Vec<AbstractInterval>> {
    let out = vec![];
    out
}
