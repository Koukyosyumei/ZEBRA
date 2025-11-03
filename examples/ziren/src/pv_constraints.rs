use std::rc::Rc;

use itertools::Itertools;
use rand::{rngs::StdRng, SeedableRng};

use p3_mersenne_31::Mersenne31;
use p3_uni_stark::{get_symbolic_constraints, SymbolicExpression};

use zkm_core_executor::{Instruction, Opcode, Program};
use zkm_core_machine::CpuChip;
use zkm_stark::MachineProver;
use zkm_stark::ZKM_PROOF_NUM_PV_ELTS;

use latticevm::symbolic::LatticeVMSymbolicEntry;
use latticevm::symbolic::LatticeVMSymbolicExpr;
use latticevm::symbolic::LatticeVMSymbolicVal;
use latticevm::{
    interval::AbstractInterval, solver::solve, symbolic::gather_boolean_variables,
    symbolic::AbstractTrace, symbolic::LatticeVMConstraints,
};

pub fn get_pv_constraints() -> (Vec<LatticeVMSymbolicExpr>, Vec<LatticeVMSymbolicExpr>) {
    let pv_pos_constraints = vec![LatticeVMSymbolicExpr::Sub(
        Box::new(LatticeVMSymbolicExpr::Variable(LatticeVMSymbolicVal {
            entry: LatticeVMSymbolicEntry::Public,
            index: 41,
        })),
        Box::new(LatticeVMSymbolicExpr::Constant(AbstractInterval {
            lo: 0,
            hi: 0,
        })),
    )];
    let pv_neg_constraints = vec![LatticeVMSymbolicExpr::Sub(
        Box::new(LatticeVMSymbolicExpr::Variable(LatticeVMSymbolicVal {
            entry: LatticeVMSymbolicEntry::Public,
            index: 40,
        })),
        Box::new(LatticeVMSymbolicExpr::Constant(AbstractInterval {
            lo: 0,
            hi: 0,
        })),
    )];

    (pv_pos_constraints, pv_neg_constraints)
}
