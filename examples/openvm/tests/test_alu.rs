#[cfg(test)]
mod tests {
    use openvm_rv32im_transpiler::BaseAluOpcode;
    use openvm_stark_sdk::p3_baby_bear::BabyBear;

    use zebra::constraint::eval_constraints;
    use zebra::interval::{AbstractInterval, MayBeFlag};
    use zebra::trace::AbstractTrace;

    use zebra_openvm::utils::{
        extract_base_alu_constraints, make_base_alu_row, BABY_BEAR_PRIME, COL_A_START,
    };

    type F = BabyBear;

    /// Build a single-row trace and verify that all constraints hold.
    fn check_valid(op: BaseAluOpcode, b_limbs: [u32; 4], c_limbs: [u32; 4]) {
        let (constraint_info, _) = extract_base_alu_constraints::<F>(BABY_BEAR_PRIME);
        let row = make_base_alu_row(b_limbs, c_limbs, op);
        let at = AbstractTrace::new(vec![row]);
        let (flag, _, _) =
            eval_constraints(&at, None, &constraint_info.constraints, BABY_BEAR_PRIME);
        assert_eq!(
            flag,
            MayBeFlag::True,
            "{op:?}: valid trace should satisfy all constraints"
        );
    }

    /// Build a trace with a corrupted result and verify that a constraint is violated.
    fn check_invalid(op: BaseAluOpcode, b_limbs: [u32; 4], c_limbs: [u32; 4]) {
        let (constraint_info, _) = extract_base_alu_constraints::<F>(BABY_BEAR_PRIME);
        let mut row = make_base_alu_row(b_limbs, c_limbs, op);
        // Corrupt the lowest limb of the result.
        let orig = row[COL_A_START].lo;
        row[COL_A_START] = AbstractInterval::from_i128((orig + 1) % 256);
        let at = AbstractTrace::new(vec![row]);
        let (flag, _, _) =
            eval_constraints(&at, None, &constraint_info.constraints, BABY_BEAR_PRIME);
        assert_eq!(
            flag,
            MayBeFlag::False,
            "{op:?}: corrupted trace should violate constraints"
        );
    }

    // ── ADD ───────────────────────────────────────────────────────────────────

    #[test]
    fn test_add_simple() {
        // 3 + 5 = 8 (fits in first limb)
        check_valid(BaseAluOpcode::ADD, [3, 0, 0, 0], [5, 0, 0, 0]);
    }

    #[test]
    fn test_add_carry() {
        // 255 + 1 = 256 = [0, 1, 0, 0] (carry into second limb)
        check_valid(BaseAluOpcode::ADD, [255, 0, 0, 0], [1, 0, 0, 0]);
    }

    #[test]
    fn test_add_overflow() {
        // 0xFFFF_FFFF + 1 = 0 (mod 2^32)
        check_valid(BaseAluOpcode::ADD, [255, 255, 255, 255], [1, 0, 0, 0]);
    }

    #[test]
    fn test_add_invalid() {
        check_invalid(BaseAluOpcode::ADD, [10, 0, 0, 0], [20, 0, 0, 0]);
    }

    // ── SUB ───────────────────────────────────────────────────────────────────

    #[test]
    fn test_sub_simple() {
        // 10 - 3 = 7
        check_valid(BaseAluOpcode::SUB, [10, 0, 0, 0], [3, 0, 0, 0]);
    }

    #[test]
    fn test_sub_borrow() {
        // 0 - 1 = 0xFFFF_FFFF (wrapping)
        check_valid(BaseAluOpcode::SUB, [0, 0, 0, 0], [1, 0, 0, 0]);
    }

    #[test]
    fn test_sub_invalid() {
        check_invalid(BaseAluOpcode::SUB, [10, 5, 0, 0], [3, 2, 0, 0]);
    }

    // ── XOR ───────────────────────────────────────────────────────────────────

    #[test]
    fn test_xor_simple() {
        // 0b1010 XOR 0b1100 = 0b0110 = 6
        check_valid(BaseAluOpcode::XOR, [0b1010, 0, 0, 0], [0b1100, 0, 0, 0]);
    }

    #[test]
    fn test_xor_self() {
        // x XOR x = 0
        check_valid(BaseAluOpcode::XOR, [42, 17, 8, 3], [42, 17, 8, 3]);
    }

    #[test]
    fn test_xor_invalid() {
        check_invalid(BaseAluOpcode::XOR, [0xFF, 0, 0, 0], [0x0F, 0, 0, 0]);
    }

    // ── OR ────────────────────────────────────────────────────────────────────

    #[test]
    fn test_or_simple() {
        // 0b1010 OR 0b1100 = 0b1110 = 14
        check_valid(BaseAluOpcode::OR, [0b1010, 0, 0, 0], [0b1100, 0, 0, 0]);
    }

    #[test]
    fn test_or_zeros() {
        check_valid(BaseAluOpcode::OR, [0, 0, 0, 0], [0, 0, 0, 0]);
    }

    #[test]
    fn test_or_invalid() {
        check_invalid(BaseAluOpcode::OR, [0xAA, 0, 0, 0], [0x55, 0, 0, 0]);
    }

    // ── AND ───────────────────────────────────────────────────────────────────

    #[test]
    fn test_and_simple() {
        // 0b1010 AND 0b1100 = 0b1000 = 8
        check_valid(BaseAluOpcode::AND, [0b1010, 0, 0, 0], [0b1100, 0, 0, 0]);
    }

    #[test]
    fn test_and_zeros() {
        check_valid(BaseAluOpcode::AND, [0xFF, 0, 0, 0], [0x00, 0, 0, 0]);
    }

    #[test]
    fn test_and_invalid() {
        check_invalid(BaseAluOpcode::AND, [0xFF, 0, 0, 0], [0xFF, 0, 0, 0]);
    }
}
