use zebra::interval::AbstractInterval;
use zebra::symbolic::ZEBRASymbolicEntry;
use zebra::symbolic::ZEBRASymbolicExpr;
use zebra::symbolic::ZEBRASymbolicVal;

/// Represents the position of a shard in the sphinx proof, mirroring the actual
/// `SphinxProver::verify()` loop logic in `prover/src/verify.rs`.
///
/// The sphinx verifier has a well-known bug: for single-shard proofs the `else`
/// branch (which contains the halt check) is never reached.
pub enum ShardPosition {
    /// i == 0: checks shard==1 and start_pc==pc_start; NO halt check (the bug).
    First { pc_start: u32 },
    /// i > 0 and not the last shard: no halt-related constraint in sphinx's else branch.
    Middle,
    /// i > 0 and i == len-1: sphinx requires next_pc == 0.
    Last,
}

fn pv_var(index: usize) -> ZEBRASymbolicExpr {
    ZEBRASymbolicExpr::Variable(ZEBRASymbolicVal {
        entry: ZEBRASymbolicEntry::Public,
        index,
    })
}

fn const_expr(value: u32) -> ZEBRASymbolicExpr {
    ZEBRASymbolicExpr::Constant(AbstractInterval {
        lo: value as i128,
        hi: value as i128,
    })
}

/// Returns `(pv_pos_constraints, pv_neg_constraints)` that faithfully model the
/// sphinx outer-verifier public-value checks for the given shard position.
///
/// PV layout (sphinx):
///   pv[40] = start_pc
///   pv[41] = next_pc
///   pv[42] = exit_code
///   pv[43] = shard
///
/// pos constraints: expressions that must evaluate to zero  (== 0)
/// neg constraints: expressions that must evaluate non-zero (!= 0)
pub fn get_pv_constraints(
    position: ShardPosition,
) -> (Vec<ZEBRASymbolicExpr>, Vec<ZEBRASymbolicExpr>) {
    match position {
        // i == 0 branch in sphinx verify():
        //   check shard == 1
        //   check start_pc == vk.pc_start
        //   (no halt check — this is the bug for single-shard proofs)
        ShardPosition::First { pc_start } => {
            let pv_pos_constraints = vec![
                // pv[43] - 1 == 0  →  shard == 1
                ZEBRASymbolicExpr::Sub(Box::new(pv_var(43)), Box::new(const_expr(1))),
                // pv[40] - pc_start == 0  →  start_pc == vk.pc_start
                ZEBRASymbolicExpr::Sub(Box::new(pv_var(40)), Box::new(const_expr(pc_start))),
            ];
            let pv_neg_constraints = vec![];
            (pv_pos_constraints, pv_neg_constraints)
        }

        // else branch, not last shard: sphinx performs no halt check here.
        ShardPosition::Middle => {
            let pv_pos_constraints = vec![];
            let pv_neg_constraints = vec![];
            (pv_pos_constraints, pv_neg_constraints)
        }

        // else branch, last shard: sphinx checks next_pc == 0 (halt required).
        ShardPosition::Last => {
            let pv_pos_constraints = vec![
                // pv[41] - 0 == 0  →  next_pc == 0
                ZEBRASymbolicExpr::Sub(Box::new(pv_var(41)), Box::new(const_expr(0))),
            ];
            let pv_neg_constraints = vec![];
            (pv_pos_constraints, pv_neg_constraints)
        }
    }
}
