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
}
