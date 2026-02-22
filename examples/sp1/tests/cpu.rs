#[cfg(test)]
mod tests {
    use p3_baby_bear::BabyBear;

    use sp1_core_executor::{Instruction, Opcode, Program};
    use sp1_core_machine::{cpu::columns::NUM_CPU_COLS, cpu::CpuChip};
    use sp1_stark::air::SP1_PROOF_NUM_PV_ELTS;

    use latticevm::constraint::eval_constraints;
    use latticevm::interval::AbstractInterval;
    use latticevm::interval::MayBeFlag;

    use latticevm::trace::AbstractTrace;

    use latticevm_sp1::utils::{extract_constraints_and_range, generate_abstract_trace};

    const PRIME: u32 = 2_u32.pow(31) - 2_u32.pow(27) + 1;

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
            extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, PRIME);

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4);
        let base_abs_main_trace_data = generate_abstract_trace(&program, "Cpu".to_string(), 5);

        let mut public_vals = vec![AbstractInterval::zero(); SP1_PROOF_NUM_PV_ELTS];
        public_vals[40] = AbstractInterval::from_i64(4);
        public_vals[41] = AbstractInterval::from_i64(18);
        public_vals[44] = AbstractInterval::one();

        let mut at = AbstractTrace::new(base_abs_main_trace_data);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::True);

        at.data[2][6] = AbstractInterval::from_i64(16);
        public_vals[41] = AbstractInterval::from_i64(16);
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
            extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, PRIME);

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4);
        let base_abs_main_trace_data = generate_abstract_trace(&program, "Cpu".to_string(), 5);

        let mut public_vals = vec![AbstractInterval::zero(); SP1_PROOF_NUM_PV_ELTS];
        public_vals[40] = AbstractInterval::from_i64(4);
        public_vals[41] = AbstractInterval::from_i64(18);
        public_vals[44] = AbstractInterval::one();

        let mut at = AbstractTrace::new(base_abs_main_trace_data);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::True);

        at.data[2][6] = AbstractInterval::from_i64(16);
        public_vals[41] = AbstractInterval::from_i64(16);
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
            extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, PRIME);

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4);
        let base_abs_main_trace_data = generate_abstract_trace(&program, "Cpu".to_string(), 5);

        let mut public_vals = vec![AbstractInterval::zero(); SP1_PROOF_NUM_PV_ELTS];
        public_vals[40] = AbstractInterval::from_i64(4);
        public_vals[41] = AbstractInterval::from_i64(18);
        public_vals[44] = AbstractInterval::one();

        let mut at = AbstractTrace::new(base_abs_main_trace_data);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::True);

        at.data[2][6] = AbstractInterval::from_i64(16);
        public_vals[41] = AbstractInterval::from_i64(16);
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
            extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, PRIME);

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4);
        let base_abs_main_trace_data = generate_abstract_trace(&program, "Cpu".to_string(), 5);

        let mut public_vals = vec![AbstractInterval::zero(); SP1_PROOF_NUM_PV_ELTS];
        public_vals[40] = AbstractInterval::from_i64(4);
        public_vals[41] = AbstractInterval::from_i64(18);
        public_vals[44] = AbstractInterval::one();

        let mut at = AbstractTrace::new(base_abs_main_trace_data);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::True);

        at.data[2][6] = AbstractInterval::from_i64(16);
        public_vals[41] = AbstractInterval::from_i64(16);
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
            extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, PRIME);

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4);
        let base_abs_main_trace_data = generate_abstract_trace(&program, "Cpu".to_string(), 5);

        let mut public_vals = vec![AbstractInterval::zero(); SP1_PROOF_NUM_PV_ELTS];
        public_vals[40] = AbstractInterval::from_i64(4);
        public_vals[41] = AbstractInterval::from_i64(18);
        public_vals[44] = AbstractInterval::one();

        let mut at = AbstractTrace::new(base_abs_main_trace_data);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::True);

        at.data[2][6] = AbstractInterval::from_i64(16);
        public_vals[41] = AbstractInterval::from_i64(16);
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
            extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, PRIME);

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4);
        let base_abs_main_trace_data = generate_abstract_trace(&program, "Cpu".to_string(), 5);

        let mut public_vals = vec![AbstractInterval::zero(); SP1_PROOF_NUM_PV_ELTS];
        public_vals[40] = AbstractInterval::from_i64(4);
        public_vals[41] = AbstractInterval::from_i64(18);
        public_vals[44] = AbstractInterval::one();

        let mut at = AbstractTrace::new(base_abs_main_trace_data);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::True);

        at.data[2][6] = AbstractInterval::from_i64(16);
        public_vals[41] = AbstractInterval::from_i64(16);
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
            extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, PRIME);

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4);
        let base_abs_main_trace_data = generate_abstract_trace(&program, "Cpu".to_string(), 5);

        let mut public_vals = vec![AbstractInterval::zero(); SP1_PROOF_NUM_PV_ELTS];
        public_vals[40] = AbstractInterval::from_i64(4);
        public_vals[41] = AbstractInterval::from_i64(18);
        public_vals[44] = AbstractInterval::one();

        let mut at = AbstractTrace::new(base_abs_main_trace_data);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::True);

        at.data[2][6] = AbstractInterval::from_i64(16);
        public_vals[41] = AbstractInterval::from_i64(16);
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
            extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, PRIME);

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4);
        let base_abs_main_trace_data = generate_abstract_trace(&program, "Cpu".to_string(), 5);

        let mut public_vals = vec![AbstractInterval::zero(); SP1_PROOF_NUM_PV_ELTS];
        public_vals[40] = AbstractInterval::from_i64(4);
        public_vals[41] = AbstractInterval::from_i64(16);
        public_vals[44] = AbstractInterval::one();

        let mut at = AbstractTrace::new(base_abs_main_trace_data);
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
            extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, PRIME);

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4);
        let base_abs_main_trace_data = generate_abstract_trace(&program, "Cpu".to_string(), 5);

        let mut public_vals = vec![AbstractInterval::zero(); SP1_PROOF_NUM_PV_ELTS];
        public_vals[40] = AbstractInterval::from_i64(4);
        public_vals[41] = AbstractInterval::from_i64(18);
        public_vals[44] = AbstractInterval::one();

        let mut at = AbstractTrace::new(base_abs_main_trace_data);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::True);

        at.data[2][6] = AbstractInterval::from_i64(16);
        public_vals[41] = AbstractInterval::from_i64(16);
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
            extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, PRIME);

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4);
        let base_abs_main_trace_data = generate_abstract_trace(&program, "Cpu".to_string(), 5);

        let mut public_vals = vec![AbstractInterval::zero(); SP1_PROOF_NUM_PV_ELTS];
        public_vals[40] = AbstractInterval::from_i64(4);
        public_vals[41] = AbstractInterval::from_i64(16);
        public_vals[44] = AbstractInterval::one();

        let mut at = AbstractTrace::new(base_abs_main_trace_data);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::True);
    }

    #[test]
    fn test_branch_jal() {
        fn target_program(pc_start: u32, pc_base: u32) -> Program {
            let mut instructions = vec![Instruction::new(Opcode::JAL, 29, 9, 0, true, true)];

            Program::new(instructions, pc_start, pc_base)
        }

        // ######################## Extract CPU Constraints ##########################
        let air = CpuChip::default();
        let (constraint_info, _general_lookup_info) =
            extract_constraints_and_range::<BabyBear, CpuChip>(&air, NUM_CPU_COLS, PRIME);

        // ######################## Program Initialization ###########################
        let program = target_program(4, 4);
        let base_abs_main_trace_data = generate_abstract_trace(&program, "Cpu".to_string(), 5);

        let mut public_vals = vec![AbstractInterval::zero(); SP1_PROOF_NUM_PV_ELTS];
        public_vals[40] = AbstractInterval::from_i64(4);
        public_vals[41] = AbstractInterval::from_i64(13);
        public_vals[44] = AbstractInterval::one();

        let mut at = AbstractTrace::new(base_abs_main_trace_data);
        println!("{}", at);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::True);

        at.data[2][6] = AbstractInterval::from_i64(8);
        public_vals[41] = AbstractInterval::from_i64(8);
        let result = eval_constraints(&at, Some(&public_vals), &constraint_info.constraints, PRIME);
        assert!(result.0 == MayBeFlag::False);
    }
}
