use latticevm::interval::AbstractInterval;
use latticevm::symbolic::LatticeVMSymbolicEntry;
use latticevm::symbolic::LatticeVMSymbolicExpr;
use latticevm::symbolic::LatticeVMSymbolicVal;

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
