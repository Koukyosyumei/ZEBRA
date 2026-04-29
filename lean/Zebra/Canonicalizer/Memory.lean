/-
Zebra — memory-operation canonical representation shapes.
-/
import Zebra.Canonicalizer.Generic
import Zebra.Canonicalizer.Generator

namespace Zebra.Memory

/-- Canonical representation shape for memory read/write tables, matching the
    rows emitted by `generate_memory_op_final_checker`: clock plus operand
    accesses and the memory access. -/
structure MemoryOpTuple where
  clk : Interval
  opA : List Interval
  opB : List Interval
  opC : List Interval
  mem : List Interval
deriving DecidableEq, Repr

/-- Build a generic memory-op canonicalizer config from column indices. -/
def mkMemoryOpConfig (clk : Nat) (opA opB opC mem : List Nat) (isReal : Row → Bool) :
    Generic.Config MemoryOpTuple where
  isReal := isReal
  projectRow := fun row =>
    { clk := row[clk]?.getD { lo := 0, hi := 0 },
      opA := row.project opA,
      opB := row.project opB,
      opC := row.project opC,
      mem := row.project mem }

/-- Direction of a canonical memory-table access. -/
inductive AccessKind where
  | read
  | write
deriving DecidableEq, Repr

/-- Canonical representation for Valida-style memory rows: clock, address,
    value, and whether the access is a read or write. -/
structure AccessTuple where
  clk : Interval
  addr : Interval
  value : List Interval
  kind : AccessKind
deriving DecidableEq, Repr

/-- Layout for memory tables whose rows may encode read and/or write accesses. -/
structure AccessConfig where
  isRead : Row → Bool
  isWrite : Row → Bool
  clk : Nat
  addr : Nat
  value : List Nat

def accessOfRow (cfg : AccessConfig) (kind : AccessKind) (row : Row) : AccessTuple :=
  { clk := row[cfg.clk]?.getD { lo := 0, hi := 0 },
    addr := row[cfg.addr]?.getD { lo := 0, hi := 0 },
    value := row.project cfg.value,
    kind := kind }

def rowAccesses (cfg : AccessConfig) (row : Row) : List AccessTuple :=
  (if cfg.isRead row then [accessOfRow cfg AccessKind.read row] else []) ++
    (if cfg.isWrite row then [accessOfRow cfg AccessKind.write row] else [])

/-- Memory-table canonicalizer for layouts such as Valida memory: flatten all
    read/write accesses and deduplicate them as a finite set. -/
def canonicalizeAccesses (cfg : AccessConfig) (t : Trace) : Finset AccessTuple :=
  (t.flatMap (rowAccesses cfg)).toFinset

lemma mem_canonicalizeAccesses_iff (cfg : AccessConfig) (t : Trace) (repr : AccessTuple) :
    repr ∈ canonicalizeAccesses cfg t ↔
      ∃ row ∈ t, repr ∈ rowAccesses cfg row := by
  unfold canonicalizeAccesses
  simp [List.mem_flatMap]

def TableEncodesAccessRecords {Record : Type}
    (cfg : AccessConfig) (recordId : Generic.RecordIdentity Record AccessTuple)
    (records : Generic.RecordSet Record) (table : Trace) : Prop :=
  ∀ repr, repr ∈ Finset.image recordId.toIdentity records ↔
    ∃ row ∈ table, repr ∈ rowAccesses cfg row

def AccessTableGeneratorFaithful {Record : Type}
    (cfg : AccessConfig) (recordId : Generic.RecordIdentity Record AccessTuple)
    (generateTable : Generic.RecordSet Record → Trace) : Prop :=
  ∀ records, TableEncodesAccessRecords cfg recordId records (generateTable records)

lemma canonicalizeAccesses_eq_recordIdentitySet_of_encodes {Record : Type}
    (cfg : AccessConfig) (recordId : Generic.RecordIdentity Record AccessTuple)
    {records : Generic.RecordSet Record} {table : Trace}
    (h : TableEncodesAccessRecords cfg recordId records table) :
    canonicalizeAccesses cfg table = Finset.image recordId.toIdentity records := by
  ext repr
  rw [mem_canonicalizeAccesses_iff]
  exact (h repr).symm

theorem canonicalizeAccesses_generated_table_eq_recordIdentitySet {Record : Type}
    (cfg : AccessConfig) (recordId : Generic.RecordIdentity Record AccessTuple)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : AccessTableGeneratorFaithful cfg recordId generateTable)
    (records : Generic.RecordSet Record) :
    canonicalizeAccesses cfg (generateTable records) = Finset.image recordId.toIdentity records :=
  canonicalizeAccesses_eq_recordIdentitySet_of_encodes cfg recordId (hgen records)

theorem canonicalizeAccesses_generator_independent {Record : Type}
    (cfg : AccessConfig) (recordId : Generic.RecordIdentity Record AccessTuple)
    (gen₁ gen₂ : Generic.RecordSet Record → Trace)
    (h₁ : AccessTableGeneratorFaithful cfg recordId gen₁)
    (h₂ : AccessTableGeneratorFaithful cfg recordId gen₂)
    (records : Generic.RecordSet Record) :
    canonicalizeAccesses cfg (gen₁ records) = canonicalizeAccesses cfg (gen₂ records) := by
  rw [canonicalizeAccesses_generated_table_eq_recordIdentitySet cfg recordId gen₁ h₁ records,
      canonicalizeAccesses_generated_table_eq_recordIdentitySet cfg recordId gen₂ h₂ records]

theorem canonicalizeAccesses_generated_eq_iff_records_eq {Record : Type}
    [DecidableEq Record]
    (cfg : AccessConfig) (recordId : Generic.RecordIdentity Record AccessTuple)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : AccessTableGeneratorFaithful cfg recordId generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    canonicalizeAccesses cfg (generateTable records₁) =
      canonicalizeAccesses cfg (generateTable records₂) ↔
    records₁ = records₂ := by
  rw [canonicalizeAccesses_generated_table_eq_recordIdentitySet cfg recordId generateTable hgen records₁,
      canonicalizeAccesses_generated_table_eq_recordIdentitySet cfg recordId generateTable hgen records₂]
  constructor
  · intro h
    exact Finset.image_injective recordId.injective h
  · intro h
    rw [h]

end Zebra.Memory
