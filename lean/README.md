# Zebra Lean Proofs

Lean 4 proofs for Zebra canonicalizer correctness. The proofs formalize:

1. Raw trace tables modulo canonicalizer equality are bijective with the image
   of the canonicalizer.
2. If a table generator faithfully encodes semantic events and the event
   encoding is injective, canonicalization is one-to-one with the event set.

## Layout

```text
lean/
├── Zebra.lean
└── Zebra/Canonicalizer/
    ├── Generic.lean      # core table canonicalizer
    ├── Quotient.lean     # D / ~R ≃ Im(R)
    ├── Generator.lean    # faithful-generator/event-set theorems
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

## Main Theorems

In `Zebra.Generic`:

```lean
canonicalTraceSpace_equiv_image
canonicalize_generator_independent
canonicalize_generated_eq_iff_events_eq
```

`canonicalTraceSpace_equiv_image` is in `Quotient.lean` and proves the
quotient-space statement:

```lean
Function.Bijective (quotientToImage cfg)
```

where the quotient identifies raw tables with equal canonical forms.

`canonicalize_generator_independent` is in `Generator.lean` and states that if:

- `gen₁` and `gen₂` are both faithful generators,
- both receive the same event set,

then their generated raw tables canonicalize to the same value. This formalizes
the collapse of different raw representations of the same computation.

`canonicalize_generated_eq_iff_events_eq` is in `Generator.lean` and states
that if:

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
