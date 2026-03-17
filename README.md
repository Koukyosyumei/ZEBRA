# ZEBRA

**ZEBRA** is a lattice-based abstract-interpretation verifier for zero-knowledge virtual machines (zkVMs). It formally verifies zkVM execution traces by performing interval analysis over AIR (Algebraic Intermediate Representation) constraints, without requiring concrete witness generation.

---

## Overview

zkVM correctness hinges on the soundness of its constraint systems. ZEBRA provides a lightweight, automated way to check whether a zkVM's AIR constraints can be satisfied by abstract (interval-valued) execution traces. Rather than enumerating concrete witnesses, ZEBRA abstracts over entire ranges of possible values and propagates interval refinements until a fixed point is reached.

**Key capabilities:**

- **Interval-based abstraction** — represents all possible values in a column as `[lo, hi]` ranges, enabling analysis over unbounded witness sets
- **Three-valued verdict** — outputs `Satisfies`, `Violates`, or `MayBe` (inconclusive) for each constraint set
- **SMT export** — emits SMT-LIB2 formulas for external solver verification (Z3, CVC5)
- **Multi-backend support** — integrates with Ziren (RISC-V), SP1, Pico, and Valida zkVM backends
- **Live terminal UI** — real-time verification status via `ratatui`.

---

## Architecture

```
Input: Interval-valued Abstract Trace
           │
           ▼
   Constraint Extraction
   (AIR symbolic expressions)
           │
           ▼
   ┌───────────────────┐
   │   Refinement Loop │◄──────────┐
   │                   │           │
   │  eval_constraints()           │
   │  → detect uncertain vars      │
   │  → shrinker analysis          │
   │  → refine_trace()             │
   └───────────┬───────┘           │
               │  converged?  ─────┘
               ▼
         Soundness Verdict
    (Satisfies / Violates / MayBe)
```

### Module Map

| Module | Role |
|---|---|
| `interval.rs` | `AbstractInterval [lo, hi]` — range arithmetic over u32 values; three-valued `MayBeFlag` |
| `symbolic.rs` | `ZEBRASymbolicExpr` — symbolic expression trees; `RangeType` domain specs (Bool, U4, U8, U16, Top, Const, Any) |
| `trace.rs` | `AbstractTrace` — execution trace as a table of symbolic intervals; singleton position tracking |
| `state.rs` | `AbstractState` — abstract VM state (clock, PC, memory, completion flag) |
| `constraint.rs` | `eval_constraints()` — evaluates symbolic constraints in strict or relaxed mode |
| `solver.rs` | `run_parallel_solver()` — main verification engine; iterative interval refinement loop |
| `wordop.rs` | 32-bit word operations (ADD, SUB, MUL, DIV, AND, OR, XOR, LT, SLT, SRL, EQ, NEQ, …) with interval semantics |
| `controlflowop.rs` | Branching and jump operations |
| `memory.rs` | `IntervalMemory` — interval-based address/value memory model |
| `shrinker.rs` | Detects selector-gated assignments and ABIR constraints; drives interval shrinking |
| `smt.rs` | Converts symbolic constraints to SMT-LIB2 format for external solver verification |
| `canonicalizer.rs` | Symbolic expression canonicalization |
| `quick.rs` | CLI entry point (`clap`-based); experiment harness with YAML output |
| `ui.rs` | Terminal UI via `ratatui` for live verification status |

---

## Getting Started

### Prerequisites

- Rust toolchain (stable, 1.70+)
- Cargo

### Build

```bash
# Clone the repository
git clone https://github.com/Koukyosyumei/TwinVM.git
cd TwinVM

# Build the main library
cargo build --release
```

### Run Tests

```bash
# Run all unit tests
cargo test --lib

# Run a specific test by name
cargo test --lib <test_name>
```

---

## zkVM Integrations

ZEBRA currently supports four zkVM backends. Each integration is an independent Cargo workspace under `examples/{vm_name}/`.

### Ziren (RISC-V / Zkm)

The most complete integration, supporting a full RISC-V ALU and control flow constraint set.

**Supported operations:** ADD, SUB, XOR, AND, OR, BEQ, BNE, BGE, BGEZ, BGTZ, BLEZ, BLTZ, LT, SLT, MUL, DIV, JUMP, JUMPI, WSBH, ShiftLeft, MEQ, MNE

```bash
cd examples/ziren

# Build all example binaries
cargo build --release --example addsub

# Run CPU integration tests
cargo test --test test_cpu
```

### SP1

Integration with the Succinct Proof System 1 zkVM.

```bash
cd examples/sp1
cargo build --release
cargo test --test test_cpu
```

### Pico

Integration with PicoVM using Plonky3/Brevis.

```bash
cd examples/pico
cargo build --release
cargo test --test test_cpu
```

### Valida

Integration with the Valida VM.

```bash
cd examples/valida
cargo build --release
```

---

## Running Verification

Each example binary accepts a common CLI interface:

```bash
./target/release/examples/<example> \
  --config ./configs/alu.config \
  --opcode-str "<OP>" \
  --method "bb" \
  --num-trial <N>
```

**Example — verify ADD in Ziren over 30 trials:**

```bash
cd examples/ziren
cargo build --release --example addsub
./target/release/examples/addsub \
  --config ./configs/alu.config \
  --opcode-str "ADD" \
  --method "bb" \
  --num-trial 30 \
  --output-path "report/add.yaml"
```

### Configuration

Each example reads a JSON config (`configs/alu.config`):

```json
{
    "time_out_ms": 1000000,
    "num_workers": 1
}
```

| Field | Description |
|---|---|
| `time_out_ms` | Per-trial solver timeout in milliseconds |
| `num_workers` | Parallel worker threads for the refinement loop |

### Scripts

Each example ships four automation scripts under `examples/{name}/scripts/`:

| Script | Purpose |
|---|---|
| `build.sh` | Build all example binaries |
| `test.sh` | Run operation-level tests |
| `verify.sh` | Run the full verification pipeline |
| `smt_verify.sh` | Verify using an external SMT solver (Z3/CVC5) |

---

## SMT Verification

ZEBRA can export constraints to SMT-LIB2 format for verification by external solvers:

```bash
cd examples/ziren
bash scripts/smt_verify.sh
```

This invokes the `smt.rs` module to serialize symbolic AIR constraints and calls Z3 or CVC5 to check satisfiability.

---

## Core Concepts

### AbstractInterval

Values are represented as closed integer intervals `[lo, hi]`. Arithmetic and bitwise operations over intervals are implemented with sound over-approximation — any concrete value reachable within the interval bounds is accounted for.

```
[2, 5] + [1, 3] = [3, 8]
[0, 15] & [4, 7] = [0, 7]
```

### MayBeFlag (Three-Valued Logic)

Comparisons and boolean constraints return one of three values:

| Value | Meaning |
|---|---|
| `True` | Holds for all concrete values in the interval |
| `False` | Holds for no concrete values in the interval |
| `MayBe` | Holds for some but not all — inconclusive |

### Refinement Loop

ZEBRA iteratively tightens interval bounds by:

1. Evaluating all AIR constraints symbolically over the current abstract trace
2. Identifying columns where a constraint forces a narrower range
3. Applying `refine_trace()` to shrink those intervals
4. Repeating until no further refinement is possible (fixed point) or timeout

### Selector-Gated Assignments

The `shrinker` module detects the pattern:

```
selector * (lhs - rhs) = 0
```

When `selector` is a singleton (concrete value), the shrinker propagates the implied equality `lhs = rhs`, enabling targeted interval refinement.

---

## Known Limitations

The following interval tests currently fail due to a shift-left overflow edge case:

- `test_bitwise_and_soundness`
- `test_bitwise_or_soundness`
- `test_bitwise_xor_soundness`
- `test_specific_edge_cases`

---

## License

This project is licensed under the **Apache 2.0** license. See [LICENSE](LICENSE) for details.

---

## Contributing

Contributions are welcome. Please open an issue or pull request on [GitHub](https://github.com/Koukyosyumei/TwinVM).
