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

/-- Record identities reconstructed from a CPU table. `tableId` interprets the
    CPU table record representation as the abstract identity of an execution
    record. -/
def recordIdsOfTable {Identity : Type} [DecidableEq Identity]
    (cfg : Config) (tableId : RecordRepr → Identity) (table : Trace) : Finset Identity :=
  Finset.image tableId (canonicalize cfg table)

lemma mem_recordIdsOfTable_iff {Identity : Type} [DecidableEq Identity]
    (cfg : Config) (tableId : RecordRepr → Identity) (table : Trace) (identity : Identity) :
    identity ∈ recordIdsOfTable cfg tableId table ↔
      ∃ row ∈ table, ∃ repr ∈ rowRecords cfg row, tableId repr = identity := by
  unfold recordIdsOfTable
  constructor
  · intro h
    rcases Finset.mem_image.mp h with ⟨repr, hrepr, hidentity⟩
    rw [mem_canonicalize_iff] at hrepr
    rcases hrepr with ⟨row, hrow, hreprInRow⟩
    exact ⟨row, hrow, repr, hreprInRow, hidentity⟩
  · rintro ⟨row, hrow, repr, hreprInRow, hidentity⟩
    apply Finset.mem_image.mpr
    exact ⟨repr, (mem_canonicalize_iff cfg table repr).mpr ⟨row, hrow, hreprInRow⟩, hidentity⟩

def TableEncodesRecordIds {Record Identity : Type} [DecidableEq Identity]
    (cfg : Config) (tableId : RecordRepr → Identity)
    (recordId : Generic.RecordIdentity Record Identity)
    (records : Generic.RecordSet Record) (table : Trace) : Prop :=
  ∀ identity, identity ∈ Finset.image recordId.toIdentity records ↔
    ∃ row ∈ table, ∃ repr ∈ rowRecords cfg row, tableId repr = identity

def TableGeneratorFaithfulToIds {Record Identity : Type} [DecidableEq Identity]
    (cfg : Config) (tableId : RecordRepr → Identity)
    (recordId : Generic.RecordIdentity Record Identity)
    (generateTable : Generic.RecordSet Record → Trace) : Prop :=
  ∀ records, TableEncodesRecordIds cfg tableId recordId records (generateTable records)

lemma recordIdsOfTable_eq_recordIds {Record Identity : Type}
    [DecidableEq Identity]
    (cfg : Config) (tableId : RecordRepr → Identity)
    (recordId : Generic.RecordIdentity Record Identity)
    {records : Generic.RecordSet Record} {table : Trace}
    (h : TableEncodesRecordIds cfg tableId recordId records table) :
    recordIdsOfTable cfg tableId table = Finset.image recordId.toIdentity records := by
  ext identity
  rw [mem_recordIdsOfTable_iff]
  exact (h identity).symm

theorem recordIds_generated_eq_iff_records_eq {Record Identity : Type}
    [DecidableEq Record] [DecidableEq Identity]
    (cfg : Config) (tableId : RecordRepr → Identity)
    (recordId : Generic.RecordIdentity Record Identity)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : TableGeneratorFaithfulToIds cfg tableId recordId generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    recordIdsOfTable cfg tableId (generateTable records₁) =
      recordIdsOfTable cfg tableId (generateTable records₂) ↔
    records₁ = records₂ := by
  rw [recordIdsOfTable_eq_recordIds cfg tableId recordId (hgen records₁),
      recordIdsOfTable_eq_recordIds cfg tableId recordId (hgen records₂)]
  constructor
  · intro h
    exact Finset.image_injective recordId.injective h
  · intro h
    rw [h]

end Zebra.CPU
