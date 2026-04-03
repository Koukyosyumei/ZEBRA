#[cfg(test)]
mod tests {
    use openvm_rv32im_transpiler::LessThanOpcode;
    use openvm_stark_sdk::p3_baby_bear::BabyBear;

    use zebra::constraint::eval_constraints;
    use zebra::interval::{AbstractInterval, MayBeFlag};
    use zebra::trace::AbstractTrace;

    use zebra_openvm::utils::{
        extract_lt_constraints, make_lt_row, BABY_BEAR_PRIME, COL_LT_CMP_RESULT,
    };

    type F = BabyBear;

    fn check_valid(op: LessThanOpcode, b_limbs: [u32; 4], c_limbs: [u32; 4]) {
        let (constraint_info, _) = extract_lt_constraints::<F>(BABY_BEAR_PRIME);
        let row = make_lt_row(b_limbs, c_limbs, op);
        let at = AbstractTrace::new(vec![row]);
        let (flag, _, _) =
            eval_constraints(&at, None, &constraint_info.constraints, BABY_BEAR_PRIME);
        assert_eq!(
            flag,
            MayBeFlag::True,
            "{op:?}: valid trace should satisfy all constraints"
        );
    }

    fn check_invalid(op: LessThanOpcode, b_limbs: [u32; 4], c_limbs: [u32; 4]) {
        let (constraint_info, _) = extract_lt_constraints::<F>(BABY_BEAR_PRIME);
        let mut row = make_lt_row(b_limbs, c_limbs, op);
        // Flip the comparison result.
        let orig = row[COL_LT_CMP_RESULT].lo;
        row[COL_LT_CMP_RESULT] = AbstractInterval::from_i128(1 - orig);
        let at = AbstractTrace::new(vec![row]);
        let (flag, _, _) =
            eval_constraints(&at, None, &constraint_info.constraints, BABY_BEAR_PRIME);
        assert_eq!(
            flag,
            MayBeFlag::False,
            "{op:?}: corrupted trace should violate constraints"
        );
    }

    // ── SLT ───────────────────────────────────────────────────────────────────

    #[test]
    fn test_slt_true() {
        // -1 < 1  (signed)  →  1
        // -1 as u32 = 0xFFFF_FFFF = [0xFF, 0xFF, 0xFF, 0xFF]
        check_valid(LessThanOpcode::SLT, [0xFF, 0xFF, 0xFF, 0xFF], [1, 0, 0, 0]);
    }

    #[test]
    fn test_slt_false() {
        // 5 < 3 = false → 0
        check_valid(LessThanOpcode::SLT, [5, 0, 0, 0], [3, 0, 0, 0]);
    }

    #[test]
    fn test_slt_equal() {
        // 7 < 7 = false → 0
        check_valid(LessThanOpcode::SLT, [7, 0, 0, 0], [7, 0, 0, 0]);
    }

    #[test]
    fn test_slt_invalid() {
        check_invalid(LessThanOpcode::SLT, [3, 0, 0, 0], [5, 0, 0, 0]);
    }

    // ── SLTU ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_sltu_true() {
        // 3 < 5 (unsigned) → 1
        check_valid(LessThanOpcode::SLTU, [3, 0, 0, 0], [5, 0, 0, 0]);
    }

    #[test]
    fn test_sltu_false_negative() {
        // 0xFFFF_FFFF > 1 (unsigned) → 0
        check_valid(LessThanOpcode::SLTU, [0xFF, 0xFF, 0xFF, 0xFF], [1, 0, 0, 0]);
    }

    #[test]
    fn test_sltu_invalid() {
        check_invalid(LessThanOpcode::SLTU, [10, 0, 0, 0], [20, 0, 0, 0]);
    }
}
