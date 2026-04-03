#[cfg(test)]
mod tests {
    use openvm_rv32im_transpiler::{BranchEqualOpcode, BranchLessThanOpcode};
    use openvm_stark_sdk::p3_baby_bear::BabyBear;

    use zebra::constraint::eval_constraints;
    use zebra::interval::{AbstractInterval, MayBeFlag};
    use zebra::trace::AbstractTrace;

    use zebra_openvm::utils::{
        extract_branch_eq_constraints, extract_branch_lt_constraints, make_branch_eq_row,
        make_branch_lt_row, BABY_BEAR_PRIME, COL_BREQ_CMP_RESULT, COL_BLT_CMP_RESULT,
    };

    type F = BabyBear;

    // ── BEQ / BNE helpers ─────────────────────────────────────────────────────

    fn check_beq_valid(op: BranchEqualOpcode, a: [u32; 4], b: [u32; 4]) {
        let (ci, _) = extract_branch_eq_constraints(BABY_BEAR_PRIME);
        let row = make_branch_eq_row(a, b, op);
        let at = AbstractTrace::new(vec![row]);
        let (flag, _, _) = eval_constraints(&at, None, &ci.constraints, BABY_BEAR_PRIME);
        assert_eq!(flag, MayBeFlag::True, "{op:?}: valid trace must satisfy constraints");
    }

    fn check_beq_invalid(op: BranchEqualOpcode, a: [u32; 4], b: [u32; 4]) {
        let (ci, _) = extract_branch_eq_constraints(BABY_BEAR_PRIME);
        let mut row = make_branch_eq_row(a, b, op);
        let orig = row[COL_BREQ_CMP_RESULT].lo;
        row[COL_BREQ_CMP_RESULT] = AbstractInterval::from_i128(1 - orig);
        let at = AbstractTrace::new(vec![row]);
        let (flag, _, _) = eval_constraints(&at, None, &ci.constraints, BABY_BEAR_PRIME);
        assert_eq!(flag, MayBeFlag::False, "{op:?}: corrupted trace must violate constraints");
    }

    // ── BLT / BLTU / BGE / BGEU helpers ──────────────────────────────────────

    fn check_blt_valid(op: BranchLessThanOpcode, a: [u32; 4], b: [u32; 4]) {
        let (ci, _) = extract_branch_lt_constraints(BABY_BEAR_PRIME);
        let row = make_branch_lt_row(a, b, op);
        let at = AbstractTrace::new(vec![row]);
        let (flag, _, _) = eval_constraints(&at, None, &ci.constraints, BABY_BEAR_PRIME);
        assert_eq!(flag, MayBeFlag::True, "{op:?}: valid trace must satisfy constraints");
    }

    fn check_blt_invalid(op: BranchLessThanOpcode, a: [u32; 4], b: [u32; 4]) {
        let (ci, _) = extract_branch_lt_constraints(BABY_BEAR_PRIME);
        let mut row = make_branch_lt_row(a, b, op);
        let orig = row[COL_BLT_CMP_RESULT].lo;
        row[COL_BLT_CMP_RESULT] = AbstractInterval::from_i128(1 - orig);
        let at = AbstractTrace::new(vec![row]);
        let (flag, _, _) = eval_constraints(&at, None, &ci.constraints, BABY_BEAR_PRIME);
        assert_eq!(flag, MayBeFlag::False, "{op:?}: corrupted trace must violate constraints");
    }

    // ── BEQ tests ─────────────────────────────────────────────────────────────

    #[test]
    fn test_beq_equal() {
        // 7 == 7 → branch taken
        check_beq_valid(BranchEqualOpcode::BEQ, [7, 0, 0, 0], [7, 0, 0, 0]);
    }

    #[test]
    fn test_beq_not_equal() {
        // 3 != 5 → branch not taken
        check_beq_valid(BranchEqualOpcode::BEQ, [3, 0, 0, 0], [5, 0, 0, 0]);
    }

    #[test]
    fn test_beq_invalid() {
        // corrupt cmp_result for equal operands
        check_beq_invalid(BranchEqualOpcode::BEQ, [4, 0, 0, 0], [4, 0, 0, 0]);
    }

    // ── BNE tests ─────────────────────────────────────────────────────────────

    #[test]
    fn test_bne_not_equal() {
        // 3 != 5 → branch taken
        check_beq_valid(BranchEqualOpcode::BNE, [3, 0, 0, 0], [5, 0, 0, 0]);
    }

    #[test]
    fn test_bne_equal() {
        // 7 == 7 → branch not taken
        check_beq_valid(BranchEqualOpcode::BNE, [7, 0, 0, 0], [7, 0, 0, 0]);
    }

    #[test]
    fn test_bne_invalid() {
        check_beq_invalid(BranchEqualOpcode::BNE, [1, 0, 0, 0], [2, 0, 0, 0]);
    }

    // ── BLT tests ─────────────────────────────────────────────────────────────

    #[test]
    fn test_blt_true() {
        // -1 < 1 (signed) → branch taken
        check_blt_valid(BranchLessThanOpcode::BLT, [0xFF, 0xFF, 0xFF, 0xFF], [1, 0, 0, 0]);
    }

    #[test]
    fn test_blt_false() {
        // 5 >= 3 (signed) → not taken
        check_blt_valid(BranchLessThanOpcode::BLT, [5, 0, 0, 0], [3, 0, 0, 0]);
    }

    #[test]
    fn test_blt_equal() {
        // 7 == 7 → not taken
        check_blt_valid(BranchLessThanOpcode::BLT, [7, 0, 0, 0], [7, 0, 0, 0]);
    }

    #[test]
    fn test_blt_invalid() {
        check_blt_invalid(BranchLessThanOpcode::BLT, [3, 0, 0, 0], [5, 0, 0, 0]);
    }

    // ── BLTU tests ────────────────────────────────────────────────────────────

    #[test]
    fn test_bltu_true() {
        check_blt_valid(BranchLessThanOpcode::BLTU, [3, 0, 0, 0], [5, 0, 0, 0]);
    }

    #[test]
    fn test_bltu_false_large() {
        // 0xFFFFFFFF > 1 unsigned → not taken
        check_blt_valid(BranchLessThanOpcode::BLTU, [0xFF, 0xFF, 0xFF, 0xFF], [1, 0, 0, 0]);
    }

    #[test]
    fn test_bltu_invalid() {
        check_blt_invalid(BranchLessThanOpcode::BLTU, [10, 0, 0, 0], [20, 0, 0, 0]);
    }

    // ── BGE tests ─────────────────────────────────────────────────────────────

    #[test]
    fn test_bge_equal() {
        // 5 >= 5 (signed) → taken
        check_blt_valid(BranchLessThanOpcode::BGE, [5, 0, 0, 0], [5, 0, 0, 0]);
    }

    #[test]
    fn test_bge_greater() {
        // 10 >= 3 (signed) → taken
        check_blt_valid(BranchLessThanOpcode::BGE, [10, 0, 0, 0], [3, 0, 0, 0]);
    }

    #[test]
    fn test_bge_less() {
        // -1 < 0 (signed) → not taken
        check_blt_valid(BranchLessThanOpcode::BGE, [0xFF, 0xFF, 0xFF, 0xFF], [0, 0, 0, 0]);
    }

    #[test]
    fn test_bge_invalid() {
        check_blt_invalid(BranchLessThanOpcode::BGE, [5, 0, 0, 0], [10, 0, 0, 0]);
    }

    // ── BGEU tests ────────────────────────────────────────────────────────────

    #[test]
    fn test_bgeu_equal() {
        check_blt_valid(BranchLessThanOpcode::BGEU, [7, 0, 0, 0], [7, 0, 0, 0]);
    }

    #[test]
    fn test_bgeu_large() {
        // 0xFFFFFFFF >= 1 unsigned → taken
        check_blt_valid(BranchLessThanOpcode::BGEU, [0xFF, 0xFF, 0xFF, 0xFF], [1, 0, 0, 0]);
    }

    #[test]
    fn test_bgeu_invalid() {
        check_blt_invalid(BranchLessThanOpcode::BGEU, [1, 0, 0, 0], [10, 0, 0, 0]);
    }
}
