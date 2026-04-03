Currently, the CPU chip does not check whether the `is_real` column of the first row is set to 1. Consequently, the malicious prover can generate a valid proof without executing the program at all.

A working PoC is available: https://github.com/Koukyosyumei/valida-vm/tree/poc_first_is_read_can_be_zero

Run with:

```bash
cd basic-api/tests/
cargo test prove_target --release
```

This PoC comments out the entire execution loop while setting the appropriate values to `pc`, `fp`, and `is_last_segment` to bypass the boundary constraints:

```rust
        let mut final_stop_flag = StoppingFlag::DidStop;

        /*
        let mut step_did_stop = StoppingFlag::DidNotStop;
        loop {
            let pc = state.machine.cpu().pc;
	@@ -781,7 +784,7 @@ impl<F: StarkField> Machine<F> for BasicMachine<F> {
                final_stop_flag = step_did_stop;
                break;
            }
        }*/
```

```rust
        // Generate main traces.
        let t_main_traces = start_timer!(|| "valida >machine.prove(..) | main_traces");
        let mut main_traces = self.generate_main_traces(config, show_main, show_main_dims);
        if let Some(trace) = &mut main_traces[0] {
            let row: &mut CpuCols<F> = trace.row_mut(0).borrow_mut();
            row.fp = F::from_canonical_u16(4096);
            row.opcode_flags.is_stop = F::one();
            row.is_last_segment = F::one();
            println!("row: {:?}", row);
        }
        end_timer!(t_main_traces);
```

**Proposed Fix:**

Add the boundary check:

```rust
builder.when_first_row.assert_one(local.is_real);
```
