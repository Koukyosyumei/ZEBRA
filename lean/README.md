# Zebra Lean Proofs

Lean 4 proofs for Zebra canonicalizer correctness. The proofs formalize:

1. Raw trace tables modulo canonicalizer equality are bijective with the image
   of the canonicalizer.
2. If a table generator faithfully encodes semantic records and the record
   identity is injective, canonicalization is one-to-one with the record set.

## Layout

```text
lean/
├── Zebra.lean
└── Zebra/Canonicalizer/
    ├── Generic.lean      # core table canonicalizer
    ├── Quotient.lean     # D / ~R ≃ Im(R)
    ├── Generator.lean    # faithful-generator/record-set theorems
    ├── ALU.lean          # ALU tuple canonicalizer
    ├── CPU.lean          # CPU multi-record canonicalizer
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
canonicalize_generated_eq_iff_records_eq
recordIds_generated_eq_iff_records_eq
```

`canonicalTraceSpace_equiv_image` is in `Quotient.lean` and proves the
quotient-space statement:

```lean
Function.Bijective (quotientToImage cfg)
```

where the quotient identifies raw tables with equal canonical forms.

`canonicalize_generator_independent` is in `Generator.lean` and states that if:

- `gen₁` and `gen₂` are both faithful generators,
- both receive the same record set,

then their generated raw tables canonicalize to the same value. This formalizes
the collapse of different raw representations of the same computation.

`canonicalize_generated_eq_iff_records_eq` is in `Generator.lean` and states
that if:

- `cfg` defines real rows and row projection,
- `recordId` injectively maps semantic records to canonical row identities,
- `generateTable` faithfully encodes each record set,

then:

```lean
canonicalize cfg (generateTable records₁) =
canonicalize cfg (generateTable records₂)
↔
records₁ = records₂
```

This proves one-to-one correctness for the canonicalizer relative to those
assumptions.

`recordIds_generated_eq_iff_records_eq` is the variant used when a
table canonicalizer extracts a table-specific tuple, such as `ALU.Tuple`, and a
separate `tableId` function interprets that tuple as the abstract identity of a
semantic record.

## Covered Layouts

`Examples.lean` instantiates the theorem for supported example layouts:

- SP1: CPU, add/sub, branch/jump, memory instructions, lookup-driven ALU tables
- Pico: CPU, add/sub, memory read/write, lookup-driven ALU tables
- Sphinx: CPU, add/sub, lookup-driven ALU tables
- Ziren: CPU, add/sub, div/rem, CLO/CLZ, movcond, branch/jump, memory instructions, lookup-driven ALU tables
- Valida: LT32, memory, and lookup-driven ALU tables
- OpenVM: lookup-driven ALU tables
