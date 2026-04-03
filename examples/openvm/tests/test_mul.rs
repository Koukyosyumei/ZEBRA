#[cfg(test)]
mod tests {
    use openvm_stark_sdk::p3_baby_bear::BabyBear;

    use zebra::constraint::eval_constraints;
    use zebra::interval::{AbstractInterval, MayBeFlag};
    use zebra::trace::AbstractTrace;

    use zebra_openvm::utils::{
        extract_mul_constraints, make_mul_row, BABY_BEAR_PRIME, COL_MUL_A_START,
    };

    type F = BabyBear;

    fn check_valid(b_limbs: [u32; 4], c_limbs: [u32; 4]) {
        let (constraint_info, _) = extract_mul_constraints::<F>(BABY_BEAR_PRIME);
        let row = make_mul_row(b_limbs, c_limbs);
        let at = AbstractTrace::new(vec![row]);
        let (flag, _, _) =
            eval_constraints(&at, None, &constraint_info.constraints, BABY_BEAR_PRIME);
        assert_eq!(
            flag,
            MayBeFlag::True,
            "MUL: valid trace should satisfy all constraints"
        );
    }

    fn check_invalid(b_limbs: [u32; 4], c_limbs: [u32; 4]) {
        let (constraint_info, _) = extract_mul_constraints::<F>(BABY_BEAR_PRIME);
        let mut row = make_mul_row(b_limbs, c_limbs);
        let orig = row[COL_MUL_A_START].lo;
        row[COL_MUL_A_START] = AbstractInterval::from_i128((orig + 1) % 256);
        let at = AbstractTrace::new(vec![row]);
        let (flag, _, _) =
            eval_constraints(&at, None, &constraint_info.constraints, BABY_BEAR_PRIME);
        assert_eq!(
            flag,
            MayBeFlag::False,
            "MUL: corrupted trace should violate constraints"
        );
    }

    #[test]
    fn test_mul_simple() {
        // 3 * 5 = 15
        check_valid([3, 0, 0, 0], [5, 0, 0, 0]);
    }

    #[test]
    fn test_mul_by_zero() {
        check_valid([42, 17, 0, 0], [0, 0, 0, 0]);
    }

    #[test]
    fn test_mul_overflow() {
        // 0xFFFF_FFFF * 2 = 0xFFFF_FFFE (mod 2^32)
        check_valid([0xFF, 0xFF, 0xFF, 0xFF], [2, 0, 0, 0]);
    }

    #[test]
    fn test_mul_invalid() {
        check_invalid([7, 0, 0, 0], [3, 0, 0, 0]);
    }
}
