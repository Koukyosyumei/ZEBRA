# ZEBRA Lean Proofs

This directory contains the Lean 4 formalization for the ZEBRA canonicalizer.
It proves that canonicalization quotients raw trace-table representations by
their canonical form and, under the stated generator assumptions, is one-to-one
with semantic record sets.

## Layout

```text
lean/
├── Zebra.lean
└── Zebra/Canonicalizer/
    ├── Generic.lean
    ├── Quotient.lean
    ├── Generator.lean
    ├── ALU.lean
    ├── CPU.lean
    ├── Memory.lean
    ├── ControlFlow.lean
    └── Examples.lean
```

## Build and Check

Prerequisites: Lean 4 via `elan` and the toolchain specified in
`lean-toolchain`.

```bash
lake build
grep -R -nE "sorry|admit|axiom" Zebra/
```

The expected result is a successful build with no matches from the grep check.

## Main Statements

The central statements are:

| Statement | File | Purpose |
|---|---|---|
| `canonicalTraceSpace_equiv_image` | `Quotient.lean` | Establishes the quotient-space characterization of canonical tables |
| `canonicalize_generator_independent` | `Generator.lean` | Shows that faithful generators for the same record set produce the same canonical table |
| `canonicalize_generated_eq_iff_records_eq` | `Generator.lean` | Relates equality of generated canonical tables to equality of semantic record sets |
| `recordIds_generated_eq_iff_records_eq` | `Generator.lean` | Provides the tuple-extraction variant used by table-specific layouts |

`Examples.lean` instantiates these results for the artifact's zkVM table
layouts, including CPU, ALU, memory, branch/jump, and lookup-driven tables.
