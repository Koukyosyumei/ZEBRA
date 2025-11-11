use std::rc::Rc;
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

use p3_koala_bear::KoalaBear;
use p3_matrix::dense::RowMajorMatrix;
use p3_mersenne_31::Mersenne31;
use p3_uni_stark::{get_symbolic_constraints, SymbolicExpression};

use zkm_core_executor::ExecutionRecord;
use zkm_core_executor::Executor;
use zkm_core_executor::MipsAirId::MemoryLocal;
use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::memory::MemoryLocalChip;
use zkm_core_machine::AddSubChip;
use zkm_core_machine::{
    cpu::columns::{CPU_COL_MAP, NUM_CPU_COLS},
    CpuChip,
};
use zkm_stark::MachineAir;
use zkm_stark::MachineProver;
use zkm_stark::ZKMCoreOpts;
use zkm_stark::ZKM_PROOF_NUM_PV_ELTS;

use latticevm::smt::expr_to_smt;
use latticevm::symbolic::eval_constraints;
use latticevm::symbolic::LatticeVMSymbolicEntry;
use latticevm::symbolic::LatticeVMSymbolicExpr;
use latticevm::symbolic::LatticeVMSymbolicVal;
use latticevm::ui::UiState;
use latticevm::{
    interval::AbstractInterval, solver::run_solver, symbolic::gather_boolean_variables,
    symbolic::AbstractTrace, symbolic::LatticeVMConstraints,
};
use latticevm_ziren::executor::run_ziren_program;
use latticevm_ziren::p3_to_tv::convert_p3_expr;
use latticevm_ziren::pv_constraints::get_pv_constraints;
use latticevm_ziren::state::ziren_abstract_trace_to_abstract_state;
use latticevm_ziren::table_generator::emit_events;

pub fn add_program(pc_start: u32, pc_base: u32) -> Program {
    let instructions = vec![Instruction::new(Opcode::ADD, 1, 5, 3, false, true)];
    Program::new(instructions, pc_start, pc_base)
}

fn main() -> Result<(), io::Error> {
    // ############### Global Parameters #################################
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1;
    let program_cols = (8..35).collect::<Vec<_>>();

    // ############### Gather Constraints ################################
    let air = CpuChip::default();

    let symbolic_constraints: Vec<SymbolicExpression<KoalaBear>> =
        get_symbolic_constraints(&air, 0, ZKM_PROOF_NUM_PV_ELTS);
    let tv_constraints = symbolic_constraints
        .iter()
        .map(|sc| convert_p3_expr::<KoalaBear>(&sc))
        .collect::<Vec<_>>();
    println!("{:?}", CPU_COL_MAP);

    for tv in &tv_constraints {
        println!("{}", tv);
    }

    // # Additional Public Value Verification
    let (pv_pos_constraints, pv_neg_constraints) = get_pv_constraints();

    // # Gather Symbolic Constraints
    let constraints = LatticeVMConstraints {
        air_constraints: tv_constraints.clone(),
        pv_pos_constraints,
        pv_neg_constraints,
    };

    // ############### Preparation of Solver ############################
    let mut rng = StdRng::seed_from_u64(42);
    let max_row_id = 0;
    let num_extracted_rows = 2;
    let potential_boolean_vars = gather_boolean_variables(&tv_constraints);

    // ############### Prepare Public Values ############################
    let mut public_vals = vec![AbstractInterval::zero(); ZKM_PROOF_NUM_PV_ELTS];
    public_vals[40] = AbstractInterval::i4();
    public_vals[41] = AbstractInterval::bool();
    public_vals[44] = AbstractInterval::one();
    let refinment_target_indicies_pv: Vec<usize> = vec![40, 41];

    // ############### Target Program ###################################
    let program = add_program(4, 4);
    let (true_abstract_states, true_abstract_traces) = run_ziren_program(&program);

    let mut base_abs_main_trace_data = vec![];
    for st in &true_abstract_traces {
        if st.0 == "Cpu" {
            base_abs_main_trace_data = st.1[..num_extracted_rows].to_vec();
        }
    }

    /*
    let mut runtime = Executor::new(program.clone(), ZKMCoreOpts::default());
    let mut u32_cpu_row: Vec<u32> = base_abs_main_trace_data[0]
        .clone()
        .into_iter()
        .map(|v| v.as_canonical_u32(prime))
        .collect();
    emit_events(&mut runtime, &u32_cpu_row);

    let chip = AddSubChip::default();
    let mut trace: RowMajorMatrix<KoalaBear> =
        chip.generate_trace(&runtime.record, &mut ExecutionRecord::default());
    println!("AddSub: {:?}", trace.row_mut(0));

    let chip = MemoryLocalChip::new();
    let mut trace: RowMajorMatrix<KoalaBear> =
        chip.generate_trace(&runtime.record, &mut ExecutionRecord::default());
    println!("memLocal: {:?}", trace.row_mut(0));
    */

    // ############### Construct SMT formula ############################
    if false {
        let mut positions = vec![];
        for i in 0..base_abs_main_trace_data.len() {
            for j in 0..base_abs_main_trace_data[0].len() {
                positions.push((i, j, base_abs_main_trace_data[i][j].clone()));
            }
        }
        let smt = expr_to_smt(
            &constraints,
            &positions,
            num_extracted_rows,
            68,
            ZKM_PROOF_NUM_PV_ELTS,
            prime,
        );
        println!("{}", smt);
    }

    // ############## Final Check Function ##############################
    fn final_check(
        trace: &AbstractTrace,
        prime: u32,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) {
        let recovered_states = trace
            .data
            .iter()
            .map(|row| ziren_abstract_trace_to_abstract_state(row, prime))
            .collect::<Vec<_>>();
        //for rs in &recovered_states {
        //    println!("{}", rs);
        //}
        //println!("========");

        let program = add_program(
            recovered_states[0].pc.as_canonical_u32(prime),
            recovered_states[0].pc.as_canonical_u32(prime),
        );
        let (true_abstract_states, true_abstract_traces) = run_ziren_program(&program);

        //for tas in &true_abstract_states {
        //    println!("{}", tas);
        //}
    }

    // ############## Solve! ###########################################
    let mut target_cols = (0..NUM_CPU_COLS).collect::<Vec<_>>();
    target_cols.retain(|x| !program_cols.contains(x));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut ui = UiState::new();

    run_solver(
        &constraints,
        &target_cols,
        &potential_boolean_vars,
        &refinment_target_indicies_pv,
        &base_abs_main_trace_data,
        public_vals,
        max_row_id,
        final_check,
        prime,
        42,
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

    Ok(())
}
