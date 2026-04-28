# Zebra — Lean 4 proofs

Formal-verification companion for the Rust crates above. Currently scopes the
canonicalizer faithfulness theorems for the multi-row ALU/memory-op pattern
(`src/canonicalizer.rs:56`, `:90`).

## Layout

```
lean/
├── lakefile.lean             — Lake config; depends on mathlib
├── lean-toolchain            — pins leanprover/lean4:v4.24.0
├── lake-manifest.json        — locked dependency revisions
├── Zebra.lean                — import aggregator for `lake build`
└── Zebra/
    └── Canonicalizer/
        ├── Generic.lean      — reusable theorem layer
        ├── ALU.lean          — ALU tuple/event wrappers + string rendering
        ├── Memory.lean       — memory-op canonical representation shape
        ├── ControlFlow.lean  — control-flow and misc representation shapes
        └── Examples.lean     — concrete zkVM example layouts
```

## Prerequisites

- [`elan`](https://github.com/leanprover/elan) (Lean's toolchain manager).
  Installs `lean` and `lake` automatically.
- The toolchain pinned in `lean-toolchain` (`leanprover/lean4:v4.24.0`) will be
  fetched on first invocation if not already installed.

## Build

### Option A — fresh build (downloads mathlib)

From `lean/`:

```bash
lake update              # fetches mathlib + transitive deps (~5 min, multi-GB)
lake exe cache get       # downloads precompiled mathlib .olean files (recommended;
                         # avoids ~30 min of mathlib compilation)
lake build               # compiles Zebra/
```

Or to type-check a single file:

```bash
lake env lean Zebra/Canonicalizer/Examples.lean
```

Exit code 0 with no output = clean build.

### Option B — share an existing mathlib build (faster)

If you already have a Lean 4.24.0 project with mathlib built locally (e.g.
under `~/Dev/SomeProject/.lake/packages/mathlib`), symlink its packages
directory into ours to skip download/compile:

```bash
mkdir -p .lake/packages
for pkg in /path/to/other/project/.lake/packages/*; do
  ln -sf "$pkg" .lake/packages/
done
lake env lean Zebra/Canonicalizer/ALU.lean
```

The current checked-in `lake-manifest.json` was bootstrapped this way (mathlib
rev `3bde4584...`). If your local mathlib is at a different revision, run
`lake update` to refresh the manifest.

## Verifying the proofs

`lake build` should complete successfully. To confirm there are no admitted
gaps:

```bash
grep -R -nE "sorry|admit|axiom" Zebra/
```

Expected: no matches.

## Theorem index

### `Zebra.Generic`

Reusable theorem layer for any table whose canonical representation is a
deduplicated set of projected real rows.

| Name | Type | What it is |
|---|---|---|
| `Config` | `Type → Type` | Generic table layout: real-row predicate plus row projection. |
| `canonicalize` | `Config Repr → Trace → Finset Repr` | Project real rows to table-specific canonical representations. |
| `EventEncoding` | `Type → Type → Type` | Injective representation of semantic events as canonical row representations. |
| `TableGeneratorFaithful` | `Prop` | Abstract contract: generated tables encode exactly their source event sets. |
| `canonicalize_generated_table_eq_eventReprSet` | theorem | Faithful generator ⇒ canonicalized table equals encoded event set. |
| `canonicalize_generated_eq_iff_events_eq` | theorem | Faithful generator ⇒ equal canonical forms iff original event sets are equal. |

This layer can be reused for ALU, control-flow, and memory-op tables by choosing
the appropriate `Repr` type.

### `Zebra.ALU`

**Definitions**

| Name | Type | What it is |
|---|---|---|
| `canonicalize` | `Config → Trace → Finset Tuple` | Per-row projection of real rows, deduped (the canonical form). |
| `ALUEvent` | `Type → Type` | Concrete ALU event with concrete input/output values. |
| `EventSet` | `Type → Type` | Semantic ALU events, distinct from the canonicalized tuple set. |
| `ALUEventEncoding` | `Type → Type` | Injective representation of concrete ALU events as canonical tuples. |
| `TableEncodesEvents` | `Config → ALUEventEncoding Value → EventSet Value → Trace → Prop` | A generated table's real projected rows are exactly the encoded event set. |
| `TableGeneratorFaithful` | `Prop` | Abstract contract: `generateTable events` encodes exactly `events`. |
| `generatedTableOfExecution` | `VMExecution → Trace` | Optional wrapper: generate a table from `execEvents exec`. |
| `stringRepr` | `Config → Trace → String` | Printer faithful to `cr_add` / `PrettySet::fmt` (`noncomputable`). |

### `Zebra.Memory` / `Zebra.ControlFlow`

| Name | Type | What it is |
|---|---|---|
| `MemoryOpTuple` | `Type` | Canonical representation shape for memory read/write rows. |
| `mkMemoryOpConfig` | constructor | Build a generic config from memory-op column groups. |
| `ControlFlowTuple` | `Type` | Canonical representation shape for branch/jump rows. |
| `UnaryTuple` | `Type` | Canonical representation shape for unary ALU-like rows such as CLO/CLZ. |
| `MovCondTuple` | `Type` | Canonical representation shape for Ziren movcond rows. |
| `ValidaLtTuple` | `Type` | Canonical representation shape for Valida LT32. |

**Main theorems**

| Name | Statement |
|---|---|
| `canonicalize_generated_table_eq_eventTupleSet` | Faithful `EventSet → Trace` generator ⇒ canonicalized generated table equals the encoded event set. |
| `canonicalize_generated_eq_iff_events_eq` | Faithful `EventSet → Trace` generator ⇒ equal canonical forms iff original event sets are equal. |

**Support lemmas**

| Name | Statement |
|---|---|
| `mem_canonicalize_iff` | `tup ∈ canonicalize cfg t ↔ ∃ row ∈ t, isReal row ∧ projectRow row = tup` |
| `canonicalize_eq_eventTupleSet_of_encodes` | If a table encodes VM events, canonicalization returns their encoded tuple set. |
| `canonicalize_eq_eventTupleSet_iff` | `canonicalize cfg table = Finset.image enc.toTuple events ↔ TableEncodesEvents cfg enc events table` |
| `canonicalize_execution_table_eq_eventTupleSet` | Optional execution wrapper: faithful generator ⇒ canonicalized execution table equals the encoded execution event set. |
| `canonicalize_execution_tables_eq_iff_events_eq` | Optional execution wrapper: equal canonical forms iff execution event sets are equal. |

**Lemmas (helpers / consequences)**

| Name | Statement |
|---|---|
| `canonicalize_nil` | `canonicalize cfg [] = ∅` |
| `canonicalize_no_real` | no real rows ⇒ `canonicalize cfg t = ∅` |
| `canonicalize_append` | distributivity over `++`: `= canonicalize t₁ ∪ canonicalize t₂` |
| `canonicalize_append_padding` | non-real row appended ⇒ canonical form unchanged |
| `canonicalize_perm` | row permutation ⇒ canonical form unchanged |
| `stringRepr_consistent` | equal canonical forms ⇒ equal strings (printer's forward direction) |

**Example table layouts**

Concrete example configs are collected under `Zebra.ALU.Examples`:

| VM | Examples |
|---|---|
| `SP1` | add/sub, branch/jump control-flow, memory instructions, lookup-driven ALU tables (`lt`, `bitwise`, `divrem`, `shiftleft`, `mul`, `sr`) |
| `Pico` | add/sub, memory read/write, lookup-driven ALU tables (`sr`, `sll`, `lessthan`, `mul`, `bitwise`, `divrem`) |
| `Sphinx` | add/sub, lookup-driven ALU tables (`sr`, `shiftleft`, `mul`, `lt`, `bitwise`, `divrem`) |
| `Ziren` | add/sub, div/rem, CLO/CLZ, movcond, branch/jump control-flow, memory instructions, lookup-driven ALU tables (`mul`, `shiftleft`, `shiftright`, `lt`, `bitwise`) |
| `Valida` | LT32 and lookup-driven ALU tables (`add32`, `sub32`, `mul32`, `div32`, `bitwise32`, `com32`) |
| `OpenVM` | lookup-driven ALU tables (`alu`, `bitwise`, `branch`, `jump`, `lt`, `mul`, `shift`) |

CPU tables and Valida memory are intentionally not modeled here because their
canonicalizers are not currently supported by Zebra.

## What the proofs do *not* cover

- **String-level injectivity** (`stringRepr cfg t₁ = stringRepr cfg t₂ ⇒
  canonicalize cfg t₁ = canonicalize cfg t₂`). This is a printer-correctness
  property, not a canonicalizer property — would require nested-bracket parser
  injectivity for `renderLimbs`. Tractable under a singleton-interval
  hypothesis (realistic for AddSub recovered states); unaddressed at present.
- **AIR-soundness for AddSub**. The proofs here treat the canonicalizer in
  isolation; relating the canonical form to the algebraic AIR semantics
  (i.e., proving `a = b ± c mod 2³²`) is a separate vertical slice.
