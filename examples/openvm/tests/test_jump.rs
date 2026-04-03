#[cfg(test)]
mod tests {
    use openvm_stark_sdk::p3_baby_bear::BabyBear;

    use zebra::constraint::eval_constraints;
    use zebra::interval::{AbstractInterval, MayBeFlag};
    use zebra::trace::AbstractTrace;

    use zebra_openvm::utils::{
        extract_jal_constraints, extract_jalr_constraints, make_jal_row, make_jalr_row,
        make_lui_row, to_limbs, BABY_BEAR_PRIME, COL_JAL_RD_START, COL_JALR_IS_VALID,
    };

    type F = BabyBear;

    // ── JAL helpers ───────────────────────────────────────────────────────────

    fn check_jal_valid(from_pc: u32, imm: i32) {
        let (ci, _) = extract_jal_constraints(BABY_BEAR_PRIME);
        let row = make_jal_row(from_pc, imm);
        let at = AbstractTrace::new(vec![row]);
        let (flag, _, _) = eval_constraints(&at, None, &ci.constraints, BABY_BEAR_PRIME);
        assert_eq!(flag, MayBeFlag::True, "JAL({from_pc}, {imm}): valid trace must pass");
    }

    fn check_jal_invalid(from_pc: u32, imm: i32) {
        let (ci, _) = extract_jal_constraints(BABY_BEAR_PRIME);
        let mut row = make_jal_row(from_pc, imm);
        // Corrupt rd_data[0] (the first limb of from_pc + 4).
        let orig = row[COL_JAL_RD_START].lo;
        row[COL_JAL_RD_START] = AbstractInterval::from_i128(orig ^ 1);
        let at = AbstractTrace::new(vec![row]);
        let (flag, _, _) = eval_constraints(&at, None, &ci.constraints, BABY_BEAR_PRIME);
        assert_eq!(flag, MayBeFlag::False, "JAL({from_pc}, {imm}): corrupted trace must fail");
    }

    // ── LUI helpers ───────────────────────────────────────────────────────────

    fn check_lui_valid(imm: u32) {
        let (ci, _) = extract_jal_constraints(BABY_BEAR_PRIME);
        let row = make_lui_row(imm);
        let at = AbstractTrace::new(vec![row]);
        let (flag, _, _) = eval_constraints(&at, None, &ci.constraints, BABY_BEAR_PRIME);
        assert_eq!(flag, MayBeFlag::True, "LUI({imm}): valid trace must pass");
    }

    fn check_lui_invalid(imm: u32) {
        let (ci, _) = extract_jal_constraints(BABY_BEAR_PRIME);
        let mut row = make_lui_row(imm);
        // Corrupt rd_data[1] (one of the limbs of imm << 12).
        let orig = row[COL_JAL_RD_START + 1].lo;
        row[COL_JAL_RD_START + 1] = AbstractInterval::from_i128(orig ^ 1);
        let at = AbstractTrace::new(vec![row]);
        let (flag, _, _) = eval_constraints(&at, None, &ci.constraints, BABY_BEAR_PRIME);
        assert_eq!(flag, MayBeFlag::False, "LUI({imm}): corrupted trace must fail");
    }

    // ── JALR helpers ──────────────────────────────────────────────────────────

    fn check_jalr_valid(rs1: [u32; 4], imm: u32, imm_sign: u32) {
        let (ci, _) = extract_jalr_constraints(BABY_BEAR_PRIME);
        let row = make_jalr_row(rs1, imm, imm_sign);
        let at = AbstractTrace::new(vec![row]);
        let (flag, _, _) = eval_constraints(&at, None, &ci.constraints, BABY_BEAR_PRIME);
        assert_eq!(
            flag,
            MayBeFlag::True,
            "JALR(rs1={rs1:?}, imm={imm}, imm_sign={imm_sign}): valid trace must pass"
        );
    }

    fn check_jalr_invalid(rs1: [u32; 4], imm: u32, imm_sign: u32) {
        let (ci, _) = extract_jalr_constraints(BABY_BEAR_PRIME);
        let mut row = make_jalr_row(rs1, imm, imm_sign);
        // Corrupt is_valid → flip to 0 so it's trivially wrong.
        row[COL_JALR_IS_VALID] = AbstractInterval::from_i128(0);
        // Then also corrupt one of the to_pc_limbs so the carry constraint fails.
        let orig = row[zebra_openvm::utils::COL_JALR_TO_PC_LIMBS_START].lo;
        row[zebra_openvm::utils::COL_JALR_TO_PC_LIMBS_START] =
            AbstractInterval::from_i128(orig.wrapping_add(1));
        let at = AbstractTrace::new(vec![row]);
        // With is_valid=0 the carry constraints are trivially 0; so we use valid trace
        // with wrong to_pc and is_valid=1 to actually fail.
        let mut row2 = make_jalr_row(rs1, imm, imm_sign);
        let orig2 = row2[zebra_openvm::utils::COL_JALR_TO_PC_LIMBS_START].lo;
        row2[zebra_openvm::utils::COL_JALR_TO_PC_LIMBS_START] =
            AbstractInterval::from_i128(orig2.wrapping_add(1));
        let at2 = AbstractTrace::new(vec![row2]);
        let (flag, _, _) = eval_constraints(&at2, None, &ci.constraints, BABY_BEAR_PRIME);
        assert_eq!(
            flag,
            MayBeFlag::False,
            "JALR: corrupted to_pc_limbs must fail"
        );
    }

    // ── JAL tests ─────────────────────────────────────────────────────────────

    #[test]
    fn test_jal_positive_imm() {
        // from_pc=0, imm=100 → to_pc=100, rd=4
        check_jal_valid(0, 100);
    }

    #[test]
    fn test_jal_zero_imm() {
        check_jal_valid(0, 0);
    }

    #[test]
    fn test_jal_from_nonzero_pc() {
        // from_pc=8, imm=16 → to_pc=24, rd=12
        check_jal_valid(8, 16);
    }

    #[test]
    fn test_jal_invalid_rd() {
        check_jal_invalid(0, 4);
    }

    // ── LUI tests ─────────────────────────────────────────────────────────────

    #[test]
    fn test_lui_small() {
        // imm=1 → rd = 0x1000
        check_lui_valid(1);
    }

    #[test]
    fn test_lui_zero() {
        check_lui_valid(0);
    }

    #[test]
    fn test_lui_large() {
        // imm=0xFFFFF → rd = 0xFFFFF000
        check_lui_valid(0xFFFFF);
    }

    #[test]
    fn test_lui_invalid_rd() {
        check_lui_invalid(5);
    }

    // ── JALR tests ────────────────────────────────────────────────────────────

    #[test]
    fn test_jalr_simple() {
        // rs1=100, imm=4, imm_sign=0 → to_pc=104
        check_jalr_valid(to_limbs(100), 4, 0);
    }

    #[test]
    fn test_jalr_zero_offset() {
        check_jalr_valid(to_limbs(0x1000), 0, 0);
    }

    #[test]
    fn test_jalr_with_carry() {
        // rs1=65535, imm=2, imm_sign=0 → to_pc=65536 (carry from low limbs)
        check_jalr_valid(to_limbs(65535), 2, 0);
    }

    #[test]
    fn test_jalr_clears_lsb() {
        // rs1=100, imm=1, imm_sign=0 → to_pc_raw=101, to_pc=100 (LSB cleared)
        check_jalr_valid(to_limbs(100), 1, 0);
    }

    #[test]
    fn test_jalr_invalid_to_pc() {
        check_jalr_invalid(to_limbs(200), 0, 0);
    }
}
