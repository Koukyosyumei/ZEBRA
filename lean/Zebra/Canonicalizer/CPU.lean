/-
Zebra — CPU table canonical representation.
-/
import Mathlib.Data.Finset.Image
import Zebra.Canonicalizer.Generic
import Zebra.Canonicalizer.Generator

namespace Zebra.CPU

/-- Canonical instruction record emitted by a real CPU row. The clock is kept as
    projected columns because different zkVMs materialize it with different limb
    layouts. -/
structure InstructionTuple where
  clk : List Interval
  pc : List Interval
deriving DecidableEq, Repr

/-- Canonical memory-write record emitted by a real CPU row when the row writes
    a register/memory cell. -/
structure MemoryWriteTuple where
  clk : List Interval
  addr : List Interval
  value : List Interval
deriving DecidableEq, Repr

/-- CPU rows can contribute more than one semantic record: always an
    instruction record, and zero or more write records. -/
inductive RecordRepr where
  | instruction : InstructionTuple → RecordRepr
  | memoryWrite : MemoryWriteTuple → RecordRepr
deriving DecidableEq, Repr

/-- Layout for one possible write record extracted from a CPU row. -/
structure WriteLayout where
  isWrite : Row → Bool
  addr : List Nat
  value : List Nat

/-- CPU canonicalization layout. `isReal` filters padding rows; `clk` and `pc`
    define the instruction identity; `writes` defines optional write records. -/
structure Config where
  isReal : Row → Bool
  clk : List Nat
  pc : List Nat
  writes : List WriteLayout

def instructionOfRow (cfg : Config) (row : Row) : InstructionTuple :=
  { clk := row.project cfg.clk, pc := row.project cfg.pc }

def writeOfRow (cfg : Config) (layout : WriteLayout) (row : Row) : MemoryWriteTuple :=
  { clk := row.project cfg.clk,
    addr := row.project layout.addr,
    value := row.project layout.value }

def rowRecords (cfg : Config) (row : Row) : List RecordRepr :=
  if cfg.isReal row then
    RecordRepr.instruction (instructionOfRow cfg row) ::
      cfg.writes.filterMap (fun layout =>
        if layout.isWrite row then
          some (RecordRepr.memoryWrite (writeOfRow cfg layout row))
        else
          none)
  else
    []

/-- CPU canonicalizer: flatten all records produced by real rows, then
    deduplicate into a finite canonical set. -/
def canonicalize (cfg : Config) (t : Trace) : Finset RecordRepr :=
  (t.flatMap (rowRecords cfg)).toFinset

lemma mem_canonicalize_iff (cfg : Config) (t : Trace) (repr : RecordRepr) :
    repr ∈ canonicalize cfg t ↔
      ∃ row ∈ t, repr ∈ rowRecords cfg row := by
  unfold canonicalize
  simp [List.mem_flatMap]

/-- A CPU table encodes a record set when the records emitted by its rows are
    exactly the canonical identities of that set. -/
def TableEncodesRecords {Record : Type}
    (cfg : Config) (enc : Generic.RecordIdentity Record RecordRepr)
    (records : Generic.RecordSet Record) (table : Trace) : Prop :=
  ∀ repr, repr ∈ Finset.image enc.toRepr records ↔
    ∃ row ∈ table, repr ∈ rowRecords cfg row

def TableGeneratorFaithful {Record : Type}
    (cfg : Config) (enc : Generic.RecordIdentity Record RecordRepr)
    (generateTable : Generic.RecordSet Record → Trace) : Prop :=
  ∀ records, TableEncodesRecords cfg enc records (generateTable records)

lemma canonicalize_eq_recordReprSet_of_encodes {Record : Type}
    (cfg : Config) (enc : Generic.RecordIdentity Record RecordRepr)
    {records : Generic.RecordSet Record} {table : Trace}
    (h : TableEncodesRecords cfg enc records table) :
    canonicalize cfg table = Finset.image enc.toRepr records := by
  ext repr
  rw [mem_canonicalize_iff]
  exact (h repr).symm

theorem canonicalize_generated_table_eq_recordReprSet {Record : Type}
    (cfg : Config) (enc : Generic.RecordIdentity Record RecordRepr)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : TableGeneratorFaithful cfg enc generateTable)
    (records : Generic.RecordSet Record) :
    canonicalize cfg (generateTable records) = Finset.image enc.toRepr records :=
  canonicalize_eq_recordReprSet_of_encodes cfg enc (hgen records)

theorem canonicalize_generator_independent {Record : Type}
    (cfg : Config) (enc : Generic.RecordIdentity Record RecordRepr)
    (gen₁ gen₂ : Generic.RecordSet Record → Trace)
    (h₁ : TableGeneratorFaithful cfg enc gen₁)
    (h₂ : TableGeneratorFaithful cfg enc gen₂)
    (records : Generic.RecordSet Record) :
    canonicalize cfg (gen₁ records) = canonicalize cfg (gen₂ records) := by
  rw [canonicalize_generated_table_eq_recordReprSet cfg enc gen₁ h₁ records,
      canonicalize_generated_table_eq_recordReprSet cfg enc gen₂ h₂ records]

theorem canonicalize_generated_eq_iff_records_eq {Record : Type}
    [DecidableEq Record]
    (cfg : Config) (enc : Generic.RecordIdentity Record RecordRepr)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : TableGeneratorFaithful cfg enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
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

end Zebra.CPU
