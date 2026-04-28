/-
Zebra — Canonicalizer faithfulness for ALU tables (multi-row).

Models the per-row projection canonicalizer pattern shared by ALU and
memory-op tables (`src/canonicalizer.rs:56` and `:90`). For each "real" row,
project the configured operand columns to a `(b, c, a)` tuple; the canonical
form is the deduplicated *set* of those projections (matching the Rust
`HashSet<…>` semantics).

The canonicalized representation is a `Finset Tuple` (a SET, not a string).
The string output is a separate printer over this set — trivially deterministic
in the forward direction; injectivity at the string level is a separate
"printer correctness" concern (see `stringRepr_consistent`).

The strongest true "one-to-one" statement is not between raw `Trace` values
and canonical forms: row order, duplicate projected rows, and non-real padding
are intentionally erased. The proof below therefore states injectivity on the
quotient of traces by canonical equivalence, i.e. one canonical form for one
original multi-row table *up to exactly the information the canonicalizer is
designed to forget*.

Build: `lake build` (uses mathlib via local symlinks under `.lake/packages/`).
-/
import Mathlib.Data.Finset.Basic
import Mathlib.Data.Finset.Dedup
import Mathlib.Data.Finset.Lattice.Lemmas
import Mathlib.Data.Quot

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

/-- The semantic events we want the canonicalizer to recover from a generated
    VM table: a set of input/output tuples. -/
abbrev EventSet := Finset Tuple

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

/-- A table encodes an event set when its real rows, after projection, are
    exactly the semantic events. This deliberately ignores helper columns,
    row order, padding rows, and duplicated events. -/
def TableEncodesEvents (cfg : Config) (events : EventSet) (table : Trace) : Prop :=
  ∀ tup, tup ∈ events ↔
    ∃ row ∈ table, cfg.isReal row ∧ cfg.projectRow row = tup

/-! ## (ii) Canonicalizer membership -/

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

/-! ## (iii) Semantic event-set layer -/

/-- **Canonicalizer correctness for encoded event sets.** If a VM-generated
    table encodes an event set, Zebra's canonicalizer returns exactly that set. -/
theorem canonicalize_eq_events_of_encodes (cfg : Config)
    {events : EventSet} {table : Trace}
    (h : TableEncodesEvents cfg events table) :
    canonicalize cfg table = events := by
  ext tup
  rw [mem_canonicalize_iff]
  exact (h tup).symm

/-- The canonicalizer returns an event set exactly when the table encodes that
    event set. This is the table-level specification of the canonicalizer. -/
theorem canonicalize_eq_events_iff (cfg : Config)
    (events : EventSet) (table : Trace) :
    canonicalize cfg table = events ↔ TableEncodesEvents cfg events table := by
  constructor
  · intro h tup
    rw [← h]
    exact mem_canonicalize_iff cfg table tup
  · exact canonicalize_eq_events_of_encodes cfg

/-- If two event sets are both encoded by the same table, they are equal. -/
theorem encoded_events_unique (cfg : Config)
    {events₁ events₂ : EventSet} {table : Trace}
    (h₁ : TableEncodesEvents cfg events₁ table)
    (h₂ : TableEncodesEvents cfg events₂ table) :
    events₁ = events₂ := by
  rw [← canonicalize_eq_events_of_encodes cfg h₁,
      ← canonicalize_eq_events_of_encodes cfg h₂]

/-- If two tables encode the same VM event set, they have the same canonicalized
    representation even if their padding rows, helper columns, order, or
    duplicated rows differ. -/
theorem canonicalize_eq_of_same_events (cfg : Config)
    {events : EventSet} {table₁ table₂ : Trace}
    (h₁ : TableEncodesEvents cfg events table₁)
    (h₂ : TableEncodesEvents cfg events table₂) :
    canonicalize cfg table₁ = canonicalize cfg table₂ := by
  rw [canonicalize_eq_events_of_encodes cfg h₁,
      canonicalize_eq_events_of_encodes cfg h₂]

/-- If two encoded tables have the same canonicalized representation, then the
    original semantic event sets they encode are equal. -/
theorem events_eq_of_canonicalize_eq (cfg : Config)
    {events₁ events₂ : EventSet} {table₁ table₂ : Trace}
    (h₁ : TableEncodesEvents cfg events₁ table₁)
    (h₂ : TableEncodesEvents cfg events₂ table₂)
    (hcanon : canonicalize cfg table₁ = canonicalize cfg table₂) :
    events₁ = events₂ := by
  rw [← canonicalize_eq_events_of_encodes cfg h₁,
      ← canonicalize_eq_events_of_encodes cfg h₂]
  exact hcanon

/-- For tables known to encode event sets, equality of canonicalized
    representations is exactly equality of the original semantic event sets.
    This is the VM/table/canonical-representation one-to-one statement, with
    table-generator correctness supplied as the two `TableEncodesEvents`
    hypotheses. -/
theorem canonicalize_eq_iff_events_eq_of_encodes (cfg : Config)
    {events₁ events₂ : EventSet} {table₁ table₂ : Trace}
    (h₁ : TableEncodesEvents cfg events₁ table₁)
    (h₂ : TableEncodesEvents cfg events₂ table₂) :
    canonicalize cfg table₁ = canonicalize cfg table₂ ↔ events₁ = events₂ := by
  constructor
  · exact events_eq_of_canonicalize_eq cfg h₁ h₂
  · intro hevents
    rw [canonicalize_eq_events_of_encodes cfg h₁,
        canonicalize_eq_events_of_encodes cfg h₂,
        hevents]

/-! ## (iv) Abstract VM/table-generator bridge -/

section VMGenerator

variable {VMExecution : Type}
variable (cfg : Config)
variable (vmEvents : VMExecution → EventSet)
variable (generateTable : VMExecution → Trace)

/-- A table generator is faithful when, for every VM execution, the generated
    table encodes exactly the semantic events recorded by that execution. -/
def TableGeneratorFaithful : Prop :=
  ∀ exec, TableEncodesEvents cfg (vmEvents exec) (generateTable exec)

/-- If the table generator is faithful, Zebra recovers the VM execution's
    semantic event set from the generated table. -/
theorem canonicalize_generated_table_eq_vmEvents
    (hgen : TableGeneratorFaithful cfg vmEvents generateTable)
    (exec : VMExecution) :
    canonicalize cfg (generateTable exec) = vmEvents exec :=
  canonicalize_eq_events_of_encodes cfg (hgen exec)

/-- For faithful generated tables, equality of Zebra canonicalized
    representations is exactly equality of the original VM event sets. -/
theorem canonicalize_generated_eq_iff_vmEvents_eq
    (hgen : TableGeneratorFaithful cfg vmEvents generateTable)
    (exec₁ exec₂ : VMExecution) :
    canonicalize cfg (generateTable exec₁) =
      canonicalize cfg (generateTable exec₂) ↔
    vmEvents exec₁ = vmEvents exec₂ :=
  canonicalize_eq_iff_events_eq_of_encodes cfg (hgen exec₁) (hgen exec₂)

end VMGenerator

/-! ## (v) One-to-one statement for original multi-row tables -/

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

/-- `TraceEquiv` is an equivalence relation: it is exactly equality of the
    canonicalized real-row projection set, phrased before calling
    `canonicalize`. -/
theorem TraceEquiv.refl (cfg : Config) (t : Trace) :
    TraceEquiv cfg t t := by
  intro tup
  exact Iff.rfl

theorem TraceEquiv.symm (cfg : Config) {t₁ t₂ : Trace}
    (h : TraceEquiv cfg t₁ t₂) :
    TraceEquiv cfg t₂ t₁ := by
  intro tup
  exact (h tup).symm

theorem TraceEquiv.trans (cfg : Config) {t₁ t₂ t₃ : Trace}
    (h₁₂ : TraceEquiv cfg t₁ t₂) (h₂₃ : TraceEquiv cfg t₂ t₃) :
    TraceEquiv cfg t₁ t₃ := by
  intro tup
  exact (h₁₂ tup).trans (h₂₃ tup)

/-- Raw traces modulo the information erased by canonicalization: padding rows,
    row order, and duplicate rows with the same projected tuple. -/
def traceSetoid (cfg : Config) : Setoid Trace where
  r := TraceEquiv cfg
  iseqv := ⟨TraceEquiv.refl cfg, TraceEquiv.symm cfg, TraceEquiv.trans cfg⟩

/-- The mathematically meaningful "original trace" for this canonicalizer:
    an equivalence class of raw traces with the same real-row projected tuples. -/
abbrev TraceClass (cfg : Config) := Quotient (traceSetoid cfg)

/-- Canonicalization descends from raw traces to trace classes. -/
noncomputable def canonicalizeClass (cfg : Config) :
    TraceClass cfg → Finset Tuple :=
  Quotient.lift (canonicalize cfg) (by
    intro t₁ t₂ h
    exact canonicalize_eq_of_equiv cfg h)

/-- **One-to-one theorem.** Canonicalized representations are injective for
    original multi-row tables once raw traces are quotiented by the exact
    observational equivalence of the canonicalizer. -/
theorem canonicalizeClass_injective (cfg : Config) :
    Function.Injective (canonicalizeClass cfg) := by
  intro q₁ q₂ h
  refine Quotient.inductionOn₂ q₁ q₂ ?_ h
  intro t₁ t₂ hcanon
  apply Quotient.sound
  exact (canonicalize_eq_iff cfg t₁ t₂).mp hcanon

/-- Equal canonical forms are the same as equal trace classes. This is the
    compact bijection-style statement: canonicalization is a lossless
    representation of traces after quotienting away order, padding, and
    duplicate projected rows. -/
theorem canonicalizeClass_eq_iff (cfg : Config) (q₁ q₂ : TraceClass cfg) :
    canonicalizeClass cfg q₁ = canonicalizeClass cfg q₂ ↔ q₁ = q₂ :=
  ⟨fun h => canonicalizeClass_injective cfg h, fun h => by rw [h]⟩

/-! ## (vi) Helper lemmas -/

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

/-! ## (vii) String presentation (separate concern: printer correctness) -/

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
