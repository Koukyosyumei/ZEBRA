use zebra::interval::AbstractInterval;
use zebra::symbolic::ZEBRASymbolicEntry;
use zebra::symbolic::ZEBRASymbolicExpr;
use zebra::symbolic::ZEBRASymbolicVal;

pub fn get_pv_constraints() -> (Vec<ZEBRASymbolicExpr>, Vec<ZEBRASymbolicExpr>) {
    let pv_pos_constraints = vec![ZEBRASymbolicExpr::Sub(
        Box::new(ZEBRASymbolicExpr::Variable(ZEBRASymbolicVal {
            entry: ZEBRASymbolicEntry::Public,
            index: 41,
        })),
        Box::new(ZEBRASymbolicExpr::Constant(AbstractInterval {
            lo: 0,
            hi: 0,
        })),
    )];
    let pv_neg_constraints = vec![ZEBRASymbolicExpr::Sub(
        Box::new(ZEBRASymbolicExpr::Variable(ZEBRASymbolicVal {
            entry: ZEBRASymbolicEntry::Public,
            index: 40,
        })),
        Box::new(ZEBRASymbolicExpr::Constant(AbstractInterval {
            lo: 0,
            hi: 0,
        })),
    )];

    (pv_pos_constraints, pv_neg_constraints)
}
