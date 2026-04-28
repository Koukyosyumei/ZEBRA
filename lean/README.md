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
└── Zebra/
    └── Canonicalizer/
        └── ALU.lean          — multi-row canonicalizer faithfulness
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
lake env lean Zebra/Canonicalizer/ALU.lean
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

`lake env lean Zebra/Canonicalizer/ALU.lean` should exit 0 with no output. To
confirm there are no admitted gaps:

```bash
grep -nE "sorry|admit|axiom" Zebra/Canonicalizer/ALU.lean
```

Expected: no matches.

## Theorem index — `Zebra.ALU` namespace

**Definitions**

| Name | Type | What it is |
|---|---|---|
| `canonicalize` | `Config → Trace → Finset Tuple` | Per-row projection of real rows, deduped (the canonical form). |
| `ALUEvent` | `Type → Type` | Concrete ALU event with concrete input/output values. |
| `EventSet` | `Type → Type` | Semantic ALU events, distinct from the canonicalized tuple set. |
| `TableEncodesEvents` | `Config → (ALUEvent Value → Tuple) → EventSet Value → Trace → Prop` | A generated table's real projected rows are exactly `Finset.image eventToTuple events`. |
| `TableGeneratorFaithful` | `Prop` | Abstract contract: `generateTable events` encodes `Finset.image eventToTuple events`. |
| `generatedTableOfExecution` | `VMExecution → Trace` | Optional wrapper: generate a table from `execEvents exec`. |
| `stringRepr` | `Config → Trace → String` | Printer faithful to `cr_add` / `PrettySet::fmt` (`noncomputable`). |

**Main theorems**

| Name | Statement |
|---|---|
| `canonicalize_generated_table_eq_eventTupleSet` | Faithful `EventSet → Trace` generator ⇒ canonicalized generated table equals `Finset.image eventToTuple events`. |
| `canonicalize_generated_eq_iff_eventTupleSet_eq` | Faithful `EventSet → Trace` generator ⇒ equal canonical forms iff tuple images of event sets are equal. |
| `canonicalize_generated_eq_iff_events_eq_of_eventToTuple_injective` | If `eventToTuple` is injective, the previous theorem strengthens to equality of full event sets. |

**Support lemmas**

| Name | Statement |
|---|---|
| `mem_canonicalize_iff` | `tup ∈ canonicalize cfg t ↔ ∃ row ∈ t, isReal row ∧ projectRow row = tup` |
| `canonicalize_eq_eventTupleSet_of_encodes` | If a table encodes VM events, canonicalization returns `Finset.image eventToTuple events`. |
| `canonicalize_eq_eventTupleSet_iff` | `canonicalize cfg table = Finset.image eventToTuple events ↔ TableEncodesEvents cfg eventToTuple events table` |
| `canonicalize_execution_table_eq_eventTupleSet` | Optional execution wrapper: faithful generator ⇒ canonicalized execution table equals `Finset.image eventToTuple (execEvents exec)`. |
| `canonicalize_execution_tables_eq_iff_eventTupleSet_eq` | Optional execution wrapper: equal canonical forms iff execution tuple images are equal. |

**Lemmas (helpers / consequences)**

| Name | Statement |
|---|---|
| `canonicalize_nil` | `canonicalize cfg [] = ∅` |
| `canonicalize_no_real` | no real rows ⇒ `canonicalize cfg t = ∅` |
| `canonicalize_append` | distributivity over `++`: `= canonicalize t₁ ∪ canonicalize t₂` |
| `canonicalize_append_padding` | non-real row appended ⇒ canonical form unchanged |
| `canonicalize_perm` | row permutation ⇒ canonical form unchanged |
| `stringRepr_consistent` | equal canonical forms ⇒ equal strings (printer's forward direction) |

## What the proofs do *not* cover

- **String-level injectivity** (`stringRepr cfg t₁ = stringRepr cfg t₂ ⇒
  canonicalize cfg t₁ = canonicalize cfg t₂`). This is a printer-correctness
  property, not a canonicalizer property — would require nested-bracket parser
  injectivity for `renderLimbs`. Tractable under a singleton-interval
  hypothesis (realistic for AddSub recovered states); unaddressed at present.
- **AIR-soundness for AddSub**. The proofs here treat the canonicalizer in
  isolation; relating the canonical form to the algebraic AIR semantics
  (i.e., proving `a = b ± c mod 2³²`) is a separate vertical slice.
