/-
Zebra — generator/record-set theorem layer.
-/
import Mathlib.Data.Finset.Image
import Zebra.Canonicalizer.Generic

namespace Zebra.Generic

/-- Injective representation of semantic records as the canonical table
    representation. This is the condition needed for one-to-one recovery of
    record sets from canonicalized tables. -/
structure RecordIdentity (Record Repr : Type) where
  toRepr : Record → Repr
  injective : Function.Injective toRepr

abbrev RecordSet (Record : Type) := Finset Record

/-- A table encodes a record set when its real projected rows are exactly the
    encoded record set. -/
def TableEncodesRecords {Record Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) (enc : RecordIdentity Record Repr)
    (records : RecordSet Record) (table : Trace) : Prop :=
  ∀ repr, repr ∈ Finset.image enc.toRepr records ↔
    ∃ row ∈ table, cfg.isReal row ∧ cfg.projectRow row = repr

/-- A table generator is faithful when every generated table encodes exactly
    the record set it was generated from. -/
def TableGeneratorFaithful {Record Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) (enc : RecordIdentity Record Repr)
    (generateTable : RecordSet Record → Trace) : Prop :=
  ∀ records, TableEncodesRecords cfg enc records (generateTable records)

lemma canonicalize_eq_recordReprSet_of_encodes {Record Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) (enc : RecordIdentity Record Repr)
    {records : RecordSet Record} {table : Trace}
    (h : TableEncodesRecords cfg enc records table) :
    canonicalize cfg table = Finset.image enc.toRepr records := by
  ext repr
  rw [mem_canonicalize_iff]
  exact (h repr).symm

/-- Generic correctness theorem: faithful table generation followed by
    canonicalization returns the encoded record set. -/
theorem canonicalize_generated_table_eq_recordReprSet {Record Repr : Type}
    [DecidableEq Repr]
    (cfg : Config Repr) (enc : RecordIdentity Record Repr)
    (generateTable : RecordSet Record → Trace)
    (hgen : TableGeneratorFaithful cfg enc generateTable)
    (records : RecordSet Record) :
    canonicalize cfg (generateTable records) = Finset.image enc.toRepr records :=
  canonicalize_eq_recordReprSet_of_encodes cfg enc (hgen records)

/-- Generator-independence of canonicalization: any two faithful generators
    produce the same canonical form on the same record set. This is the collapse
    property that different raw representations of the same computation map to
    the same canonical form. -/
theorem canonicalize_generator_independent {Record Repr : Type}
    [DecidableEq Repr]
    (cfg : Config Repr) (enc : RecordIdentity Record Repr)
    (gen₁ gen₂ : RecordSet Record → Trace)
    (h₁ : TableGeneratorFaithful cfg enc gen₁)
    (h₂ : TableGeneratorFaithful cfg enc gen₂)
    (records : RecordSet Record) :
    canonicalize cfg (gen₁ records) = canonicalize cfg (gen₂ records) := by
  rw [canonicalize_generated_table_eq_recordReprSet cfg enc gen₁ h₁ records,
      canonicalize_generated_table_eq_recordReprSet cfg enc gen₂ h₂ records]

/-- Generic one-to-one theorem: if the table generator is faithful and record
    encoding is injective, then equal canonical representations are exactly
    equal original record sets. -/
theorem canonicalize_generated_eq_iff_records_eq {Record Repr : Type}
    [DecidableEq Repr]
    (cfg : Config Repr) (enc : RecordIdentity Record Repr)
    (generateTable : RecordSet Record → Trace)
    (hgen : TableGeneratorFaithful cfg enc generateTable)
    (records₁ records₂ : RecordSet Record) :
    canonicalize cfg (generateTable records₁) =
      canonicalize cfg (generateTable records₂) ↔
    records₁ = records₂ := by
  rw [canonicalize_generated_table_eq_recordReprSet cfg enc generateTable hgen records₁,
      canonicalize_generated_table_eq_recordReprSet cfg enc generateTable hgen records₂]
  constructor
  · intro h
    exact Finset.image_injective enc.injective h
  · intro h
    rw [h]

end Zebra.Generic
