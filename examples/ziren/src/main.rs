use std::rc::Rc;

use p3_mersenne_31::Mersenne31;
use p3_uni_stark::{get_symbolic_constraints, SymbolicExpression};
use rand::{rngs::StdRng, SeedableRng};

use zkm_core_executor::{
    syscalls::SyscallCode, ExecutionState, Executor, Instruction, Opcode, Program,
};
use zkm_core_machine::CpuChip;
use zkm_stark::{ZKMCoreOpts, ZKM_PROOF_NUM_PV_ELTS};

use latticevm::symbolic::expr_to_smt_over_trace;
use latticevm::symbolic::LatticeVMSymbolicEntry;
use latticevm::symbolic::LatticeVMSymbolicExpr;
use latticevm::symbolic::LatticeVMSymbolicVal;
use latticevm::{
    interval::AbstractInterval, solver::solve, symbolic::gather_boolean_variables,
    symbolic::AbstractTrace, symbolic::LatticeVMConstraints, utils::BitCombinationsDictOrder,
};

use latticevm_ziren::p3_to_tv::convert_p3_expr;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ZirenAbstractState {
    pub clk: AbstractInterval,
    pub pc: AbstractInterval,
    pub next_pc: AbstractInterval,
}

pub fn ziren_state_to_abstract_state(ziren_state: &ExecutionState) -> ZirenAbstractState {
    ZirenAbstractState {
        clk: AbstractInterval::from_i64(ziren_state.clk as i64),
        pc: AbstractInterval::from_i64(ziren_state.pc as i64),
        next_pc: AbstractInterval::from_i64(ziren_state.next_pc as i64),
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
    }
}

pub fn add_program(pc_start: u32, pc_base: u32) -> Program {
    let mut instructions = vec![Instruction::new(Opcode::ADD, 1, 0, 1, false, true)];
    instructions.extend(vec![Instruction::new(Opcode::MUL, 1, 0, 1, false, true)]);
    instructions.extend(vec![
        Instruction::new(Opcode::ADD, 2, 0, SyscallCode::HALT as u32, false, true),
        Instruction::new(Opcode::ADD, 4, 0, 0, false, true),
        Instruction::new(Opcode::SYSCALL, 2, 4, 5, false, false),
    ]);
    Program::new(instructions, pc_start, pc_base)
}

fn main() -> Result<(), ()> {
    let program = add_program(0, 0);
    let mut runtime = Executor::new(program, ZKMCoreOpts::default());
    runtime.run().unwrap();
    let true_abstract_states = runtime
        .state_history
        .iter()
        .map(|s| ziren_state_to_abstract_state(s))
        .collect::<Vec<_>>();
    println!("#history: {}", true_abstract_states.len());

    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1; //2_u32.pow(31) - 1;
    let mut rng = StdRng::seed_from_u64(42);

    let air = CpuChip::default();
    let symbolic_constraints: Vec<SymbolicExpression<Mersenne31>> =
        get_symbolic_constraints(&air, 0, ZKM_PROOF_NUM_PV_ELTS);
    let mut tv_constraints = symbolic_constraints
        .iter()
        .map(|sc| convert_p3_expr::<Mersenne31>(&sc))
        .collect::<Vec<_>>();

    println!("#symbolic_constraints: {}", symbolic_constraints.len());
    for tv in &tv_constraints {
        println!("{} = 0", tv);
    }

    //let smt = expr_to_smt_over_trace(&tv_constraints, 1, 68, prime);
    //println!("{}", smt);
    //println!("====");

    let potential_boolean_vars = gather_boolean_variables(&tv_constraints);
    println!("boolean vars: {:?}", potential_boolean_vars);

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

    let constraints = LatticeVMConstraints {
        air_constraints: tv_constraints,
        pv_pos_constraints,
        pv_neg_constraints,
    };

    let rows = vec![vec![
        1, 0, 0, 0, 0, 0, 4, 8, 0, 29, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 5, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0,
        0, 0, 0, 0, 0, 0, 1, 0,
    ]];
    let mut abs_main_trace_data = rows
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|v| AbstractInterval { lo: v, hi: v })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for i in 0..abs_main_trace_data.len() {
        abs_main_trace_data[i][5] = AbstractInterval::i4();
        abs_main_trace_data[i][6] = AbstractInterval::i4();

        for j in &potential_boolean_vars {
            abs_main_trace_data[i][*j] = AbstractInterval::one();
        }

        //abs_main_trace_data[i][2] = AbstractInterval::i4();
        //abs_main_trace_data[i][1] = AbstractInterval::i4();
        //abs_main_trace_data[i][7] = AbstractInterval::i8();
        //abs_main_trace_data[i][18] = AbstractInterval::bool();
        //abs_main_trace_data[i][19] = AbstractInterval::bool();
        //abs_main_trace_data[i][20] = AbstractInterval::bool();
        //abs_main_trace_data[i][23] = AbstractInterval::bool();
    }

    //let mut abs_main_trace_data =
    //    vec![vec![AbstractInterval::zero(); ZKM_PROOF_NUM_PV_ELTS]; num_steps];

    let comb =
        BitCombinationsDictOrder::new(potential_boolean_vars.len() * abs_main_trace_data.len());
    for c in comb {
        let mut tmp = abs_main_trace_data.clone();
        for (i, b) in c.iter().enumerate() {
            tmp[0][potential_boolean_vars[i % potential_boolean_vars.len()]] = if *b == 1 {
                AbstractInterval::one()
            } else {
                AbstractInterval::zero()
            };
        }

        let abs_main_trace = AbstractTrace::new(tmp);

        let mut public_vals = vec![AbstractInterval::zero(); ZKM_PROOF_NUM_PV_ELTS];
        public_vals[40] = AbstractInterval::i4();
        public_vals[41] = AbstractInterval::i4();
        public_vals[44] = AbstractInterval::one();

        let refinment_target_indicies_main: Vec<usize> = vec![5, 6];
        let refinment_target_indicies_pv: Vec<usize> = vec![40, 41];

        let result = solve(
            abs_main_trace,
            public_vals,
            &constraints,
            1,
            &refinment_target_indicies_main,
            &refinment_target_indicies_pv,
            prime,
            &mut rng,
        );

        if let Some(trace) = result {
            println!("\nFind SAT assignment: {}", trace);
            break;
        } else {
            println!("\nCouln't Find SAT assignment");
        }
    }

    Ok(())
}
