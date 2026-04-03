# OpenVM RV32IM Circuit Constraint Analysis

This document records the results of a manual constraint audit of the OpenVM `extensions/rv32im/circuit/src/` chips, examining each for under-constrained (invalid trace passes) or over-constrained (valid trace fails) issues.

---

## Chips Audited

| Chip | File | Result |
|------|------|--------|
| BaseAlu | `base_alu/core.rs` | No finding |
| Shift | `shift/core.rs` | No finding |
| LessThan | `less_than/core.rs` | No finding |
| BranchEqual | `branch_eq/core.rs` | No finding |
| BranchLessThan | `branch_lt/core.rs` | No finding |
| JalLui | `jal_lui/core.rs` | No finding |
| Jalr | `jalr/core.rs` | No finding |
| LoadStore | `loadstore/core.rs` | Observation (see below) |
| LoadSignExtend | `load_sign_extend/core.rs` | No finding |
| MulH | `mulh/core.rs` | No finding |
| DivRem | `divrem/core.rs` | No finding |
| Auipc | `auipc/core.rs` | No finding |
| LoadStore Adapter | `adapters/loadstore.rs` | No finding |
| JALR Adapter | `adapters/jalr.rs` | No finding |

---

## Finding OVM-1 (Observation): LoadStore `is_load` lacks an explicit boolean constraint

**Type**: Design concern (not exploitable given current encoding)
**File**: `extensions/rv32im/circuit/src/loadstore/core.rs`, lines 141–146

### Description

The `is_load` column is constrained only by:

```rust
builder.assert_eq(
    is_load,
    opcode_when(&[LoadW0, LoadHu0, LoadHu2, LoadBu0, LoadBu1, LoadBu2, LoadBu3]),
);
builder.when(is_load).assert_one(is_valid);
```

There is no explicit `builder.assert_bool(is_load)`. The struct documentation claims:

```rust
/// is_load is constrained to be bool, and can only be 1 if is_valid is 1
pub is_load: T,
```

In practice `is_load` is implicitly boolean because `opcode_when(...)` evaluates to either 0 or 1 for any valid combination of the `flags` columns (each flag ∈ {0,1,2}, sum ∈ {0,1,2}). No valid flag assignment makes `opcode_when(load ops)` take a value outside {0,1}.

However, the missing `assert_bool` means that a future change to the flag-encoding scheme—or a subtle algebraic manipulation that forces `opcode_when(...)` to a non-boolean value—would not be caught by an explicit constraint. The safeguard is entirely implicit.

### Why not immediately exploitable

For the current encoding:
- Each `flag ∈ {0,1,2}` (enforced by `flag * (flag−1) * (flag−2) = 0`)
- `sum ∈ {0,1,2}` (enforced similarly)
- The 14 InstructionOpcode indices are uniquely mapped to disjoint expressions of `flags`; at most one `opcode_flags[i]` is 1 for any valid assignment

Therefore `opcode_when(load ops)` is always in {0,1}, and `is_load` is effectively boolean.

### Recommendation

Add an explicit boolean constraint for clarity and robustness:

```rust
builder.assert_bool(is_load);
```

---

## Design Observation: Non-standard {0,1,2} Flag Encoding in LoadStore

**File**: `extensions/rv32im/circuit/src/loadstore/core.rs`, lines 110–133

The LoadStore chip uses a novel 3-valued flag encoding: each `flag[i] ∈ {0,1,2}` and `sum ∈ {0,1,2}`. This compactly encodes 14 distinct (opcode, shift) pairs:

- `sum=2, flag[i]=2` → 4 single-byte or single-halfword loads (LoadW0, LoadHu0, LoadHu2, LoadBu0)
- `sum=1, flag[i]=1` → 4 store or extended-shift loads (LoadBu1, LoadBu2, LoadBu3, StoreW0)
- `sum=2, flag[i]=flag[j]=1` → 6 multi-byte stores (StoreH0, StoreH2, StoreB0–B3)

The encoding is mathematically sound: the three derived expressions
```rust
flag*(flag-1)*inv_2          // nonzero only when flag=2
flag*(sum-2)*(-1)            // nonzero only when flag=1 and sum=1
flags[i]*flags[j]            // product of two 1-valued flags
```
are pairwise disjoint across all 14 opcodes, so exactly one `opcode_flags[k] = 1` for every valid instruction row.

This is correct but unusual. Any future maintenance must preserve the mutual exclusivity of these expressions.

---

## Design Observation: JALR and LoadStore `imm_sign` Constrained via Program Bus, Not Core

**Files**: `jalr/core.rs` line 137; `adapters/jalr.rs` line 165; `adapters/loadstore.rs` line 352

In the JALR core:
```rust
builder.assert_bool(imm_sign);
```

`imm_sign` is constrained only to be boolean in the core chip. Its actual value—whether it matches the sign bit of the 12-bit immediate field—is enforced through the execution bridge program bus lookup, which includes `imm_sign` as instruction field 6.

This means the constraint is distributed across two components (core + adapter). While sound in the full system, an audit that examines only the core chip would incorrectly conclude that `imm_sign` is freely chosen.

### Why this is sound

The execution bridge fires with multiplicity `is_valid` and performs a lookup against the program bus, which stores all instruction fields as encoded in the compiled program. A prover cannot forge `imm_sign` without failing this lookup.

---

## Key Constraint Patterns Verified as Sound

### BranchEqual: Inverse-based equality proof
```
cmp_eq + Σ_i((a[i]-b[i]) * inv_marker[i]) = 1   when is_valid
cmp_eq * (a[i]-b[i]) = 0   for all i
```
Correctly proves equality/inequality without allowing false witnesses.

### BranchLessThan / LessThan: First-differing-limb scheme
The `diff_marker` prefix-sum pattern combined with `range_check(diff_val-1, ...)` ensures:
- At most one marker per row (prefix_sum ∈ {0,1})
- The marker is at the MOST significant differing limb (earlier limbs forced equal by `not(prefix_sum)*diff=0`)
- The differing limb is truly non-zero (`diff_val ≥ 1` from range check)
- When equal (prefix_sum=0): `cmp_lt=0` is forced

### DivRem: Zero-divisor and zero-remainder interlocking
The constraints `assert_bool(zero_divisor + r_zero)` and `assert_bool(is_valid - zero_divisor)` correctly enforce:
- At most one special case active at a time
- `is_valid=0` implies no special case flags
- `c=0 ↔ zero_divisor=1` enforced via `c_sum * c_sum_inv = 1` when not zero-divisor

### LoadSignExtend: Shift alignment constraint
The range check `(mem_ptr_limbs[0] - shift_amount) * inv_4 ∈ [0, 2^14)` correctly enforces `mem_ptr % 4 = shift_amount`, which ties the `shift_most_sig_bit` witness column to the actual address alignment.

### JALR: to_pc LSB clearing
The two-limb carry decomposition (`rs1 + sign_extend(imm) = 2*to_pc_limbs[0] + to_pc_lsb + to_pc_limbs[1]*2^16`) correctly implements RISC-V's LSB-clearing rule, and the range checks on both limbs prevent `to_pc` overflow.

---

## Summary

No exploitable under-constrained or over-constrained issues were found in the OpenVM RV32IM circuit chips. The single design observation (OVM-1) regarding the implicit boolean constraint on `is_load` does not constitute a vulnerability under the current encoding but warrants an explicit `assert_bool` call for robustness and clarity.
