#[cfg(test)]
mod tests {
    use openvm_rv32im_transpiler::ShiftOpcode;
    use openvm_stark_sdk::p3_baby_bear::BabyBear;

    use zebra::constraint::eval_constraints;
    use zebra::interval::{AbstractInterval, MayBeFlag};
    use zebra::trace::AbstractTrace;

    use zebra_openvm::utils::{
        extract_shift_constraints, make_shift_row, BABY_BEAR_PRIME, COL_SHIFT_A_START,
    };

    type F = BabyBear;

    fn check_valid(b_limbs: [u32; 4], c_limbs: [u32; 4]) {
        let (constraint_info, _) = extract_shift_constraints(BABY_BEAR_PRIME);
        let row = make_shift_row(b_limbs, c_limbs, ShiftOpcode::SRL);
        let at = AbstractTrace::new(vec![row]);
        let (flag, _, _) =
            eval_constraints(&at, None, &constraint_info.constraints, BABY_BEAR_PRIME);
        assert_eq!(
            flag,
            MayBeFlag::True,
            "SRL: valid trace should satisfy all constraints"
        );
    }

    fn check_invalid(b_limbs: [u32; 4], c_limbs: [u32; 4]) {
        let (constraint_info, _) = extract_shift_constraints(BABY_BEAR_PRIME);
        let mut row = make_shift_row(b_limbs, c_limbs, ShiftOpcode::SRL);
        let orig = row[COL_SHIFT_A_START].lo;
        row[COL_SHIFT_A_START] = AbstractInterval::from_i128((orig + 1) % 256);
        let at = AbstractTrace::new(vec![row]);
        let (flag, _, _) =
            eval_constraints(&at, None, &constraint_info.constraints, BABY_BEAR_PRIME);
        assert_eq!(
            flag,
            MayBeFlag::False,
            "SRL: corrupted trace should violate constraints"
        );
    }

    #[test]
    fn test_srl_simple() {
        // 0xFF >> 4 = 0x0F
        check_valid([0xFF, 0, 0, 0], [4, 0, 0, 0]);
    }

    #[test]
    fn test_srl_zero_shift() {
        // x >> 0 = x
        check_valid([0xAB, 0xCD, 0, 0], [0, 0, 0, 0]);
    }

    #[test]
    fn test_srl_full_shift() {
        // 0xFF_FF_FF_FF >> 31 = 1
        check_valid([0xFF, 0xFF, 0xFF, 0xFF], [31, 0, 0, 0]);
    }

    #[test]
    fn test_srl_invalid() {
        check_invalid([0xFF, 0, 0, 0], [1, 0, 0, 0]);
    }
}
