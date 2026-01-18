use itertools::Itertools;
use std::collections::HashSet;
use std::fs;
use std::io;

use p3_koala_bear::KoalaBear;

use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::{
    cpu::columns::{CPU_COL_MAP, NUM_CPU_COLS},
    CpuChip,
};
use zkm_stark::MachineProver;
use zkm_stark::ZKM_PROOF_NUM_PV_ELTS;

use latticevm::interval::AbstractInterval;
use latticevm::quick::quick_api;
use latticevm::solver::{dummy_adjust_pc_program, dummy_program_counter_refine_fn};
use latticevm::ui::UiState;
use latticevm::utils::{create_or_clear_dir, indices_arr};
use latticevm::{symbolic::AbstractTrace, symbolic::LatticeVMConstraints};

use latticevm_ziren::pv_constraints::get_pv_constraints;
use latticevm_ziren::state::ziren_abstract_trace_to_abstract_state;
use latticevm_ziren::utils::{
    extract_constraints_and_range, generate_abstract_trace, get_program_str,
};

// ############## Final Check Function ##############################
fn final_check(
    trace: &AbstractTrace,
    num_trial: usize,
    prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    let mut string_representation = String::new();
    let recovered_states = trace
        .data
        .iter()
        .map(|row| ziren_abstract_trace_to_abstract_state(row, prime))
        .collect::<Vec<_>>();
    string_representation.push_str("Malicious States:\n");
    for rs in &recovered_states {
        string_representation.push_str(&format!("\t{}\n", rs));
    }
    string_representation.push_str("-----------------\n");

    if !known_reprt.contains(&string_representation) {
        known_reprt.insert(string_representation.clone());
        ui.recovered = string_representation;

        fs::write(
            format!("voutput/{}_states.txt", known_reprt.len()),
            ui.recovered.clone(),
        )
        .unwrap();
        fs::write(
            format!("voutput/{}_assignments.txt", known_reprt.len()),
            ui.logs.clone(),
        )
        .unwrap();
    }
}

pub fn target_program(pc_start: u32, pc_base: u32) -> Program {
    // this program is expected to invalid according to the semantics of ziren, while
    // we can find the satisfying solution.
    let instructions = vec![Instruction::new(Opcode::ADD, 1, 5, 3, false, true)];

    Program::new(instructions, pc_start, pc_base)
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;
    let program_cols = (8..35).collect::<Vec<_>>();

    // ######################## Solver Parameters ###############################
    let max_iteration = 100000;
    let min_row_id = 0;
    let max_row_id = 0;
    let num_extracted_rows = 1;
    let seed = 41;

    // ######################## Extract CPU Constraints ##########################
    let air = CpuChip::default();
    let air_name = "Cpu";
    println!("{:?}", CPU_COL_MAP);
    //let colmap = make_col_map();
    //println!("operand_1: {:?}", colmap.operand_1);
    //println!("operand_2: {:?}", colmap.operand_2);

    let (tv_constraints, mut refinable_cols, range_types, general_lookup_info) =
        extract_constraints_and_range::<KoalaBear, CpuChip>(&air, NUM_CPU_COLS, prime);
    refinable_cols.retain(|x| !program_cols.contains(x));

    let (pv_pos_constraints, pv_neg_constraints) = get_pv_constraints();

    //refinable_cols.extend(&[2, 3, 4, 5]); // output
    //refinable_cols.extend(&[9, 13]); // input
    //range_types.insert(9, RangeType::U4);
    //range_types.insert(13, RangeType::U4);
    println!("{:?}", refinable_cols);
    println!("{:?}", range_types);

    let constraints = LatticeVMConstraints {
        air_constraints: tv_constraints.clone(),
        pv_pos_constraints,
        pv_neg_constraints,
    };
    let minimum_num_taregt_cols = 1; //refinable_cols.len();

    // ######################## Program Initialization ###########################
    let program = target_program(4, 4);
    let base_abs_main_trace_data =
        generate_abstract_trace(&program, air_name.to_string(), num_extracted_rows);

    // ######################## Public Values ####################################
    let mut public_vals = vec![AbstractInterval::zero(); ZKM_PROOF_NUM_PV_ELTS];
    public_vals[40] = AbstractInterval::i4();
    public_vals[41] = AbstractInterval::bool();
    public_vals[44] = AbstractInterval::one();
    let refinment_target_indicies_pv: Vec<usize> = vec![40, 41];

    // ######################## Solve ############################################
    quick_api(
        get_program_str(&program),
        &constraints,
        &refinable_cols,
        &range_types,
        &refinment_target_indicies_pv,
        &base_abs_main_trace_data,
        public_vals,
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
