/-
Zebra — generator/record-set theorem layer.
-/
import Mathlib.Data.Finset.Image
import Zebra.Canonicalizer.Generic

namespace Zebra.Generic

/-- Injective identity of semantic records as canonical table identities. The
    second type parameter is the identity produced by the canonicalizer, not
    necessarily the full semantic record type. -/
structure RecordIdentity (Record Identity : Type) where
  toIdentity : Record → Identity
  injective : Function.Injective toIdentity

abbrev RecordSet (Record : Type) := Finset Record

/-- A table encodes a record set when its real projected rows are exactly the
    canonical identities of that record set. -/
def TableEncodesRecords {Record Identity : Type} [DecidableEq Identity]
    (cfg : Config Identity) (recordId : RecordIdentity Record Identity)
    (records : RecordSet Record) (table : Trace) : Prop :=
  ∀ identity, identity ∈ Finset.image recordId.toIdentity records ↔
    ∃ row ∈ table, cfg.isReal row ∧ cfg.projectRow row = identity

/-- A table generator is faithful when every generated table encodes exactly
    the record set it was generated from. -/
def TableGeneratorFaithful {Record Identity : Type} [DecidableEq Identity]
    (cfg : Config Identity) (recordId : RecordIdentity Record Identity)
    (generateTable : RecordSet Record → Trace) : Prop :=
  ∀ records, TableEncodesRecords cfg recordId records (generateTable records)

lemma canonicalize_eq_recordIdentitySet_of_encodes {Record Identity : Type}
    [DecidableEq Identity]
    (cfg : Config Identity) (recordId : RecordIdentity Record Identity)
    {records : RecordSet Record} {table : Trace}
    (h : TableEncodesRecords cfg recordId records table) :
    canonicalize cfg table = Finset.image recordId.toIdentity records := by
  ext identity
  rw [mem_canonicalize_iff]
  exact (h identity).symm

/-- Generic correctness theorem: faithful table generation followed by
    canonicalization returns the canonical identities of the record set. -/
theorem canonicalize_generated_table_eq_recordIdentitySet {Record Identity : Type}
    [DecidableEq Identity]
    (cfg : Config Identity) (recordId : RecordIdentity Record Identity)
    (generateTable : RecordSet Record → Trace)
    (hgen : TableGeneratorFaithful cfg recordId generateTable)
    (records : RecordSet Record) :
    canonicalize cfg (generateTable records) = Finset.image recordId.toIdentity records :=
  canonicalize_eq_recordIdentitySet_of_encodes cfg recordId (hgen records)

/-- Generator-independence of canonicalization: any two faithful generators
    produce the same canonical form on the same record set. This is the collapse
    property that different raw representations of the same computation map to
    the same canonical form. -/
theorem canonicalize_generator_independent {Record Identity : Type}
    [DecidableEq Identity]
    (cfg : Config Identity) (recordId : RecordIdentity Record Identity)
    (gen₁ gen₂ : RecordSet Record → Trace)
    (h₁ : TableGeneratorFaithful cfg recordId gen₁)
    (h₂ : TableGeneratorFaithful cfg recordId gen₂)
    (records : RecordSet Record) :
    canonicalize cfg (gen₁ records) = canonicalize cfg (gen₂ records) := by
  rw [canonicalize_generated_table_eq_recordIdentitySet cfg recordId gen₁ h₁ records,
      canonicalize_generated_table_eq_recordIdentitySet cfg recordId gen₂ h₂ records]

/-- Generic one-to-one theorem: if the table generator is faithful and record
    identity is injective, then equal canonical identities are exactly equal
    original record sets. -/
theorem canonicalize_generated_eq_iff_records_eq {Record Identity : Type}
    [DecidableEq Identity]
    (cfg : Config Identity) (recordId : RecordIdentity Record Identity)
    (generateTable : RecordSet Record → Trace)
    (hgen : TableGeneratorFaithful cfg recordId generateTable)
    (records₁ records₂ : RecordSet Record) :
    canonicalize cfg (generateTable records₁) =
      canonicalize cfg (generateTable records₂) ↔
    records₁ = records₂ := by
  rw [canonicalize_generated_table_eq_recordIdentitySet cfg recordId generateTable hgen records₁,
      canonicalize_generated_table_eq_recordIdentitySet cfg recordId generateTable hgen records₂]
  constructor
  · intro h
    exact Finset.image_injective recordId.injective h
  · intro h
    rw [h]

end Zebra.Generic
