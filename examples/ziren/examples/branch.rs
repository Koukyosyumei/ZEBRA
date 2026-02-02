use std::collections::HashSet;
use std::fs;
use std::io;
use std::mem::transmute;

use itertools::Itertools;

use p3_koala_bear::KoalaBear;

use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::control_flow::BranchColumns;
use zkm_core_machine::control_flow::NUM_BRANCH_COLS;
use zkm_core_machine::BranchChip;
use zkm_stark::MachineProver;

use latticevm::interval::AbstractInterval;
use latticevm::quick::quick_api;
use latticevm::smt::expr_to_smt_bv;
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn, RangeType};
use latticevm::symbolic::LatticeVMSymbolicEntry;
use latticevm::symbolic::LatticeVMSymbolicExpr;
use latticevm::symbolic::LatticeVMSymbolicVal;
use latticevm::ui::save_repr_if_unique;
use latticevm::ui::{generate_alu_final_checker, UiState};
use latticevm::utils::trace_fmt_with_idxs;
use latticevm::utils::{create_or_clear_dir, indices_arr};
use latticevm::{symbolic::AbstractTrace, symbolic::LatticeVMConstraints};

use latticevm_ziren::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

// ############## Final Check Function ##############################
fn final_check(
    trace: &AbstractTrace,
    _num_trial: usize,
    _prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    let string_representation = format!(
        "pc: {}, next_pc: [{}], next_next_pc: [{}], op_a_value: [{}], op_b_value: [{}], op_c_value: [{}]",
        trace.data[0][0],
        trace_fmt_with_idxs(trace, 0, &[1, 2, 3, 4]),
        trace_fmt_with_idxs(trace, 0, &[23, 24, 25, 26]),
        trace_fmt_with_idxs(trace, 0, &[41, 42, 43, 44]),
        trace_fmt_with_idxs(trace, 0, &[45, 46, 47, 48]),
        trace_fmt_with_idxs(trace, 0, &[49, 50, 51, 52]),
    );
    save_repr_if_unique(&string_representation, known_reprt, ui);
}

const fn make_col_map() -> BranchColumns<usize> {
    let indices_arr = indices_arr::<{ NUM_BRANCH_COLS }>();
    unsafe { transmute::<[usize; NUM_BRANCH_COLS], BranchColumns<usize>>(indices_arr) }
}

pub fn target_program(
    opcode: Opcode,
    pc_start: u32,
    pc_base: u32,
    x: u32,
    y: u32,
    z: u32,
) -> Program {
    let instructions = vec![
        Instruction::new(Opcode::ADD, 1, 0, x, true, true),
        Instruction::new(Opcode::ADD, 2, 0, y, true, true),
        Instruction::new(opcode, 1, 2, z, true, true),
    ];
    Program::new(instructions, pc_start, pc_base)
}

pub fn get_opcode_addsub(target_opcode: &str) -> Opcode {
    match target_opcode {
        "BEQ" => Opcode::BEQ,
        "BGEZ" => Opcode::BGEZ,
        "BGTZ" => Opcode::BGTZ,
        "BLEZ" => Opcode::BLEZ,
        "BLTZ" => Opcode::BLTZ,
        "BNE" => Opcode::BNE,
        _ => panic!("unsupported instruction"),
    }
}

fn main() -> Result<(), io::Error> {
    let target_opcode = "BEQ";

    create_or_clear_dir("voutput")?;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;

    // ######################## Solver Parameters ###############################
    let max_iteration = 1000000000;
    let min_row_id = 0;
    let max_row_id = 0;
    let num_extracted_rows = 1;
    let seed = 41;

    // ######################## Extract CPU Constraints ##########################
    let air = BranchChip::default();
    let air_name = "Branch";
    let _colmap = make_col_map();

    let (
        air_constraints,
        lookup_constraints,
        mut refinable_cols,
        mut range_types,
        general_lookup_info,
    ) = extract_constraints_and_range::<KoalaBear, BranchChip>(&air, NUM_BRANCH_COLS, prime);
    refinable_cols.extend(&[23, 24, 25, 26]);

    let nc = vec![LatticeVMSymbolicExpr::Sub(
        Box::new(LatticeVMSymbolicExpr::Variable(LatticeVMSymbolicVal {
            entry: LatticeVMSymbolicEntry::Main { is_curr: true },
            index: 1,
        })),
        Box::new(LatticeVMSymbolicExpr::Constant(AbstractInterval::from_i64(
            16,
        ))),
    )];

    let constraints = LatticeVMConstraints {
        air_constraints,
        lookup_constraints,
        pv_pos_constraints: vec![],
        pv_neg_constraints: nc,
    };
    let minimum_num_taregt_cols = refinable_cols.len();

    // ######################## Program Initialization ###########################
    let program = target_program(get_opcode_addsub(target_opcode), 4, 4, 3, 4, 12);
    let base_abs_main_trace_data =
        generate_abstract_trace(&program, air_name.to_string(), num_extracted_rows);

    let mut constants: Vec<(usize, usize, AbstractInterval)> = vec![];
    let mut neg_constants: Vec<(usize, usize, AbstractInterval)> = vec![];
    for j in 0..NUM_BRANCH_COLS {
        if !refinable_cols.contains(&j) {
            constants.push((0, j, base_abs_main_trace_data[0][j].clone()));
        }
    }
    for j in vec![23, 24, 25, 26] {
        neg_constants.push((0, j, base_abs_main_trace_data[0][j].clone()));
    }
    let smt_str = expr_to_smt_bv(
        &constraints,
        &constants,
        &neg_constants,
        &range_types,
        1,
        NUM_BRANCH_COLS,
        0,
        prime,
    );
    println!("{}", smt_str);
    println!("rr: {:?}", range_types);

    // ######################## Solve ############################################
    quick_api(
        get_program_str(&program),
        &constraints,
        &refinable_cols,
        &range_types,
        &vec![],
        &base_abs_main_trace_data,
        vec![],
        max_iteration,
        minimum_num_taregt_cols,
        min_row_id,
        max_row_id,
        program.instructions.len(),
        dummy_program_counter_refine_fn,
        dummy_adjust_pc_program,
        final_check,
        prime,
        seed,
    )
}
