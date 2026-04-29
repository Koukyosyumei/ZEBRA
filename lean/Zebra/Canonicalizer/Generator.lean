/-
Zebra — generator/event-set theorem layer.
-/
import Mathlib.Data.Finset.Image
import Zebra.Canonicalizer.Generic

namespace Zebra.Generic

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

end Zebra.Generic
