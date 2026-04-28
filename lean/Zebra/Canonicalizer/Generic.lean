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
