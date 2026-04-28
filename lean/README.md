# Zebra Lean Proofs

Lean 4 proofs for Zebra canonicalizer correctness. The core result is generic:
for any table whose canonical form is a finite set of projected real rows,
canonicalization is one-to-one with the original event set, assuming a faithful
table generator and an injective event encoding.

## Layout

```text
lean/
├── Zebra.lean
└── Zebra/Canonicalizer/
    ├── Generic.lean      # reusable theorem layer
    ├── ALU.lean          # ALU tuple canonicalizer
    ├── Memory.lean       # memory-op tuple shape
    ├── ControlFlow.lean  # control-flow and misc tuple shapes
    └── Examples.lean     # zkVM layouts and theorem instantiations
```

## Verify

From `lean/`:

```bash
lake build
grep -R -nE "sorry|admit|axiom" Zebra/
```

Expected: build succeeds and grep returns no matches.

## Main Theorem

In `Zebra.Generic`:

```lean
canonicalize_generated_eq_iff_events_eq
```

states that if:

- `cfg` defines real rows and row projection,
- `enc` injectively maps semantic events to canonical row representations,
- `generateTable` faithfully encodes each event set,

then:

```lean
canonicalize cfg (generateTable events₁) =
canonicalize cfg (generateTable events₂)
↔
events₁ = events₂
```

This proves one-to-one correctness for the canonicalizer relative to those
assumptions.

## Covered Layouts

`Examples.lean` instantiates the theorem for supported example layouts. Static
layouts have concrete column-index configs. Lookup-driven ALU layouts use
`ExtractedALULayout`, modeling the `GeneralLookupInfo` columns extracted by the
Rust pipeline (`is_real`, `op_b`, `op_c`, `op_a`).

- SP1: add/sub, branch/jump, memory instructions, lookup-driven ALU tables
- Pico: add/sub, memory read/write, lookup-driven ALU tables
- Sphinx: add/sub, lookup-driven ALU tables
- Ziren: add/sub, div/rem, CLO/CLZ, movcond, branch/jump, memory instructions, lookup-driven ALU tables
- Valida: LT32 and lookup-driven ALU tables
- OpenVM: lookup-driven ALU tables

CPU tables and Valida memory are intentionally skipped.

## Non-Claims

These proofs do not prove:

- Rust table generators are faithful.
- Event encodings used by a real VM are injective.
- AIR constraints are sound.
- ALU arithmetic semantics are correct.
- String rendering is injective.
