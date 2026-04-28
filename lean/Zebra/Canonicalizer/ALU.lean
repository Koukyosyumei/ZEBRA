/-
Zebra — Canonicalizer faithfulness for ALU tables (multi-row).

Models the per-row projection canonicalizer pattern shared by ALU and
memory-op tables (`src/canonicalizer.rs:56` and `:90`). For each "real" row,
project the configured operand columns to a `(b, c, a)` tuple; the canonical
form is the deduplicated *set* of those projections (matching the Rust
`HashSet<…>` semantics).

Note on AddSub: the Rust code's `cr_add` (`examples/sp1/examples/addsub.rs:26`)
and `generate_alu_final_checker` (`src/canonicalizer.rs:72`) both hardcode row
0 for the projection regardless of which row triggered `is_real`. That is the
documented "currently only support one-row table" simplification. The semantic
model below uses per-row projection (matching the memory-op canonicalizer at
`src/canonicalizer.rs:111` and what the ALU canonicalizer should do once
generalized). Under "at most one real row," the two coincide.

The canonicalized representation is a `Finset Tuple` (a SET, not a string).
The string output is a separate printer over this set — trivially deterministic
in the forward direction; injectivity at the string level is a separate
"printer correctness" concern (see `stringRepr_consistent`).

Build: `lake build` (uses mathlib via local symlinks under `.lake/packages/`).
-/
import Mathlib.Data.Finset.Basic
import Mathlib.Data.Finset.Dedup
import Mathlib.Data.Finset.Lattice.Lemmas

namespace Zebra.ALU

/-! ## Trace, Tuple, Config -/

/-- An abstract interval over `Int` (mirrors `AbstractInterval` in `src/interval.rs`). -/
structure Interval where
  lo : Int
  hi : Int
deriving DecidableEq, Repr

/-- Display for `AbstractInterval` (`src/interval.rs:66`). -/
def Interval.repr (i : Interval) : String :=
  if i.lo = i.hi then toString i.lo
  else "[" ++ toString i.lo ++ ", " ++ toString i.hi ++ "]"

abbrev Row   := List Interval
abbrev Trace := List Row

/-- Project a row at the given column indices.
    Mirrors `trace_fmt_with_idxs` (`src/trace.rs:123`). -/
def Row.project (r : Row) (idxs : List Nat) : List Interval :=
  idxs.filterMap (fun j => r[j]?)

/-- The canonical operand tuple: byte-limb groups for inputs and result. -/
structure Tuple where
  b : List Interval
  c : List Interval
  a : List Interval
deriving DecidableEq, Repr

/-- Configuration: column indices for the three operand groups, plus the
    `is_real` predicate over a row. -/
structure Config where
  idxB   : List Nat
  idxC   : List Nat
  idxA   : List Nat
  isReal : Row → Bool

/-- The per-row projection: maps a row to its (b, c, a) tuple. -/
def Config.projectRow (cfg : Config) (row : Row) : Tuple :=
  { b := row.project cfg.idxB,
    c := row.project cfg.idxC,
    a := row.project cfg.idxA }

/-! ## (i) `canonicalize` — multi-row projection to a deduplicated set -/

/-- The canonical form of a trace: the deduplicated set of (b, c, a) tuples
    over its real rows. Faithfully models the multi-row pattern at
    `src/canonicalizer.rs:111` (memory-op) — each real row contributes its
    own projection, with `HashSet` deduplication. -/
def canonicalize (cfg : Config) (t : Trace) : Finset Tuple :=
  ((t.filter cfg.isReal).map cfg.projectRow).toFinset

/-! ## (ii) Substantive theorems -/

/-- **Membership characterization.** A tuple is in the canonical form of `t`
    iff it is the projection of some real row of `t`. This is the substantive
    "what the canonicalizer extracts" theorem — both directions. -/
theorem mem_canonicalize_iff (cfg : Config) (t : Trace) (tup : Tuple) :
    tup ∈ canonicalize cfg t ↔ ∃ row ∈ t, cfg.isReal row ∧ cfg.projectRow row = tup := by
  unfold canonicalize
  simp [List.mem_toFinset, List.mem_map, List.mem_filter]
  constructor
  · rintro ⟨row, ⟨hin, hreal⟩, heq⟩
    exact ⟨row, hin, hreal, heq⟩
  · rintro ⟨row, hin, hreal, heq⟩
    exact ⟨row, ⟨hin, hreal⟩, heq⟩

/-- Two traces are *canonically equivalent* iff their real-row projections
    coincide as a set. This is the equivalence relation under which the
    canonicalizer is one-to-one — designed to be insensitive to padding rows
    and to row permutations (see `canonicalize_append_padding`, `canonicalize_perm`). -/
def TraceEquiv (cfg : Config) (t₁ t₂ : Trace) : Prop :=
  ∀ tup, (∃ row ∈ t₁, cfg.isReal row ∧ cfg.projectRow row = tup) ↔
         (∃ row ∈ t₂, cfg.isReal row ∧ cfg.projectRow row = tup)

/-- **Bijection statement.** Two traces have equal canonical forms iff they
    are canonically equivalent. -/
theorem canonicalize_eq_iff (cfg : Config) (t₁ t₂ : Trace) :
    canonicalize cfg t₁ = canonicalize cfg t₂ ↔ TraceEquiv cfg t₁ t₂ := by
  rw [Finset.ext_iff]
  exact ⟨fun h tup => by simpa [mem_canonicalize_iff] using h tup,
         fun h tup => by simpa [mem_canonicalize_iff] using h tup⟩

/-- **Same → same.** Canonically equivalent traces yield equal canonical forms.
    (The "soundness" half of one-to-one: traces that ought to be considered the
    same — same set of real-row projections — produce the same output.) -/
theorem canonicalize_eq_of_equiv (cfg : Config) {t₁ t₂ : Trace}
    (h : TraceEquiv cfg t₁ t₂) :
    canonicalize cfg t₁ = canonicalize cfg t₂ :=
  (canonicalize_eq_iff cfg t₁ t₂).mpr h

/-- **Different → different.** Canonically *inequivalent* traces yield distinct
    canonical forms. (The "completeness" half of one-to-one: traces that
    genuinely differ in their real-row projections produce different outputs.

    NOTE: this is stated on `TraceEquiv`, not raw trace equality. The naive
    claim "`t₁ ≠ t₂ → canonicalize cfg t₁ ≠ canonicalize cfg t₂`" is *false* by
    design — appending a non-real padding row to a trace yields a different
    list but the same canonical form, and that's exactly the intended
    behaviour.) -/
theorem canonicalize_ne_of_not_equiv (cfg : Config) {t₁ t₂ : Trace}
    (h : ¬ TraceEquiv cfg t₁ t₂) :
    canonicalize cfg t₁ ≠ canonicalize cfg t₂ :=
  fun heq => h ((canonicalize_eq_iff cfg t₁ t₂).mp heq)

/-! ## (iii) Helper lemmas -/

/-- Empty traces canonicalize to the empty set. -/
lemma canonicalize_nil (cfg : Config) :
    canonicalize cfg [] = ∅ := by simp [canonicalize]

/-- A trace with no real rows canonicalizes to the empty set. -/
lemma canonicalize_no_real (cfg : Config) (t : Trace)
    (h : ∀ row ∈ t, cfg.isReal row = false) :
    canonicalize cfg t = ∅ := by
  unfold canonicalize
  have hfilt : t.filter cfg.isReal = [] := by
    apply List.filter_eq_nil_iff.mpr
    intro row hrow
    rw [h row hrow]
    decide
  rw [hfilt]
  simp

/-- **Distributivity over append.** The canonicalizer treats traces as sets:
    appending two traces unions their canonical forms. -/
lemma canonicalize_append (cfg : Config) (t₁ t₂ : Trace) :
    canonicalize cfg (t₁ ++ t₂) = canonicalize cfg t₁ ∪ canonicalize cfg t₂ := by
  unfold canonicalize
  rw [List.filter_append, List.map_append, List.toFinset_append]

/-- **Padding invariance.** Appending a non-real row leaves the canonical form
    unchanged. -/
lemma canonicalize_append_padding (cfg : Config) (t : Trace) (pad : Row)
    (h : cfg.isReal pad = false) :
    canonicalize cfg (t ++ [pad]) = canonicalize cfg t := by
  rw [canonicalize_append]
  have : canonicalize cfg [pad] = ∅ :=
    canonicalize_no_real cfg [pad] (fun row hrow => by
      rw [List.mem_singleton] at hrow
      rw [hrow]; exact h)
  rw [this, Finset.union_empty]

/-- **Permutation invariance.** Permuting trace rows leaves the canonical form
    unchanged (the canonicalizer is order-blind). -/
lemma canonicalize_perm (cfg : Config) {t₁ t₂ : Trace} (h : t₁.Perm t₂) :
    canonicalize cfg t₁ = canonicalize cfg t₂ := by
  unfold canonicalize
  exact List.toFinset_eq_of_perm _ _ ((h.filter _).map _)

/-! ## (iv) String presentation (separate concern: printer correctness) -/

/-- Mirrors `trace_fmt_with_idxs` (`src/trace.rs:123`). -/
def renderLimbs (xs : List Interval) : String :=
  String.intercalate ", " (xs.map Interval.repr)

/-- Mirrors the `format!` in `cr_add` (`examples/sp1/examples/addsub.rs:30`). -/
def renderTuple (tup : Tuple) : String :=
  "input0: [" ++ renderLimbs tup.b ++
  "], input1: [" ++ renderLimbs tup.c ++
  "], output: [" ++ renderLimbs tup.a ++ "]"

/-- Mirrors `PrettySet::fmt` (`src/utils.rs:66`). -/
def renderSet (strs : List String) : String :=
  "{\n" ++ String.join (strs.map (fun s => s ++ ",\n")) ++ "}"

/-- The full Rust-faithful string canonicalizer. Marked `noncomputable`
    because `Finset.toList` chooses an underlying representative
    noncomputably; this is a model used in proofs, not for execution. -/
noncomputable def stringRepr (cfg : Config) (t : Trace) : String :=
  renderSet ((canonicalize cfg t).toList.map renderTuple)

/-- **Printer consistency** (forward at the string level): equal canonical
    forms produce equal canonical strings. -/
lemma stringRepr_consistent (cfg : Config) (t₁ t₂ : Trace) :
    canonicalize cfg t₁ = canonicalize cfg t₂ →
    stringRepr cfg t₁ = stringRepr cfg t₂ := by
  intro h
  show renderSet ((canonicalize cfg t₁).toList.map renderTuple)
     = renderSet ((canonicalize cfg t₂).toList.map renderTuple)
  rw [h]

/-! ## Sanity checks -/

/-- The sp1 ADD column layout (`examples/sp1/examples/addsub.rs:32-34`). -/
def sp1AddConfig : Config where
  idxB   := [8, 9, 10, 11]
  idxC   := [12, 13, 14, 15]
  idxA   := [1, 2, 3, 4]
  isReal := fun _ => true  -- stub; real predicate: col 17 nonzero mod prime

/-- Padding-invariance check on a concrete trace. -/
example (t : Trace) (pad : Row) (h : sp1AddConfig.isReal pad = false) :
    canonicalize sp1AddConfig (t ++ [pad]) = canonicalize sp1AddConfig t :=
  canonicalize_append_padding sp1AddConfig t pad h

/-- Distributivity check. -/
example (t₁ t₂ : Trace) :
    canonicalize sp1AddConfig (t₁ ++ t₂)
      = canonicalize sp1AddConfig t₁ ∪ canonicalize sp1AddConfig t₂ :=
  canonicalize_append sp1AddConfig t₁ t₂

/-- Reverse-direction (membership) check: any tuple in the canonical form
    must originate from a specific real row. -/
example (t : Trace) (tup : Tuple) (h : tup ∈ canonicalize sp1AddConfig t) :
    ∃ row ∈ t, sp1AddConfig.isReal row ∧ sp1AddConfig.projectRow row = tup :=
  (mem_canonicalize_iff sp1AddConfig t tup).mp h

end Zebra.ALU
