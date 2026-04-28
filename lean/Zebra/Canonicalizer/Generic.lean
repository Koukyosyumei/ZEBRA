/-
Zebra — generic canonicalizer theorem layer.

This file proves the reusable fact shared by ALU, memory-op, and control-flow
canonicalizers: if a table generator faithfully encodes a semantic event set as
real projected rows, then canonicalization is one-to-one with the event set.
-/
import Mathlib.Data.Finset.Basic
import Mathlib.Data.Finset.Dedup
import Mathlib.Data.Finset.Image
import Mathlib.Data.Finset.Lattice.Lemmas
import Mathlib.Data.Quot

namespace Zebra

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

namespace Generic

/-- Generic table layout: decide which rows are real and project each real row
    to the table-specific canonical representation. -/
structure Config (Repr : Type) where
  isReal : Row → Bool
  projectRow : Row → Repr

/-- Generic canonicalizer: project all real rows and deduplicate them as a
    finite set. Different table families choose different `Repr` types. -/
def canonicalize {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) (t : Trace) : Finset Repr :=
  ((t.filter cfg.isReal).map cfg.projectRow).toFinset

/-- Injective representation of semantic events as the canonical table
    representation. This is the condition needed for one-to-one recovery of
    event sets from canonicalized tables. -/
structure EventEncoding (Event Repr : Type) where
  toRepr : Event → Repr
  injective : Function.Injective toRepr

abbrev EventSet (Event : Type) := Finset Event

/-- A table encodes an event set when its real projected rows are exactly the
    encoded event set. -/
def TableEncodesEvents {Event Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) (enc : EventEncoding Event Repr)
    (events : EventSet Event) (table : Trace) : Prop :=
  ∀ repr, repr ∈ Finset.image enc.toRepr events ↔
    ∃ row ∈ table, cfg.isReal row ∧ cfg.projectRow row = repr

/-- A table generator is faithful when every generated table encodes exactly
    the event set it was generated from. -/
def TableGeneratorFaithful {Event Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) (enc : EventEncoding Event Repr)
    (generateTable : EventSet Event → Trace) : Prop :=
  ∀ events, TableEncodesEvents cfg enc events (generateTable events)

lemma mem_canonicalize_iff {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) (t : Trace) (repr : Repr) :
    repr ∈ canonicalize cfg t ↔
      ∃ row ∈ t, cfg.isReal row ∧ cfg.projectRow row = repr := by
  unfold canonicalize
  simp [List.mem_toFinset, List.mem_map, List.mem_filter]
  constructor
  · rintro ⟨row, ⟨hin, hreal⟩, heq⟩
    exact ⟨row, hin, hreal, heq⟩
  · rintro ⟨row, hin, hreal, heq⟩
    exact ⟨row, ⟨hin, hreal⟩, heq⟩

lemma canonicalize_eq_eventReprSet_of_encodes {Event Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) (enc : EventEncoding Event Repr)
    {events : EventSet Event} {table : Trace}
    (h : TableEncodesEvents cfg enc events table) :
    canonicalize cfg table = Finset.image enc.toRepr events := by
  ext repr
  rw [mem_canonicalize_iff]
  exact (h repr).symm

/-! ## Quotient by semantic equivalence -/

/-- Semantic equivalence induced by the canonicalizer: two raw trace tables are
    equivalent exactly when canonicalization returns the same representation set. -/
def SemanticEquiv {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) (t₁ t₂ : Trace) : Prop :=
  canonicalize cfg t₁ = canonicalize cfg t₂

theorem semanticEquiv_refl {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) (t : Trace) :
    SemanticEquiv cfg t t := rfl

theorem semanticEquiv_symm {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) {t₁ t₂ : Trace}
    (h : SemanticEquiv cfg t₁ t₂) :
    SemanticEquiv cfg t₂ t₁ := h.symm

theorem semanticEquiv_trans {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) {t₁ t₂ t₃ : Trace}
    (h₁₂ : SemanticEquiv cfg t₁ t₂)
    (h₂₃ : SemanticEquiv cfg t₂ t₃) :
    SemanticEquiv cfg t₁ t₃ := Eq.trans h₁₂ h₂₃

/-- Setoid of raw trace tables modulo semantic equivalence. -/
def semanticSetoid {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) : Setoid Trace where
  r := SemanticEquiv cfg
  iseqv := ⟨semanticEquiv_refl cfg, semanticEquiv_symm cfg, semanticEquiv_trans cfg⟩

/-- Canonical trace-table space `D / ~R`. -/
abbrev CanonicalTraceSpace {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) := Quotient (semanticSetoid cfg)

/-- Image of the canonicalizer `Im(R)`. -/
abbrev CanonicalizerImage {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) := { reprs : Finset Repr // ∃ table, canonicalize cfg table = reprs }

/-- The natural map from canonical trace classes to canonicalizer image. -/
noncomputable def quotientToImage {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) :
    CanonicalTraceSpace cfg → CanonicalizerImage cfg :=
  Quotient.lift
    (fun table => ⟨canonicalize cfg table, ⟨table, rfl⟩⟩)
    (by
      intro t₁ t₂ h
      apply Subtype.ext
      exact h)

/-- The natural map from canonicalizer image back to canonical trace classes,
    choosing any raw table that realizes the image element. -/
noncomputable def imageToQuotient {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) :
    CanonicalizerImage cfg → CanonicalTraceSpace cfg :=
  fun image => Quotient.mk (semanticSetoid cfg) (Classical.choose image.property)

theorem quotientToImage_imageToQuotient {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) (image : CanonicalizerImage cfg) :
    quotientToImage cfg (imageToQuotient cfg image) = image := by
  apply Subtype.ext
  exact Classical.choose_spec image.property

theorem imageToQuotient_quotientToImage {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) (q : CanonicalTraceSpace cfg) :
    imageToQuotient cfg (quotientToImage cfg q) = q := by
  refine Quotient.inductionOn q ?_
  intro table
  apply Quotient.sound
  exact Classical.choose_spec (quotientToImage cfg (Quotient.mk (semanticSetoid cfg) table)).property

/-- Formal version of the paper statement: the quotient of raw trace tables by
    canonicalizer-induced semantic equivalence is naturally bijective with the
    image of the canonicalizer. -/
theorem canonicalTraceSpace_equiv_image {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) :
    Function.Bijective (quotientToImage cfg) := by
  constructor
  · intro q₁ q₂ h
    rw [← imageToQuotient_quotientToImage cfg q₁,
        ← imageToQuotient_quotientToImage cfg q₂,
        h]
  · intro image
    exact ⟨imageToQuotient cfg image, quotientToImage_imageToQuotient cfg image⟩

/-- Generic correctness theorem: faithful table generation followed by
    canonicalization returns the encoded event set. -/
theorem canonicalize_generated_table_eq_eventReprSet {Event Repr : Type}
    [DecidableEq Repr]
    (cfg : Config Repr) (enc : EventEncoding Event Repr)
    (generateTable : EventSet Event → Trace)
    (hgen : TableGeneratorFaithful cfg enc generateTable)
    (events : EventSet Event) :
    canonicalize cfg (generateTable events) = Finset.image enc.toRepr events :=
  canonicalize_eq_eventReprSet_of_encodes cfg enc (hgen events)

/-- Generator-independence of canonicalization: any two faithful generators
    produce the same canonical form on the same event set. This is the collapse
    property that different raw representations of the same computation map to
    the same canonical form. -/
theorem canonicalize_generator_independent {Event Repr : Type}
    [DecidableEq Repr]
    (cfg : Config Repr) (enc : EventEncoding Event Repr)
    (gen₁ gen₂ : EventSet Event → Trace)
    (h₁ : TableGeneratorFaithful cfg enc gen₁)
    (h₂ : TableGeneratorFaithful cfg enc gen₂)
    (events : EventSet Event) :
    canonicalize cfg (gen₁ events) = canonicalize cfg (gen₂ events) := by
  rw [canonicalize_generated_table_eq_eventReprSet cfg enc gen₁ h₁ events,
      canonicalize_generated_table_eq_eventReprSet cfg enc gen₂ h₂ events]

/-- Generic one-to-one theorem: if the table generator is faithful and event
    encoding is injective, then equal canonical representations are exactly
    equal original event sets. -/
theorem canonicalize_generated_eq_iff_events_eq {Event Repr : Type}
    [DecidableEq Repr]
    (cfg : Config Repr) (enc : EventEncoding Event Repr)
    (generateTable : EventSet Event → Trace)
    (hgen : TableGeneratorFaithful cfg enc generateTable)
    (events₁ events₂ : EventSet Event) :
    canonicalize cfg (generateTable events₁) =
      canonicalize cfg (generateTable events₂) ↔
    events₁ = events₂ := by
  rw [canonicalize_generated_table_eq_eventReprSet cfg enc generateTable hgen events₁,
      canonicalize_generated_table_eq_eventReprSet cfg enc generateTable hgen events₂]
  constructor
  · intro h
    exact Finset.image_injective enc.injective h
  · intro h
    rw [h]

end Generic
end Zebra
