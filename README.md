# ZEBRA

ZEBRA is an anonymized conference-submission artifact for localized verification of zkVM constraints using interval abstraction and refinement.

The artifact contains:

- the Rust verifier implementation,
- zkVM integration examples under `examples/`,
- SMT-LIB export support for external solver checks, and
- Lean 4 proofs for the canonicalizer component under `lean/`.

## Repository Layout

| Path | Contents |
|---|---|
| `src/` | Core interval domain, symbolic expressions, traces, refinement, SMT export, and terminal UI |
| `examples/ziren/` | Ziren/RISC-V integration |
| `examples/sp1/` | SP1 integration |
| `examples/pico/` | Pico integration |
| `examples/sphinx/` | Sphinx integration |
| `examples/valida/` | Valida integration |
| `lean/` | Lean 4 formalization of canonicalizer correctness |

## Rust Build and Tests

Prerequisites: Rust stable and Cargo.

```bash
cargo build --release
cargo test --lib
```

To run a single Rust test:

```bash
cargo test --lib <test_name>
```

## Example Integrations

Each zkVM integration is a separate Cargo workspace under `examples/`.
The common scripts are:

| Script | Purpose |
|---|---|
| `scripts/build.sh` | Build example binaries |
| `scripts/test.sh` | Run integration tests |
| `scripts/verify.sh` | Run the verification pipeline |
| `scripts/smt_verify.sh` | Run SMT-based checks where available |

Example:

```bash
cd examples/ziren
bash scripts/build.sh
bash scripts/test.sh
bash scripts/verify.sh
```

Individual example binaries accept a common interface:

```bash
./target/release/examples/<example> \
  --config ./configs/alu.config \
  --opcode-str "<OP>" \
  --method "bb" \
  --num-trial <N> \
  --output-path <report.yaml>
```

## Configuration

Example configurations are stored in each integration's `configs/` directory.
A typical configuration is:

```json
{
  "time_out_ms": 1000000,
  "num_workers": 1
}
```

| Field | Meaning |
|---|---|
| `time_out_ms` | Per-trial solver timeout in milliseconds |
| `num_workers` | Number of worker threads |

## Lean Proofs

The Lean formalization is in `lean/`.

```bash
cd lean
lake build
grep -R -nE "sorry|admit|axiom" Zebra/
```

The expected result is a successful build with no matches from the grep check.

## SMT Export

Where supported by an integration, SMT checks can be run with:

```bash
cd examples/<integration>
bash scripts/smt_verify.sh
```

The scripts emit SMT-LIB2 constraints and invoke an available external solver.
