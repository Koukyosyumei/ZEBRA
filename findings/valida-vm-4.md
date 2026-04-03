Currently, witness generation fills the entire `is_last_segment` column with `self.is_last_segment`:
[https://github.com/lita-xyz/valida-vm/blob/c36d3177c1708c70d641cfda8512ac42b1e6d9bf/cpu/src/lib.rs#L447](https://github.com/lita-xyz/valida-vm/blob/c36d3177c1708c70d641cfda8512ac42b1e6d9bf/cpu/src/lib.rs#L447)

However, the AIR constraint only enforces correctness on the *first row*:
[https://github.com/lita-xyz/valida-vm/blob/c36d3177c1708c70d641cfda8512ac42b1e6d9bf/cpu/src/stark.rs#L543](https://github.com/lita-xyz/valida-vm/blob/c36d3177c1708c70d641cfda8512ac42b1e6d9bf/cpu/src/stark.rs#L543)

This means a malicious prover can arbitrarily flip non–first-row values of `is_last_segment`. Doing so reintroduces the **early termination vulnerability** (see #16). Importantly, the fix in #17 assumes that `is_last_segment` is trustworthy, and manipulating later rows bypasses that protection.

A working PoC is available: [https://github.com/Koukyosyumei/valida-vm/tree/fix-poc-mal-DidStop-flag](https://github.com/Koukyosyumei/valida-vm/tree/fix-poc-mal-DidStop-flag)

This PoC reproduces the attack from #16 with a minor mutation in the CPU chip’s `op_to_row`:

```rust
if self.registers[clk as usize].pc != 0 {
    cols.is_last_segment = SC::Val::zero();
}
```

**Proposed Fix:**
Remove the `.when_first_row()` restriction in:
[https://github.com/lita-xyz/valida-vm/blob/c36d3177c1708c70d641cfda8512ac42b1e6d9bf/cpu/src/stark.rs#L543](https://github.com/lita-xyz/valida-vm/blob/c36d3177c1708c70d641cfda8512ac42b1e6d9bf/cpu/src/stark.rs#L543)

and instead enforce:

```rust
assert_eq(local.is_last_segment, public.is_last_segment);
```

on **all rows**, ensuring that every value of `is_last_segment` is consistent and not forgeable.
