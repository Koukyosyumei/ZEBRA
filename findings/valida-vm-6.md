Currently, for branch instructions such as `BNE`, the CPU chip does not check whether the `address` of the second `mem_read_channel` is equal to the second operand, even if it is NOT the immediate value.

A working PoC is available here: https://github.com/Koukyosyumei/valida-vm/tree/poc_bne_read_addr_2

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
CPU row 0: CpuCols { clk: 0, pc: 0, fp: 4096, instruction: InstructionCols { opcode: 7, operands: Operands([0, 1, 0, 0, 0]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 0, is_pointer_op: 0, is_imm_op: 0, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 0, is_jal: 0, is_jalv: 0, is_imm32: 1, is_advice: 0, is_stop: 0, is_loadfp: 0, is_write: 0 }, diff: 0, diff_inv: 0, not_equal: 0, mem_read_channels: [ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }, ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 1, addr: 4096, value: Word([1, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }
CPU row 1: CpuCols { clk: 1, pc: 1, fp: 4096, instruction: InstructionCols { opcode: 7, operands: Operands([2013265917, 2, 0, 0, 0]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 0, is_pointer_op: 0, is_imm_op: 0, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 0, is_jal: 0, is_jalv: 0, is_imm32: 1, is_advice: 0, is_stop: 0, is_loadfp: 0, is_write: 0 }, diff: 0, diff_inv: 0, not_equal: 0, mem_read_channels: [ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }, ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 1, addr: 4092, value: Word([2, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }
CPU row 2: CpuCols { clk: 2, pc: 2, fp: 4096, instruction: InstructionCols { opcode: 100, operands: Operands([2013265913, 2013265913, 1, 0, 1]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 1, is_pointer_op: 0, is_imm_op: 1, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 0, is_jal: 0, is_jalv: 0, is_imm32: 0, is_advice: 0, is_stop: 0, is_loadfp: 0, is_write: 0 }, diff: 1, diff_inv: 1, not_equal: 1, mem_read_channels: [ReadChannelCols { used: 1, addr: 4088, value: Word([0, 0, 0, 0]) }, ReadChannelCols { used: 0, addr: 0, value: Word([1, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 1, addr: 4088, value: Word([1, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }
CPU row 3: CpuCols { clk: 3, pc: 3, fp: 4096, instruction: InstructionCols { opcode: 6, operands: Operands([24, 2013265913, 2013265917, 0, 0]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 0, is_pointer_op: 0, is_imm_op: 0, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 1, is_jal: 0, is_jalv: 0, is_imm32: 0, is_advice: 0, is_stop: 0, is_loadfp: 0, is_write: 0 }, diff: 1, diff_inv: 1, not_equal: 1, mem_read_channels: [ReadChannelCols { used: 1, addr: 4088, value: Word([1, 0, 0, 0]) }, ReadChannelCols { used: 1, addr: 4092, value: Word([2, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }
CPU row 4: CpuCols { clk: 4, pc: 1, fp: 4096, instruction: InstructionCols { opcode: 7, operands: Operands([2013265917, 2, 0, 0, 0]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 0, is_pointer_op: 0, is_imm_op: 0, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 0, is_jal: 0, is_jalv: 0, is_imm32: 1, is_advice: 0, is_stop: 0, is_loadfp: 0, is_write: 0 }, diff: 0, diff_inv: 0, not_equal: 0, mem_read_channels: [ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }, ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 1, addr: 4092, value: Word([2, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }
CPU row 5: CpuCols { clk: 5, pc: 2, fp: 4096, instruction: InstructionCols { opcode: 100, operands: Operands([2013265913, 2013265913, 1, 0, 1]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 1, is_pointer_op: 0, is_imm_op: 1, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 0, is_jal: 0, is_jalv: 0, is_imm32: 0, is_advice: 0, is_stop: 0, is_loadfp: 0, is_write: 0 }, diff: 0, diff_inv: 0, not_equal: 0, mem_read_channels: [ReadChannelCols { used: 1, addr: 4088, value: Word([1, 0, 0, 0]) }, ReadChannelCols { used: 0, addr: 0, value: Word([1, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 1, addr: 4088, value: Word([2, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }
CPU row 6: CpuCols { clk: 6, pc: 3, fp: 4096, instruction: InstructionCols { opcode: 6, operands: Operands([24, 2013265913, 2013265917, 0, 0]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 0, is_pointer_op: 0, is_imm_op: 0, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 1, is_jal: 0, is_jalv: 0, is_imm32: 0, is_advice: 0, is_stop: 0, is_loadfp: 0, is_write: 0 }, diff: 0, diff_inv: 0, not_equal: 0, mem_read_channels: [ReadChannelCols { used: 1, addr: 4088, value: Word([2, 0, 0, 0]) }, ReadChannelCols { used: 1, addr: 4092, value: Word([2, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }
CPU row 7: CpuCols { clk: 7, pc: 4, fp: 4096, instruction: InstructionCols { opcode: 8, operands: Operands([0, 0, 0, 0, 0]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 0, is_pointer_op: 0, is_imm_op: 0, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 0, is_jal: 0, is_jalv: 0, is_imm32: 0, is_advice: 0, is_stop: 1, is_loadfp: 0, is_write: 0 }, diff: 0, diff_inv: 0, not_equal: 0, mem_read_channels: [ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }, ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }
Main trace dimensions for Chip: CPU
```

By modifying the execution function of the `BneInstruction`:

```rust
let read_addr_2 = state.machine.cpu().fp as u32; //(state.machine.cpu().fp as i32 + ops.c()) as u32;
```

The program reads the value from an address different from the operand’s intended address, altering the state transition:

```
CPU row 0: CpuCols { clk: 0, pc: 0, fp: 4096, instruction: InstructionCols { opcode: 7, operands: Operands([0, 1, 0, 0, 0]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 0, is_pointer_op: 0, is_imm_op: 0, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 0, is_jal: 0, is_jalv: 0, is_imm32: 1, is_advice: 0, is_stop: 0, is_loadfp: 0, is_write: 0 }, diff: 0, diff_inv: 0, not_equal: 0, mem_read_channels: [ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }, ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 1, addr: 4096, value: Word([1, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }
CPU row 1: CpuCols { clk: 1, pc: 1, fp: 4096, instruction: InstructionCols { opcode: 7, operands: Operands([2013265917, 2, 0, 0, 0]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 0, is_pointer_op: 0, is_imm_op: 0, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 0, is_jal: 0, is_jalv: 0, is_imm32: 1, is_advice: 0, is_stop: 0, is_loadfp: 0, is_write: 0 }, diff: 0, diff_inv: 0, not_equal: 0, mem_read_channels: [ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }, ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 1, addr: 4092, value: Word([2, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }
CPU row 2: CpuCols { clk: 2, pc: 2, fp: 4096, instruction: InstructionCols { opcode: 100, operands: Operands([2013265913, 2013265913, 1, 0, 1]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 1, is_pointer_op: 0, is_imm_op: 1, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 0, is_jal: 0, is_jalv: 0, is_imm32: 0, is_advice: 0, is_stop: 0, is_loadfp: 0, is_write: 0 }, diff: 1, diff_inv: 1, not_equal: 1, mem_read_channels: [ReadChannelCols { used: 1, addr: 4088, value: Word([0, 0, 0, 0]) }, ReadChannelCols { used: 0, addr: 0, value: Word([1, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 1, addr: 4088, value: Word([1, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }
CPU row 3: CpuCols { clk: 3, pc: 3, fp: 4096, instruction: InstructionCols { opcode: 6, operands: Operands([24, 2013265913, 2013265917, 0, 0]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 0, is_pointer_op: 0, is_imm_op: 0, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 1, is_jal: 0, is_jalv: 0, is_imm32: 0, is_advice: 0, is_stop: 0, is_loadfp: 0, is_write: 0 }, diff: 0, diff_inv: 0, not_equal: 0, mem_read_channels: [ReadChannelCols { used: 1, addr: 4088, value: Word([1, 0, 0, 0]) }, ReadChannelCols { used: 1, addr: 4096, value: Word([1, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }
CPU row 4: CpuCols { clk: 4, pc: 4, fp: 4096, instruction: InstructionCols { opcode: 8, operands: Operands([0, 0, 0, 0, 0]) }, opcode_flags: OpcodeFlagCols { is_bus_op: 0, is_pointer_op: 0, is_imm_op: 0, is_left_imm_op: 0, is_load: 0, is_load_u8: 0, is_load_s8: 0, is_store: 0, is_store_u8: 0, is_beq: 0, is_bne: 0, is_jal: 0, is_jalv: 0, is_imm32: 0, is_advice: 0, is_stop: 1, is_loadfp: 0, is_write: 0 }, diff: 0, diff_inv: 0, not_equal: 0, mem_read_channels: [ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }, ReadChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]) }], mem_write_channels: [WriteChannelCols { used: 0, addr: 0, value: Word([0, 0, 0, 0]), old_value: Word([0, 0, 0, 0]) }], addr_offset_flags: Word([0, 0, 0, 0]), sign_bit: 0, is_last_segment: 1, is_real: 1 }
```
