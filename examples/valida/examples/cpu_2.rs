use std::collections::HashSet;
use std::fs;
use std::io;

use p3_baby_bear::BabyBear;
use p3_field::AbstractField;
//use p3_uni_stark::symbolic_builder::get_symbolic_constraints;

use valida_alu_u32::add::Add32Instruction;
use valida_basic_api::BasicMachine;
use valida_cpu::BneInstruction;
use valida_cpu::Imm32Instruction;
use valida_cpu::StopInstruction;
use valida_cpu::{
    columns::{CPU_COL_MAP, NUM_CPU_COLS},
    CpuChip,
};
use valida_machine::{Instruction, InstructionWord, Operands, StarkField};
use valida_opcodes::BYTES_PER_INSTR;

use latticevm::interval::{AbstractInterval, MayBeFlag};
use latticevm::quick::quick_api;
use latticevm::solver::{
    dummy_adjust_pc_program, dummy_program_counter_refine_fn, dummy_table_deriver, RangeType,
};
use latticevm::state::AbstractState;
use latticevm::state::MemoryOp;
use latticevm::state::MemoryOpKind;
use latticevm::symbolic::{AbstractTrace, LatticeVMConstraints};
use latticevm::ui::save_repr_if_unique;
use latticevm::ui::UiState;
use latticevm::utils::create_or_clear_dir;

use latticevm_valida::config::MyConfig;
use latticevm_valida::utils::{
    extract_constraints_and_range, generate_bootstrap_trace_from_program, make_pc_adjuster,
    refine_pc_interval,
};

fn reconstruct_word(row: &[AbstractInterval], base: usize) -> AbstractInterval {
    let mut val = AbstractInterval::from_i64(0);
    let mut mul = 1_i64;
    for i in 0..4 {
        val = val + row[base + i].clone() * AbstractInterval::from_i64(mul);
        mul *= 256;
    }
    val
}

pub fn valida_abstract_trace_to_abstract_state(
    abstract_row: &Vec<AbstractInterval>,
    prime: u32,
) -> AbstractState {
    let mut memory_ops = Vec::new();

    if abstract_row[30].is_non_zero(prime) != MayBeFlag::False {
        memory_ops.push(MemoryOp {
            kind: MemoryOpKind::Read,
            addr: abstract_row[31].clone(),
            value: reconstruct_word(abstract_row, 32),
        });
    }

    if abstract_row[36].is_non_zero(prime) != MayBeFlag::False {
        memory_ops.push(MemoryOp {
            kind: MemoryOpKind::Read,
            addr: abstract_row[37].clone(),
            value: reconstruct_word(abstract_row, 38),
        });
    }

    if abstract_row[42].is_non_zero(prime) != MayBeFlag::False {
        memory_ops.push(MemoryOp {
            kind: MemoryOpKind::Write,
            addr: abstract_row[43].clone(),
            value: reconstruct_word(abstract_row, 44),
        });
    }

    AbstractState {
        clk: abstract_row[0].clone(),
        pc: abstract_row[1].clone(),
        is_done: match abstract_row[24].clone().is_zero(prime) {
            MayBeFlag::True => MayBeFlag::False,
            MayBeFlag::False => MayBeFlag::True,
            MayBeFlag::MayBe => MayBeFlag::MayBe,
        },
        memory_ops,
    }
}

// ############## Final Check Function ##############################
fn final_check(
    trace: &AbstractTrace,
    num_trial: usize,
    prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    let mut output = String::new();

    let mut recovered_states = vec![];
    for row in &trace.data {
        if let MayBeFlag::False = row[58].is_zero(prime) {
            recovered_states.push(valida_abstract_trace_to_abstract_state(row, prime));
        }
    }

    let mut string_representation = String::new();
    for state in &recovered_states {
        string_representation.push_str(&format!("{}\n", state));
        if state.is_done != MayBeFlag::False {
            break;
        }
    }

    save_repr_if_unique(&string_representation, known_reprt, ui);
}

fn get_target_program<Val: StarkField>() -> Vec<InstructionWord<i32>> {
    let bytes_per_instr = BYTES_PER_INSTR as i32;

    let mut program = vec![];
    program.extend([
        InstructionWord {
            opcode: <Imm32Instruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands([-4, 2, 0, 0, 0]),
        },
        InstructionWord {
            opcode: <StopInstruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands::default(),
        },
    ]);

    program
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(27) + 1;

    // ######################## Solver Parameters ###############################
    let max_iteration = 100000;
    let min_row_id = 0;
    let max_row_id = 1;
    let seed = 41;
    let base_pc = 0;

    // ######################## Extract Add Constraints ##########################
    // Columns reserved for program counters / instructions
    let program_cols = (3..8).collect::<Vec<_>>();

    println!("CPU AIR MAP");
    println!("  {:?}", CPU_COL_MAP);

    let air = CpuChip::default();
    let num_col = NUM_CPU_COLS;
    let chip_idx = 0;

    let machine = BasicMachine::<BabyBear>::default();
    let (
        air_constraints,
        lookup_constraints,
        mut refinable_cols,
        mut range_types,
        general_lookup_info,
    ) = extract_constraints_and_range::<BasicMachine<BabyBear>, MyConfig, _>(
        &machine, &air, num_col, prime,
    );
    refinable_cols.retain(|x| !program_cols.contains(x));

    let mut public_vals = vec![AbstractInterval::zero(); 3];
    public_vals[0] = AbstractInterval::from_i64(0);
    public_vals[1] = AbstractInterval::from_i64(4096);
    public_vals[2] = AbstractInterval::from_i64(1);
    //let refinment_target_indicies_pv: Vec<usize> = vec![0, 1, 2];

    let constraints = LatticeVMConstraints {
        air_constraints,
        lookup_constraints,
        pv_pos_constraints: vec![],
        pv_neg_constraints: vec![],
    };
    let minimum_num_taregt_cols = 3; //refinable_cols.len();

    // ######################## Program Initialization ###########################
    let program = get_target_program::<BabyBear>();
    range_types.insert(
        1,
        RangeType::Any(base_pc, base_pc + (program.len() as i64) - 1),
    );
    let program_str = program
        .iter()
        .map(|inst| format!("{}\n", inst))
        .collect::<String>();

    let base_abs_main_trace_data =
        generate_bootstrap_trace_from_program(&program, chip_idx, base_pc as u32, 0x1000);
    let adjust_pc_program = make_pc_adjuster(program.clone());

    let mut rs: HashSet<usize> = HashSet::new();
    for i in 0..base_abs_main_trace_data.len() {
        for j in 0..base_abs_main_trace_data[i].len() {
            if base_abs_main_trace_data[i][j].as_canonical_u32(prime) != 0 {
                rs.insert(j);
            }
        }
    }
    refinable_cols.clear();
    for j in rs.iter() {
        refinable_cols.push(*j);
    }
    println!("------- {:?}", refinable_cols);

    // ######################## Solve ############################################
    quick_api(
        program_str,
        &constraints,
        &refinable_cols,
        &range_types,
        &vec![],
        &base_abs_main_trace_data,
        public_vals,
        max_iteration,
        minimum_num_taregt_cols,
        min_row_id,
        max_row_id,
        program.len(),
        dummy_program_counter_refine_fn,
        adjust_pc_program,
        final_check,
        prime,
        seed,
    )
}
