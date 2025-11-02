use std::collections::HashMap;
use std::io;
use std::rc::Rc;

use itertools::Itertools;
use rand::{rngs::StdRng, SeedableRng};

use p3_air::BaseAir;
use p3_mersenne_31::Mersenne31;
use p3_uni_stark::{get_symbolic_constraints, SymbolicExpression};

use zkm_core_executor::{
    syscalls::SyscallCode, ExecutionState, Executor, Instruction, Opcode, Program,
};
use zkm_core_machine::mips::MipsAir;
use zkm_core_machine::utils::trace_checkpoint;
use zkm_core_machine::utils::ZKMCoreProverError;
use zkm_core_machine::CpuChip;
use zkm_stark::{koala_bear_poseidon2::KoalaBearPoseidon2, StarkGenericConfig};
use zkm_stark::{CpuProver, MachineProver};
use zkm_stark::{ZKMCoreOpts, ZKM_PROOF_NUM_PV_ELTS};

use latticevm::interval::MayBeFlag;
use latticevm::smt::expr_to_smt;
use latticevm::symbolic::preprocess_row;
use latticevm::symbolic::LatticeVMSymbolicEntry;
use latticevm::symbolic::LatticeVMSymbolicExpr;
use latticevm::symbolic::LatticeVMSymbolicVal;
use latticevm::{
    interval::AbstractInterval, solver::solve, symbolic::gather_boolean_variables,
    symbolic::AbstractTrace, symbolic::LatticeVMConstraints, utils::BitCombinationsDictOrder,
};

use latticevm_ziren::p3_to_tv::convert_p3_expr;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ZirenAbstractState {
    pub clk: AbstractInterval,
    pub pc: AbstractInterval,
    pub next_pc: AbstractInterval,
    pub is_done: bool,
    pub memory: HashMap<u32, u32>,
}

pub fn ziren_state_to_abstract_state(ziren_state: &ExecutionState) -> ZirenAbstractState {
    let memory = ziren_state
        .memory
        .clone()
        .into_iter()
        .map(|(addr, record)| (addr, record.value))
        .collect();

    ZirenAbstractState {
        clk: AbstractInterval::from_i64(ziren_state.clk as i64),
        pc: AbstractInterval::from_i64(ziren_state.pc as i64),
        next_pc: AbstractInterval::from_i64(ziren_state.next_pc as i64),
        is_done: ziren_state.pc == 0, // TODO || ziren_state.exited,
        memory: memory,
    }
}

pub fn abstract_trace_to_abstract_state(
    abstract_row: &Vec<AbstractInterval>,
) -> ZirenAbstractState {
    ZirenAbstractState {
        clk: abstract_row[1].clone()
            + abstract_row[2].clone() * AbstractInterval::from_i64(2_usize.pow(16) as i64),
        pc: abstract_row[5].clone(),
        next_pc: abstract_row[6].clone(),
        is_done: abstract_row[5].clone().is_zero(1) == MayBeFlag::True,
        memory: HashMap::new(),
    }
}

pub fn add_program(pc_start: u32, pc_base: u32) -> Program {
    let mut instructions = vec![Instruction::new(Opcode::ADD, 1, 0, 1, false, true)];
    //instructions.extend(vec![Instruction::new(Opcode::MUL, 1, 0, 1, false, true)]);
    //instructions.extend(vec![
    //    Instruction::new(Opcode::ADD, 2, 0, SyscallCode::HALT as u32, false, true),
    //    Instruction::new(Opcode::ADD, 4, 0, 0, false, true),
    //    Instruction::new(Opcode::SYSCALL, 2, 4, 5, false, false),
    //]);
    Program::new(instructions, pc_start, pc_base)
}

fn main() -> Result<(), ()> {
    // # Setting
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1; //2_u32.pow(31) - 1;
    let mut rng = StdRng::seed_from_u64(42);

    // # Target Program
    let program = add_program(0, 0);

    // # Execute the Target Program
    let mut runtime = Executor::new(program.clone(), ZKMCoreOpts::default());
    // runtime.run().unwrap();
    let (checkpoint, done) = runtime.execute_state(false).unwrap();
    let mut checkpoint_file = tempfile::tempfile()
        .map_err(ZKMCoreProverError::IoError)
        .unwrap();
    checkpoint
        .save(&mut checkpoint_file)
        .map_err(ZKMCoreProverError::IoError)
        .unwrap();

    let true_abstract_states = runtime
        .state_history
        .iter()
        .map(|s| ziren_state_to_abstract_state(s))
        .collect::<Vec<_>>();
    println!("#history: {}", true_abstract_states.len());

    type SC = KoalaBearPoseidon2;

    // # Gather True Matrices
    let config = KoalaBearPoseidon2::new();
    let machine = MipsAir::machine(config);
    let prover = CpuProver::new(machine);

    let mut reader = io::BufReader::new(checkpoint_file);
    let execution_state: ExecutionState =
        bincode::deserialize_from(&mut reader).expect("failed to deserialize state");
    let (records, report) = trace_checkpoint::<SC>(
        program.clone(),
        execution_state,
        ZKMCoreOpts::default(),
        None,
    );
    let mut main_traces = records
        .iter()
        .map(|record| prover.generate_traces(record))
        .collect::<Vec<_>>();
    println!("#records: {:?}", records.len());
    for mt in &mut main_traces[0] {
        if mt.0 == "Cpu" {
            let nrows = mt.1.values.len() / mt.1.width;
            for i in 0..nrows {
                println!("{:?}", mt.1.row_mut(i));
            }
        }
    }

    // # Construct Cpu Chip
    let air = CpuChip::default();
    let program_cols = (8..20).collect::<Vec<_>>();

    // # Gather Constraints
    let symbolic_constraints: Vec<SymbolicExpression<Mersenne31>> =
        get_symbolic_constraints(&air, 0, ZKM_PROOF_NUM_PV_ELTS);
    let mut tv_constraints = symbolic_constraints
        .iter()
        .map(|sc| convert_p3_expr::<Mersenne31>(&sc))
        .collect::<Vec<_>>();

    //println!("#symbolic_constraints: {}", symbolic_constraints.len());
    //for tv in &tv_constraints {
    //    println!("{} = 0", tv);
    //}

    // # Gather Potential Boolean Variables
    let potential_boolean_vars = gather_boolean_variables(&tv_constraints);
    // println!("boolean vars: {:?}", potential_boolean_vars);

    // # Additional Public Value Verification
    let pv_pos_constraints = vec![LatticeVMSymbolicExpr::Sub(
        Rc::new(LatticeVMSymbolicExpr::Variable(LatticeVMSymbolicVal {
            entry: LatticeVMSymbolicEntry::Public,
            index: 41,
        })),
        Rc::new(LatticeVMSymbolicExpr::Constant(AbstractInterval {
            lo: 0,
            hi: 0,
        })),
    )];
    let pv_neg_constraints = vec![LatticeVMSymbolicExpr::Sub(
        Rc::new(LatticeVMSymbolicExpr::Variable(LatticeVMSymbolicVal {
            entry: LatticeVMSymbolicEntry::Public,
            index: 40,
        })),
        Rc::new(LatticeVMSymbolicExpr::Constant(AbstractInterval {
            lo: 0,
            hi: 0,
        })),
    )];

    // # Gather Symbolic Constraints
    let constraints = LatticeVMConstraints {
        air_constraints: tv_constraints,
        pv_pos_constraints,
        pv_neg_constraints,
    };

    // # Gather Real Trace
    let real_rows = vec![vec![
        1, 0, 0, 0, 0, 0, 4, 8, 0, 29, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 5, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0,
        0, 0, 0, 0, 0, 0, 1, 0,
    ]];
    // code location: 8 - 20

    // # Construct SMT formula
    //let smt = expr_to_smt(&constraints, 1, 68, prime);
    //println!("{}", smt);

    let mut found_solution_flag = false;

    let mut target_cols = (0..68).collect::<Vec<_>>();
    target_cols.retain(|x| !program_cols.contains(x));
    for k in 1..(target_cols.len() + 1) {
        for combo in target_cols.iter().combinations(k) {
            //println!("{:?}", combo);
            let mut abs_main_trace_data = real_rows
                .clone()
                .into_iter()
                .map(|row| {
                    row.into_iter()
                        .map(|v| AbstractInterval { lo: v, hi: v })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();

            for i in 0..abs_main_trace_data.len() {
                for c in &combo {
                    if potential_boolean_vars.contains(c) {
                        abs_main_trace_data[i][**c] = AbstractInterval::bool();
                    } else {
                        abs_main_trace_data[i][**c] = AbstractInterval::i4();
                    }
                }
            }
            let abs_main_trace = AbstractTrace::new(abs_main_trace_data);

            let mut public_vals = vec![AbstractInterval::zero(); ZKM_PROOF_NUM_PV_ELTS];
            public_vals[40] = AbstractInterval::i4();
            public_vals[41] = AbstractInterval::i4();
            public_vals[44] = AbstractInterval::one();

            let refinment_target_indicies_main = combo.clone().into_iter().cloned().collect();
            let refinment_target_indicies_pv: Vec<usize> = vec![40, 41];

            let result = solve(
                abs_main_trace,
                public_vals.clone(),
                &constraints,
                1,
                &refinment_target_indicies_main,
                &refinment_target_indicies_pv,
                prime,
                &mut rng,
                &format!("{:?}", combo),
            );

            if let Some(trace) = result {
                println!("\nFind SAT assignment: {}", trace);
                found_solution_flag = true;
                break;
            } else {
                //println!("\nCouln't Find SAT assignment");
            }
        }

        if found_solution_flag {
            break;
        }
    }

    Ok(())
}
