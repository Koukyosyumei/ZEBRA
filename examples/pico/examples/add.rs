use core::mem::transmute;
use itertools::Itertools;
use std::collections::HashSet;
use std::fs;
use std::io;

use p3_koala_bear::KoalaBear;

use pico_vm::chips::chips::alu::add_sub::columns::{AddSubCols, NUM_ADD_SUB_COLS};
use pico_vm::chips::chips::alu::add_sub::AddSubChip;
use pico_vm::compiler::riscv::program::Program;
use pico_vm::compiler::riscv::{instruction::Instruction, opcode::Opcode, register::Register};

use latticevm::quick::quick_api;
use latticevm::solver::RangeType;
use latticevm::ui::UiState;
use latticevm::utils::create_or_clear_dir;
use latticevm::{symbolic::AbstractTrace, symbolic::LatticeVMConstraints};

use latticevm_pico::lookup::get_symbolic_lookup_constraints;

pub const fn indices_arr<const N: usize>() -> [usize; N] {
    let mut indices_arr = [0; N];
    let mut i = 0;
    while i < N {
        indices_arr[i] = i;
        i += 1;
    }
    indices_arr
}

// ############## Final Check Function ##############################
fn final_check(
    trace: &AbstractTrace,
    num_trial: usize,
    prime: u32,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    let string_representation = format!(
        "input0: [{}, {}, {}, {}], input1: [{}, {}, {}, {}], output: [{}, {}, {}, {}]",
        trace.data[0][9],
        trace.data[0][10],
        trace.data[0][11],
        trace.data[0][12],
        trace.data[0][13],
        trace.data[0][14],
        trace.data[0][15],
        trace.data[0][16],
        trace.data[0][2],
        trace.data[0][3],
        trace.data[0][4],
        trace.data[0][5],
    );

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

const fn make_col_map() -> AddSubCols<usize> {
    let indices_arr = indices_arr::<{ NUM_ADD_SUB_COLS }>();
    unsafe { transmute::<[usize; NUM_ADD_SUB_COLS], AddSubCols<usize>>(indices_arr) }
}

pub fn target_program(pc_start: u32, pc_base: u32) -> Program {
    let instructions = vec![Instruction::new(Opcode::ADD, 1, 2, 3, true, true)];
    Program::new(instructions, pc_start, pc_base)
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;

    // ######################## Solver Parameters ###############################
    let max_iteration = 100000000;
    let min_row_id = 0;
    let max_row_id = 0;
    let num_extracted_rows = 1;
    let seed = 41;
    //let aux_tg_fns: Vec<_> = vec![dummy_table_deriver];

    // ######################## Extract CPU Constraints ##########################
    let air: AddSubChip<KoalaBear> = AddSubChip::default();
    let air_name = "AddSub";
    let colmap = make_col_map();
    println!("output: {:?}", colmap.values[0].add_operation.value);
    println!("operand_1: {:?}", colmap.values[0].operand_1);
    println!("operand_2: {:?}", colmap.values[0].operand_2);

    let mut u8_cols = vec![];
    let mut multiplicities = HashSet::new();
    let mut lookup_constraints = vec![];
    let mut received_vars_from_cpu = HashSet::new();
    get_symbolic_lookup_constraints(
        &air,
        0,
        0,
        &mut u8_cols,
        &mut multiplicities,
        &mut lookup_constraints,
        &mut received_vars_from_cpu,
        prime,
    );

    Ok(())
}
