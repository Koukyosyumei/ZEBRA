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

/-- Record identities reconstructed from an access-style memory table. `tableId`
    interprets the memory access tuple as the abstract identity of an execution
    record. -/
def recordIdsOfAccessTable {Identity : Type} [DecidableEq Identity]
    (cfg : AccessConfig) (tableId : AccessTuple → Identity) (table : Trace) : Finset Identity :=
  Finset.image tableId (canonicalizeAccesses cfg table)

lemma mem_recordIdsOfAccessTable_iff {Identity : Type} [DecidableEq Identity]
    (cfg : AccessConfig) (tableId : AccessTuple → Identity) (table : Trace) (identity : Identity) :
    identity ∈ recordIdsOfAccessTable cfg tableId table ↔
      ∃ row ∈ table, ∃ repr ∈ rowAccesses cfg row, tableId repr = identity := by
  unfold recordIdsOfAccessTable
  constructor
  · intro h
    rcases Finset.mem_image.mp h with ⟨repr, hrepr, hidentity⟩
    rw [mem_canonicalizeAccesses_iff] at hrepr
    rcases hrepr with ⟨row, hrow, hreprInRow⟩
    exact ⟨row, hrow, repr, hreprInRow, hidentity⟩
  · rintro ⟨row, hrow, repr, hreprInRow, hidentity⟩
    apply Finset.mem_image.mpr
    exact ⟨repr, (mem_canonicalizeAccesses_iff cfg table repr).mpr
      ⟨row, hrow, hreprInRow⟩, hidentity⟩

def AccessTableEncodesRecordIds {Record Identity : Type} [DecidableEq Identity]
    (cfg : AccessConfig) (tableId : AccessTuple → Identity)
    (recordId : Generic.RecordIdentity Record Identity)
    (records : Generic.RecordSet Record) (table : Trace) : Prop :=
  ∀ identity, identity ∈ Finset.image recordId.toIdentity records ↔
    ∃ row ∈ table, ∃ repr ∈ rowAccesses cfg row, tableId repr = identity

def AccessTableGeneratorFaithfulToIds {Record Identity : Type}
    [DecidableEq Identity]
    (cfg : AccessConfig) (tableId : AccessTuple → Identity)
    (recordId : Generic.RecordIdentity Record Identity)
    (generateTable : Generic.RecordSet Record → Trace) : Prop :=
  ∀ records, AccessTableEncodesRecordIds cfg tableId recordId records (generateTable records)

lemma recordIdsOfAccessTable_eq_recordIds {Record Identity : Type}
    [DecidableEq Identity]
    (cfg : AccessConfig) (tableId : AccessTuple → Identity)
    (recordId : Generic.RecordIdentity Record Identity)
    {records : Generic.RecordSet Record} {table : Trace}
    (h : AccessTableEncodesRecordIds cfg tableId recordId records table) :
    recordIdsOfAccessTable cfg tableId table = Finset.image recordId.toIdentity records := by
  ext identity
  rw [mem_recordIdsOfAccessTable_iff]
  exact (h identity).symm

theorem recordIdsOfAccessTable_eq_iff_records_eq {Record Identity : Type}
    [DecidableEq Record] [DecidableEq Identity]
    (cfg : AccessConfig) (tableId : AccessTuple → Identity)
    (recordId : Generic.RecordIdentity Record Identity)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : AccessTableGeneratorFaithfulToIds cfg tableId recordId generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    recordIdsOfAccessTable cfg tableId (generateTable records₁) =
      recordIdsOfAccessTable cfg tableId (generateTable records₂) ↔
    records₁ = records₂ := by
  rw [recordIdsOfAccessTable_eq_recordIds cfg tableId recordId
        (hgen records₁),
      recordIdsOfAccessTable_eq_recordIds cfg tableId recordId
        (hgen records₂)]
  constructor
  · intro h
    exact Finset.image_injective recordId.injective h
  · intro h
    rw [h]

end Zebra.Memory
