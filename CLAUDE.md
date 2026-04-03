# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.


**Never use `--debug` mode of cargo. It creats too huge binaries. Always use `--release` mode**

## Project Overview

**ZEBRA** (Zero-knowledge zkVM Bounded Refinement Analysis) is a lattice-based abstract-interpretation verifier for zero-knowledge VMs (zkVMs). It formally verifies zkVM execution traces by performing interval analysis over AIR (Algebraic Intermediate Representation) constraints.

## Commands

### Build and Test (Main Library)
```bash
# Build
cargo build --release

# Run all library unit tests
cargo test --lib

# Run a single test by name
cargo test --lib <test_name>
```

### Example Crates
Each example is an independent Cargo workspace under `examples/{vm_name}/`:
```bash
cd examples/ziren
cargo build --release --example addsub
cargo test --test test_cpu

cd examples/sp1
cargo build --release
cargo test --test test_cpu
```

### Running Verification
```bash
./target/release/examples/<example> --config ./configs/alu.config --opcode-str "<OP>" --method "bb" --num-trial 1
```

### SMT Verification
Each example has scripts in `examples/{name}/scripts/`:
- `build.sh` — build examples
- `test.sh` — run operation tests
- `verify.sh` — run verification pipeline
- `smt_verify.sh` — verify via SMT solver

## Architecture

### Core Data Flow
1. **Input**: Abstract trace initialized with interval-valued columns (representing ranges of possible values)
2. **Constraint extraction**: Symbolic constraints are extracted from the zkVM's AIR definition
3. **Refinement loop** (`solver.rs`): Iteratively evaluate constraints → refine intervals → propagate backward
4. **Output**: Soundness verdict (satisfies / violates all constraints)

### Key Abstractions

| Module | Role |
|--------|------|
| `interval.rs` | `AbstractInterval [lo, hi]` — range arithmetic over u32 values; three-valued `MayBeFlag` (True/False/MayBe) |
| `symbolic.rs` | `ZEBRASymbolicExpr` — symbolic expression trees; `RangeType` domain specs (Bool, U4, U8, U16, Top, Const, Any) |
| `trace.rs` | `AbstractTrace` — execution trace as a table of symbolic intervals; tracks singleton positions |
| `state.rs` | `AbstractState` — abstract VM state (clock, PC, memory, completion flag) |
| `constraint.rs` | `eval_constraints()` — evaluates symbolic constraints over an abstract trace in strict or relaxed mode |
| `solver.rs` | `run_parallel_solver()` — main verification engine; interval refinement and constraint propagation loop |
| `wordop.rs` | 32-bit word ops (ADD, SUB, MUL, DIV, AND, OR, XOR, LT, SLT, SRL, EQ, NEQ, …) with interval semantics |
| `controlflowop.rs` | Branching and jump operations |
| `memory.rs` | `IntervalMemory` — interval-based address/value memory model |
| `smt.rs` | Converts symbolic constraints to SMT-LIB2 format for external solver verification |
| `shrinker.rs` | Detects selector-gated assignments and ABIR constraints; drives interval refinement |
| `quick.rs` | CLI entry point / experimental harness (`clap`-based) |
| `ui.rs` | Terminal UI via `ratatui` for live verification status |
| `canonicalizer.rs` | Symbolic expression canonicalization |

### Example Integrations (`examples/`)
Each subdirectory is a self-contained Cargo crate integrating ZEBRA with a specific zkVM backend:
- **ziren** — Zkm/RISC-V CPU (most complete; includes ALU config and CPU integration tests)
- **sp1** — Succinct Proof System 1 (uses custom SP1 fork)
- **pico** — PicoVM (uses custom Pico fork with Plonky3/Brevis)
- **valida** — Valida VM

Integration tests in each example (`tests/test_cpu.rs`) verify constraint satisfaction for individual CPU operations (BEQ, BNE, BGE, ADD, SUB, …).

### Known Failing Tests
Four tests in `interval.rs` currently fail with "attempt to shift left with overflow":
- `test_bitwise_and_soundness`
- `test_bitwise_or_soundness`
- `test_bitwise_xor_soundness`
- `test_specific_edge_cases`
