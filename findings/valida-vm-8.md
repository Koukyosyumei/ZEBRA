The CPU chip currently does **not enforce a range check on the written word**, which allows a malicious prover to write a value whose **wrapped field value** matches the expected value modulo the field prime.

As a result, different byte decompositions can evaluate to the same field element under the BabyBear prime, allowing incorrect values to pass verification.

For example, the following code in the CPU chip checks:

```
24 * (pc + 1) == value[0] + value[1] * 256 + value[2] * (256^2) + value[3] * (256^3)
```

Code reference:

```rust
// https://github.com/lita-xyz/valida-vm/blob/c36d3177c1708c70d641cfda8512ac42b1e6d9bf/cpu/src/stark.rs#L385

builder.when_transition().when(is_jal + is_jalv).assert_eq(
    bytes_per_instr_expr.clone() * (local.pc + AB::F::one()),
    reduce::<AB>(base, local.write_value()),
);
```

However, since the computation is performed in the BabyBear field, the byte decomposition is **not guaranteed to represent a canonical 32-bit value**.

For instance, both $[24, 0, 0, 0]$ and $[50, 0, 0, 240]$ evaluate to `24` modulo the BabyBear prime, even though they represent different 32-bit integers.

This means a malicious prover could supply a non-canonical byte representation that still satisfies the constraint in the field.

### Proposed fix

A **range check should be added** to ensure that the reconstructed word does not exceed the field modulus.

One possible reference implementation is the range-checke circuit used here:

[https://github.com/ProjectZKM/Ziren/blob/7af605dba7e9980c5451102364d03bd119318fad/crates/core/machine/src/operations/koala_bear_word.rs#L11](https://github.com/ProjectZKM/Ziren/blob/7af605dba7e9980c5451102364d03bd119318fad/crates/core/machine/src/operations/koala_bear_word.rs#L11)

Adding a similar check would prevent wrapped representations from satisfying the constraint.

