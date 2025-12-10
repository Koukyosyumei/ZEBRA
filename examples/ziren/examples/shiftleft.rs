use core::mem::{size_of, transmute};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::rc::Rc;
use std::time;
use std::{io, thread, time::Duration};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use itertools::Itertools;
use rand::{rngs::StdRng, SeedableRng};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

use p3_air::Air;
use p3_air::PairCol;
use p3_field::Field;
use p3_koala_bear::KoalaBear;
use p3_matrix::dense::RowMajorMatrix;
use p3_mersenne_31::Mersenne31;
use p3_uni_stark::SymbolicAirBuilder;
use p3_uni_stark::{get_symbolic_constraints, SymbolicExpression};

use zkm_core_executor::syscalls::SyscallCode;
use zkm_core_executor::ExecutionRecord;
use zkm_core_executor::Executor;
use zkm_core_executor::MipsAirId::MemoryLocal;
use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::alu::LtCols;
use zkm_core_machine::alu::ShiftLeftCols;
use zkm_core_machine::alu::NUM_ADD_SUB_COLS;
use zkm_core_machine::alu::NUM_LT_COLS;
use zkm_core_machine::alu::NUM_SHIFT_LEFT_COLS;
use zkm_core_machine::memory::MemoryLocalChip;
use zkm_core_machine::AddSubChip;
use zkm_core_machine::LtChip;
use zkm_core_machine::MulChip;
use zkm_core_machine::ShiftLeft;
use zkm_core_machine::{
    cpu::columns::{CPU_COL_MAP, NUM_CPU_COLS},
    CpuChip,
};
use zkm_stark::LookupBuilder;
use zkm_stark::LookupKind;
use zkm_stark::MachineAir;
use zkm_stark::MachineProver;
use zkm_stark::ZKMCoreOpts;
use zkm_stark::ZKM_PROOF_NUM_PV_ELTS;

use latticevm::smt::expr_to_smt;
use latticevm::solver::RangeType;
use latticevm::symbolic::eval_constraints;
use latticevm::symbolic::LatticeVMSymbolicEntry;
use latticevm::symbolic::LatticeVMSymbolicExpr;
use latticevm::symbolic::LatticeVMSymbolicVal;
use latticevm::ui::UiState;
use latticevm::utils::create_or_clear_dir;
use latticevm::{
    interval::AbstractInterval, solver::run_solver, symbolic::gather_boolean_variables,
    symbolic::AbstractTrace, symbolic::LatticeVMConstraints,
};

use latticevm_ziren::executor::run_ziren_program;
use latticevm_ziren::p3_to_tv::{convert_p3_expr, convert_p3_virtual_pair_col};
use latticevm_ziren::pv_constraints::get_pv_constraints;
use latticevm_ziren::state::ziren_abstract_trace_to_abstract_state;

fn program_counter_refine_fn(
    abs_main_trace_data: &mut Vec<Vec<AbstractInterval>>,
    program_len: usize,
    i: usize,
    j: usize,
) {
}

fn adjust_pc_program(main_trace: &mut AbstractTrace, prime: u32) {}

pub fn dummy_table_deriver(
    cpu_main_trace: &Vec<Vec<AbstractInterval>>,
    range_types: &HashMap<usize, RangeType>,
    prime: u32,
) -> Vec<Vec<AbstractInterval>> {
    let out = vec![];
    out
}

pub fn get_symbolic_constraints_look<F, A>(
    air: &A,
    preprocessed_width: usize,
    num_public_values: usize,
    u8_cols: &mut Vec<usize>,
    multiplicities: &mut HashSet<usize>,
    lookup_constraints: &mut Vec<LatticeVMSymbolicExpr>,
    received_vars_from_cpu: &mut HashSet<usize>,
) where
    F: p3_field::PrimeField32,
    A: Air<LookupBuilder<F>>,
{
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
    for s in &sends {
        for (w, _) in &s.multiplicity.column_weights {
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
                let shard = &r.values[0];
                let clk = &r.values[1];
                let pc = &r.values[2];
                let next_pc = &r.values[3];
                let next_next_pc = &r.values[4];
                let num_extra_cycles = &r.values[5];
                let opcode = &r.values[6];
                let a0 = &r.values[7];
                let a1 = &r.values[8];
                let a2 = &r.values[9];
                let a3 = &r.values[10];
                let b0 = &r.values[11];
                let b1 = &r.values[12];
                let b2 = &r.values[13];
                let b3 = &r.values[14];
                let c0 = &r.values[15];
                let c1 = &r.values[16];
                let c2 = &r.values[17];
                let c3 = &r.values[18];

                let hi0 = &r.values[19];
                let hi1 = &r.values[20];
                let hi2 = &r.values[21];
                let hi3 = &r.values[22];
                let op_a_immutable = &r.values[24];
                let is_rw_a = &r.values[25];
                let is_check_memory = &r.values[26];
                let is_halt = &r.values[27];
                let is_sequential = &r.values[28];

                /*
                println!("shard: {:?}", shard);
                println!("clk: {:?}", clk);
                println!("pc: {:?}", pc);
                println!("next_pc: {:?}", next_pc);
                println!("next_next_pc: {:?}", next_next_pc);
                println!("num_extra_cycles: {:?}", num_extra_cycles);
                println!("opcode: {:?}", opcode);
                println!("a0: {:?}", a0);
                println!("a1: {:?}", a1);
                println!("a2: {:?}", a2);
                println!("a3: {:?}", a3);
                println!("b0: {:?}", b0);
                println!("b1: {:?}", b1);
                println!("b2: {:?}", b2);
                println!("b3: {:?}", b3);
                println!("c0: {:?}", c0);
                println!("c1: {:?}", c1);
                println!("c2: {:?}", c2);
                println!("c3: {:?}", c3);
                println!("hi0: {:?}", hi0);
                println!("hi1: {:?}", hi1);
                println!("hi2: {:?}", hi2);
                println!("hi3: {:?}", hi3);
                println!("op_a_immutable: {:?}", op_a_immutable);
                println!("is_rw_a: {:?}", is_rw_a);
                println!("is_check_memory: {:?}", is_check_memory);
                println!("is_halt: {:?}", is_halt);
                println!("is_sequential: {:?}", is_sequential);
                */
            }
            _ => {}
        }
    }

    for s in &sends {
        match s.kind {
            LookupKind::Byte => {
                let opcode = &s.values[0];
                let a1 = &s.values[1];
                let a2 = &s.values[2];
                let b = &s.values[3];
                let c = &s.values[4];
                if opcode.constant == F::from_canonical_u64(4) {
                    // RangeU8
                    if let p3_air::PairCol::Main(col_idx) = b.column_weights[0].0 {
                        u8_cols.push(col_idx);
                    }
                    if let p3_air::PairCol::Main(col_idx) = c.column_weights[0].0 {
                        u8_cols.push(col_idx);
                    }
                } else if opcode.constant == F::from_canonical_u64(0) {
                    // AND
                    println!("b: {:?}", b);
                    let constraint = LatticeVMSymbolicExpr::Sub(
                        Box::new(convert_p3_virtual_pair_col(a1)),
                        Box::new(LatticeVMSymbolicExpr::And(
                            Box::new(convert_p3_virtual_pair_col(b)),
                            Box::new(convert_p3_virtual_pair_col(c)),
                        )),
                    );
                    lookup_constraints.push(constraint.clone());

                    println!("constraint: {}", constraint);
                } else if opcode.constant == F::from_canonical_u64(6) {
                    // LT
                    let constraint = LatticeVMSymbolicExpr::Sub(
                        Box::new(convert_p3_virtual_pair_col(a1)),
                        Box::new(LatticeVMSymbolicExpr::Lt(
                            Box::new(convert_p3_virtual_pair_col(b)),
                            Box::new(convert_p3_virtual_pair_col(c)),
                        )),
                    );
                    lookup_constraints.push(constraint.clone());

                    println!("constraint: {}", constraint);
                }
            }
            _ => {}
        }
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
    let string_representation = format!(
        "input0: [{}, {}, {}, {}], input1: [{}, {}, {}, {}], output: [{}, {}, {}, {}]",
        trace.data[0][2],
        trace.data[0][3],
        trace.data[0][4],
        trace.data[0][5],
        trace.data[0][6],
        trace.data[0][7],
        trace.data[0][8],
        trace.data[0][9],
        trace.data[0][10],
        trace.data[0][11],
        trace.data[0][12],
        trace.data[0][13],
    );

    // 2130706432

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

pub const fn indices_arr<const N: usize>() -> [usize; N] {
    let mut indices_arr = [0; N];
    let mut i = 0;
    while i < N {
        indices_arr[i] = i;
        i += 1;
    }
    indices_arr
}

const fn make_col_map() -> ShiftLeftCols<usize> {
    let indices_arr = indices_arr::<{ NUM_SHIFT_LEFT_COLS }>();
    unsafe { transmute::<[usize; NUM_SHIFT_LEFT_COLS], ShiftLeftCols<usize>>(indices_arr) }
}

pub fn add_program(pc_start: u32, pc_base: u32) -> Program {
    let mut instructions = vec![Instruction::new(Opcode::SLL, 1, 2, 3, true, true)];
    /*
    instructions.extend(vec![
        Instruction::new(Opcode::ADD, 2, 0, SyscallCode::HALT as u32, false, true),
        Instruction::new(Opcode::ADD, 4, 0, 0, false, true),
        Instruction::new(Opcode::SYSCALL, 2, 4, 5, false, false),
    ]);*/

    Program::new(instructions, pc_start, pc_base)
}

fn main() -> Result<(), io::Error> {
    create_or_clear_dir("voutput")?;

    // ######################## Prime and Column Settings ########################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;
    // Columns reserved for program counters / instructions

    // ######################## Extract CPU Constraints ##########################
    let air = ShiftLeft::default();
    let colmap = make_col_map();
    println!("a: {:?}", colmap.a);
    println!("b: {:?}", colmap.b);
    println!("c: {:?}", colmap.c);
    //println!("{:?}", colmap.byte_equality_check);
    println!("{:?}", NUM_SHIFT_LEFT_COLS);

    let mut u8_cols = vec![];
    let mut multiplicities = HashSet::new();
    let mut lookup_symbolic_constraints = Vec::new();
    let mut received_vars_from_cpu = HashSet::new();
    let symbolic_constraints: Vec<SymbolicExpression<KoalaBear>> =
        get_symbolic_constraints(&air, 0, ZKM_PROOF_NUM_PV_ELTS);
    get_symbolic_constraints_look::<KoalaBear, ShiftLeft>(
        &air,
        0,
        ZKM_PROOF_NUM_PV_ELTS,
        &mut u8_cols,
        &mut multiplicities,
        &mut lookup_symbolic_constraints,
        &mut received_vars_from_cpu,
    );

    let mut refinable_cols: Vec<usize> = (0..NUM_SHIFT_LEFT_COLS).collect();
    refinable_cols.retain(|c| !multiplicities.contains(c));
    refinable_cols.retain(|c| !received_vars_from_cpu.contains(c));
    refinable_cols.extend(&[2, 3, 4, 5]);
    println!("------------------: {:?}", u8_cols);
    println!("------------------: {:?}", multiplicities);
    println!("------------------: {:?}", received_vars_from_cpu);

    //let tmp: Vec<usize> = vec![22];
    //refinable_cols.retain(|c| !tmp.contains(c));

    let mut tv_constraints = symbolic_constraints
        .iter()
        .map(|sc| convert_p3_expr::<KoalaBear>(&sc))
        .collect::<Vec<_>>();
    println!("aaaaaaaaaaaaaaaaaaaa{:?}", lookup_symbolic_constraints);
    tv_constraints.extend(lookup_symbolic_constraints);
    for s in &tv_constraints {
        println!("{}", s);
    }
    let potential_boolean_vars = gather_boolean_variables(&tv_constraints, &multiplicities);
    println!("########################: {:?}", potential_boolean_vars);

    let mut range_types: HashMap<usize, RangeType> = potential_boolean_vars
        .iter()
        .map(|k| (*k, RangeType::Bool))
        .collect();
    //range_types.clear();

    //println!("{:?}", CPU_COL_MAP);

    // # Additional Public Value Verification
    //let (pv_pos_constraints, pv_neg_constraints) = get_pv_constraints();
    let pv_pos_constraints = vec![];
    let pv_neg_constraints = vec![];

    // # Gather Symbolic Constraints
    let constraints = LatticeVMConstraints {
        air_constraints: tv_constraints.clone(),
        pv_pos_constraints,
        pv_neg_constraints,
    };

    // Columns available for refinement (excluding reserved program columns)
    //let mut target_cols = (0..NUM_LT_COLS).collect::<Vec<_>>();
    //target_cols = vec![2, 3, 4, 5, 6, 7, 8, 9, 13];
    for c in &u8_cols {
        range_types.insert(*c, RangeType::U8);
    }
    for c in &potential_boolean_vars {
        range_types.insert(*c, RangeType::Bool);
    }
    //range_types.insert(30, RangeType::U8);
    //range_types.insert(31, RangeType::U8);

    //range_types.insert(9, RangeType::U4);
    //range_types.insert(13, RangeType::U4);

    println!("{:?}", refinable_cols);
    println!("{:?}", range_types);

    // ######################## Auxiliary ALU Constraints #######################
    //let alu_constraints = get_alu_constraints();
    let aux_objs: Vec<_> = vec![];
    let aux_tg_fns: Vec<_> = vec![dummy_table_deriver];

    // ######################## Solver Parameters ###############################
    let max_iteration = 100000000;
    let minimum_num_taregt_cols = refinable_cols.len();
    let min_row_id = 0;
    let max_row_id = 0;
    let num_extracted_rows = 1;
    let seed = 41;

    // Public trace values (example: program start, memory base, initial step)
    let mut public_vals = vec![AbstractInterval::zero(); ZKM_PROOF_NUM_PV_ELTS];
    public_vals[40] = AbstractInterval::i4();
    public_vals[41] = AbstractInterval::bool();
    public_vals[44] = AbstractInterval::one();
    let refinment_target_indicies_pv: Vec<usize> = vec![];

    // ######################## Program Initialization ###########################
    let program = add_program(4, 4);
    let program_len = program.instructions.len();

    // Convert program to string for UI display
    let program_str = program
        .instructions
        .iter()
        .map(|inst| format!("{:?}\n", inst))
        .collect::<String>();

    let (true_abstract_states, true_abstract_traces) = run_ziren_program(&program);
    let mut base_abs_main_trace_data = vec![];
    for st in &true_abstract_traces {
        println!("{}", st.0);
        if st.0 == "ShiftLeft" {
            base_abs_main_trace_data = st.1[..num_extracted_rows].to_vec();
        }
    }
    println!("{:?}", base_abs_main_trace_data);

    // ######################## UI Initialization ################################

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut ui = UiState::new();
    ui.program = program_str;

    // ######################## Run Solver ######################################
    let mut known_solution = HashSet::<String>::new();
    let mut logs = Vec::new();
    let start_time = time::Instant::now();
    run_solver(
        &constraints,
        &refinable_cols,
        &range_types,
        &aux_objs,
        &aux_tg_fns,
        &refinment_target_indicies_pv,
        &base_abs_main_trace_data,
        public_vals,
        max_iteration,
        minimum_num_taregt_cols,
        min_row_id,
        max_row_id,
        program_len,
        program_counter_refine_fn,
        adjust_pc_program,
        final_check,
        prime,
        seed,
        &mut known_solution,
        &mut logs,
        &mut ui,
        &mut terminal,
    );

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    eprintln!("Execution Time    : {:?}", start_time.elapsed());
    eprintln!("#Unique Solution  : {}", known_solution.len());

    Ok(())
}

/*
(curr[25] - (curr[23] * curr[2]))
(curr[26] - (curr[24] * curr[2]))
(curr[23] - ((curr[11] - curr[20]) * 2114060289))
(curr[24] - ((curr[15] - curr[21]) * 2114060289))
(curr[29] * (curr[29] - 1))
(curr[29] * (curr[25] - curr[26]))
((curr[2] + curr[3]) * ((curr[29] - 1) * ((curr[25] + curr[26]) - 1)))
(curr[4] - ((curr[25] * (1 - curr[26])) + (curr[29] * curr[27])))
curr[5]
curr[6]
curr[7]
(curr[16] * (curr[16] - 1))
(curr[17] * (curr[17] - 1))
(curr[18] * (curr[18] - 1))
(curr[19] * (curr[19] - 1))
((((curr[16] + curr[17]) + curr[18]) + curr[19]) * ((((curr[16] + curr[17]) + curr[18]) + curr[19]) - 1))
((curr[2] + curr[3]) * ((1 - curr[28]) - (((curr[16] + curr[17]) + curr[18]) + curr[19])))
(curr[28] * (curr[28] - 1))
(((0 + curr[19]) - 1) * (((curr[11] * curr[3]) + (curr[20] * curr[2])) - ((curr[15] * curr[3]) + (curr[21] * curr[2]))))
(curr[28] * (0 + curr[19]))
((((0 + curr[19]) + curr[18]) - 1) * (curr[10] - curr[14]))
(curr[28] * ((0 + curr[19]) + curr[18]))
(((((0 + curr[19]) + curr[18]) + curr[17]) - 1) * (curr[9] - curr[13]))
(curr[28] * (((0 + curr[19]) + curr[18]) + curr[17]))
((((((0 + curr[19]) + curr[18]) + curr[17]) + curr[16]) - 1) * (curr[8] - curr[12]))
(curr[28] * ((((0 + curr[19]) + curr[18]) + curr[17]) + curr[16]))
(curr[30] - ((((0 + (((curr[11] * curr[3]) + (curr[20] * curr[2])) * curr[19])) + (curr[10] * curr[18])) + (curr[9] * curr[17])) + (curr[8] * curr[16])))
(curr[31] - ((((0 + (((curr[15] * curr[3]) + (curr[21] * curr[2])) * curr[19])) + (curr[14] * curr[18])) + (curr[13] * curr[17])) + (curr[12] * curr[16])))
((curr[28] - 1) * ((curr[22] * (curr[30] - curr[31])) - (curr[2] + curr[3])))
(curr[2] * (curr[2] - 1))
(curr[3] * (curr[3] - 1))
((curr[2] + curr[3]) * ((curr[2] + curr[3]) - 1))
*/
