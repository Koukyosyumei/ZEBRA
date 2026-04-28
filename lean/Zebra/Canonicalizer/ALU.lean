/-
Zebra — Canonicalizer faithfulness for ALU tables (multi-row).

Models the per-row projection canonicalizer pattern shared by ALU and
memory-op tables (`src/canonicalizer.rs:56` and `:90`). For each "real" row,
project the configured operand columns to a `(b, c, a)` tuple; the canonical
form is the deduplicated *set* of those projections (matching the Rust
`HashSet<…>` semantics).

The canonicalized representation is a `Finset Tuple` (a SET, not a string).
The semantic VM events are modeled separately as concrete ALU execution events,
not abstract intervals. The canonicalizer is proved faithful to an explicit
projection from ALU events to canonical tuples.

The string output is a separate printer over this set — trivially deterministic
in the forward direction; injectivity at the string level is a separate
"printer correctness" concern (see `stringRepr_consistent`).

Build: `lake build` (uses mathlib via local symlinks under `.lake/packages/`).
-/
import Mathlib.Data.Finset.Basic
import Mathlib.Data.Finset.Dedup
import Mathlib.Data.Finset.Image
import Mathlib.Data.Finset.Lattice.Lemmas

namespace Zebra

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

/-! ## Generic table canonicalizer -/

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

namespace ALU

/-- The canonical operand tuple: byte-limb groups for inputs and result. -/
structure Tuple where
  b : List Interval
  c : List Interval
  a : List Interval
deriving DecidableEq, Repr

/-- Example canonical representation shape for memory read/write tables, matching
    the rows emitted by `generate_memory_op_final_checker`: clock plus operand
    accesses and the memory access. -/
structure MemoryOpTuple where
  clk : Interval
  opA : List Interval
  opB : List Interval
  opC : List Interval
  mem : List Interval
deriving DecidableEq, Repr

/-- Example canonical representation shape for control-flow tables: current
    program counter, next program counter, and operand groups used by branch or
    jump logic. Specific VMs can choose a richer representation if needed. -/
structure ControlFlowTuple where
  pc : List Interval
  nextPc : List Interval
  nextNextPc : List Interval
  opA : List Interval
  opB : List Interval
  opC : List Interval
deriving DecidableEq, Repr

/-- Canonical representation shape for unary ALU-like tables such as CLO/CLZ. -/
structure UnaryTuple where
  input0 : List Interval
  output : List Interval
deriving DecidableEq, Repr

/-- Canonical representation shape for Ziren conditional move. -/
structure MovCondTuple where
  opA : List Interval
  prevA : List Interval
  opB : List Interval
  opC : List Interval
deriving DecidableEq, Repr

/-- Canonical representation shape for Valida LT32, whose output is a single
    field rather than a four-limb word in the current example. -/
structure ValidaLtTuple where
  input0 : List Interval
  input1 : List Interval
  output : Interval
deriving DecidableEq, Repr

/-- A concrete semantic ALU event. `Value` is the VM's concrete value type
    (for example words or byte limbs). -/
structure ALUEvent (Value : Type) where
  input0 : Value
  input1 : Value
  output : Value
deriving DecidableEq, Repr

/-- The semantic events recorded by VM execution. This is intentionally
    separate from the canonicalized representation. -/
abbrev EventSet (Value : Type) := Finset (ALUEvent Value)

/-- How concrete ALU events are represented by the canonicalizer's tuple
    format. Injectivity is the condition needed for a one-to-one theorem on
    full event sets rather than only on their represented tuple images. -/
structure ALUEventEncoding (Value : Type) where
  toTuple : ALUEvent Value → Tuple
  injective : Function.Injective toTuple

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
    exactly the canonical tuple representation of the semantic events. This
    deliberately ignores helper columns, row order, padding rows, and duplicated
    events. -/
def TableEncodesEvents {Value : Type} (cfg : Config)
    (enc : ALUEventEncoding Value)
    (events : EventSet Value) (table : Trace) : Prop :=
  ∀ tup, tup ∈ Finset.image enc.toTuple events ↔
    ∃ row ∈ table, cfg.isReal row ∧ cfg.projectRow row = tup

/-- A table generator is faithful when, for every semantic event set, the
    generated table encodes exactly that event set. -/
def TableGeneratorFaithful {Value : Type} (cfg : Config)
    (enc : ALUEventEncoding Value)
    (generateTable : EventSet Value → Trace) : Prop :=
  ∀ events, TableEncodesEvents cfg enc events (generateTable events)

/-- A VM execution is canonicalized by first extracting its semantic event set,
    then generating the corresponding table. -/
def generatedTableOfExecution {VMExecution Value : Type}
    (execEvents : VMExecution → EventSet Value)
    (generateTable : EventSet Value → Trace)
    (exec : VMExecution) : Trace :=
  generateTable (execEvents exec)

/-! ## (ii) Canonicalizer membership -/

/-- A tuple is in the canonical form of `t` iff it is the projection of some
    real row of `t`. -/
lemma mem_canonicalize_iff (cfg : Config) (t : Trace) (tup : Tuple) :
    tup ∈ canonicalize cfg t ↔ ∃ row ∈ t, cfg.isReal row ∧ cfg.projectRow row = tup := by
  unfold canonicalize
  simp [List.mem_toFinset, List.mem_map, List.mem_filter]
  constructor
  · rintro ⟨row, ⟨hin, hreal⟩, heq⟩
    exact ⟨row, hin, hreal, heq⟩
  · rintro ⟨row, hin, hreal, heq⟩
    exact ⟨row, ⟨hin, hreal⟩, heq⟩

/-! ## (iii) Semantic event-set layer -/

/-- If a table encodes an event set, Zebra's canonicalizer returns exactly that
    event set's tuple projection. -/
lemma canonicalize_eq_eventTupleSet_of_encodes {Value : Type} (cfg : Config)
    (enc : ALUEventEncoding Value)
    {events : EventSet Value} {table : Trace}
    (h : TableEncodesEvents cfg enc events table) :
    canonicalize cfg table = Finset.image enc.toTuple events := by
  ext tup
  rw [mem_canonicalize_iff]
  exact (h tup).symm

/-- The canonicalizer returns a canonical event projection exactly when the
    table encodes that event set. This is the table-level specification of the
    canonicalizer. -/
lemma canonicalize_eq_eventTupleSet_iff {Value : Type} (cfg : Config)
    (enc : ALUEventEncoding Value)
    (events : EventSet Value) (table : Trace) :
    canonicalize cfg table = Finset.image enc.toTuple events ↔
      TableEncodesEvents cfg enc events table := by
  constructor
  · intro h tup
    rw [← h]
    exact mem_canonicalize_iff cfg table tup
  · exact canonicalize_eq_eventTupleSet_of_encodes cfg enc

/-! ## (iv) Abstract table-generator bridge -/

/-- **Main correctness theorem.** If the table generator is faithful, Zebra
    recovers exactly the canonical tuple projection of the semantic event set
    used to generate the table. -/
theorem canonicalize_generated_table_eq_eventTupleSet {Value : Type}
    (cfg : Config)
    (enc : ALUEventEncoding Value)
    (generateTable : EventSet Value → Trace)
    (hgen : TableGeneratorFaithful cfg enc generateTable)
    (events : EventSet Value) :
    canonicalize cfg (generateTable events) = Finset.image enc.toTuple events :=
  canonicalize_eq_eventTupleSet_of_encodes cfg enc (hgen events)

/-- **Main one-to-one theorem.** For faithful generated tables, equality of
    Zebra canonicalized representations is exactly equality of the original
    ALU event sets. -/
theorem canonicalize_generated_eq_iff_events_eq {Value : Type}
    (cfg : Config)
    (enc : ALUEventEncoding Value)
    (generateTable : EventSet Value → Trace)
    (hgen : TableGeneratorFaithful cfg enc generateTable)
    (events₁ events₂ : EventSet Value) :
    canonicalize cfg (generateTable events₁) =
      canonicalize cfg (generateTable events₂) ↔
    events₁ = events₂ :=
  by
    rw [canonicalize_generated_table_eq_eventTupleSet cfg enc generateTable hgen events₁,
        canonicalize_generated_table_eq_eventTupleSet cfg enc generateTable hgen events₂]
    constructor
    · intro h
      exact Finset.image_injective enc.injective h
    · intro h
      rw [h]

/-! ## (v) Optional VM execution bridge -/

/-- With a faithful event-set-to-table generator, Zebra recovers the canonical
    tuple projection of the event set recorded by a VM execution. -/
lemma canonicalize_execution_table_eq_eventTupleSet {VMExecution Value : Type}
    (cfg : Config)
    (enc : ALUEventEncoding Value)
    (execEvents : VMExecution → EventSet Value)
    (generateTable : EventSet Value → Trace)
    (hgen : TableGeneratorFaithful cfg enc generateTable)
    (exec : VMExecution) :
    canonicalize cfg (generatedTableOfExecution execEvents generateTable exec) =
      Finset.image enc.toTuple (execEvents exec) :=
  canonicalize_generated_table_eq_eventTupleSet cfg enc generateTable hgen
    (execEvents exec)

/-- With a faithful event-set-to-table generator, equality of canonicalized VM
    tables is exactly equality of the executions' canonical event projections. -/
lemma canonicalize_execution_tables_eq_iff_events_eq {VMExecution Value : Type}
    (cfg : Config)
    (enc : ALUEventEncoding Value)
    (execEvents : VMExecution → EventSet Value)
    (generateTable : EventSet Value → Trace)
    (hgen : TableGeneratorFaithful cfg enc generateTable)
    (exec₁ exec₂ : VMExecution) :
    canonicalize cfg (generatedTableOfExecution execEvents generateTable exec₁) =
      canonicalize cfg (generatedTableOfExecution execEvents generateTable exec₂) ↔
    execEvents exec₁ = execEvents exec₂ :=
  canonicalize_generated_eq_iff_events_eq cfg enc generateTable hgen
    (execEvents exec₁) (execEvents exec₂)

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

/-! ## (vii) Example table layouts used by the Rust experiments -/

namespace Examples

/-- Examples below focus on canonical representation shape and column layout.
    The real-row predicate comes from lookup selector expressions in Rust, so it
    is left as a parameter here. -/
def mkALUConfig (idxB idxC idxA : List Nat) (isReal : Row → Bool) :
    Generic.Config Tuple where
  isReal := isReal
  projectRow := fun row =>
    { b := row.project idxB, c := row.project idxC, a := row.project idxA }

def mkUnaryConfig (input output : List Nat) (isReal : Row → Bool) :
    Generic.Config UnaryTuple where
  isReal := isReal
  projectRow := fun row =>
    { input0 := row.project input, output := row.project output }

def mkMemoryOpConfig (clk : Nat) (opA opB opC mem : List Nat) (isReal : Row → Bool) :
    Generic.Config MemoryOpTuple where
  isReal := isReal
  projectRow := fun row =>
    { clk := row[clk]?.getD { lo := 0, hi := 0 },
      opA := row.project opA,
      opB := row.project opB,
      opC := row.project opC,
      mem := row.project mem }

def mkControlFlowConfig (pc nextPc nextNextPc opA opB opC : List Nat) (isReal : Row → Bool) :
    Generic.Config ControlFlowTuple where
  isReal := isReal
  projectRow := fun row =>
    { pc := row.project pc,
      nextPc := row.project nextPc,
      nextNextPc := row.project nextNextPc,
      opA := row.project opA,
      opB := row.project opB,
      opC := row.project opC }

def mkMovCondConfig (opA prevA opB opC : List Nat) (isReal : Row → Bool) :
    Generic.Config MovCondTuple where
  isReal := isReal
  projectRow := fun row =>
    { opA := row.project opA,
      prevA := row.project prevA,
      opB := row.project opB,
      opC := row.project opC }

def mkValidaLtConfig (input0 input1 : List Nat) (output : Nat) (isReal : Row → Bool) :
    Generic.Config ValidaLtTuple where
  isReal := isReal
  projectRow := fun row =>
    { input0 := row.project input0,
      input1 := row.project input1,
      output := row[output]?.getD { lo := 0, hi := 0 } }

/-! ### SP1 -/

namespace SP1

def addConfig (isReal : Row → Bool) : Generic.Config Tuple :=
  mkALUConfig [8, 9, 10, 11] [12, 13, 14, 15] [1, 2, 3, 4] isReal

def subConfig (isReal : Row → Bool) : Generic.Config Tuple :=
  mkALUConfig [1, 2, 3, 4] [12, 13, 14, 15] [8, 9, 10, 11] isReal

/-- Shared by SP1 branch and jump examples. -/
def controlFlowConfig (isReal : Row → Bool) : Generic.Config ControlFlowTuple :=
  mkControlFlowConfig [0, 1, 2, 3] [5, 6, 7, 8] []
    [10, 11, 12, 13] [14, 15, 16, 17] [18, 19, 20, 21] isReal

def memoryInstrsConfig (isReal : Row → Bool) : Generic.Config MemoryOpTuple :=
  mkMemoryOpConfig 2 [3, 4, 5, 6] [7, 8, 9, 10] [11, 12, 13, 14]
    [38, 39, 40, 41] isReal

/-- Tables using `generate_alu_final_checker`, whose columns are supplied by
    `GeneralLookupInfo` at extraction time. -/
def lookupDrivenALUTables : List String :=
  ["lt", "bitwise", "divrem", "shiftleft", "mul", "sr"]

end SP1

/-! ### Pico -/

namespace Pico

def addConfig (isReal : Row → Bool) : Generic.Config Tuple :=
  mkALUConfig [7, 8, 9, 10] [11, 12, 13, 14] [0, 1, 2, 3] isReal

def subConfig (isReal : Row → Bool) : Generic.Config Tuple :=
  mkALUConfig [0, 1, 2, 3] [11, 12, 13, 14] [7, 8, 9, 10] isReal

def memoryReadWriteConfig (isReal : Row → Bool) : Generic.Config MemoryOpTuple :=
  mkMemoryOpConfig 1 [68, 69, 70, 71] [77, 78, 79, 80] [86, 87, 88, 89]
    [28, 29, 30, 31] isReal

def lookupDrivenALUTables : List String :=
  ["sr", "sll", "lessthan", "mul", "bitwise", "divrem"]

end Pico

/-! ### Sphinx -/

namespace Sphinx

def addConfig (isReal : Row → Bool) : Generic.Config Tuple :=
  mkALUConfig [10, 11, 12, 13] [14, 15, 16, 17] [3, 4, 5, 6] isReal

def subConfig (isReal : Row → Bool) : Generic.Config Tuple :=
  mkALUConfig [3, 4, 5, 6] [14, 15, 16, 17] [10, 11, 12, 13] isReal

def lookupDrivenALUTables : List String :=
  ["sr", "shiftleft", "mul", "lt", "bitwise", "divrem"]

end Sphinx

/-! ### Ziren -/

namespace Ziren

def addConfig (isReal : Row → Bool) : Generic.Config Tuple :=
  mkALUConfig [9, 10, 11, 12] [13, 14, 15, 16] [2, 3, 4, 5] isReal

def subConfig (isReal : Row → Bool) : Generic.Config Tuple :=
  mkALUConfig [2, 3, 4, 5] [13, 14, 15, 16] [9, 10, 11, 12] isReal

def divConfig (isReal : Row → Bool) : Generic.Config Tuple :=
  mkALUConfig [2, 3, 4, 5] [6, 7, 8, 9] [10, 11, 12, 13] isReal

def remConfig (isReal : Row → Bool) : Generic.Config Tuple :=
  mkALUConfig [2, 3, 4, 5] [6, 7, 8, 9] [14, 15, 16, 17] isReal

def cloClzConfig (isReal : Row → Bool) : Generic.Config UnaryTuple :=
  mkUnaryConfig [6, 7, 8, 9] [2, 3, 4, 5] isReal

def movCondConfig (isReal : Row → Bool) : Generic.Config MovCondTuple :=
  mkMovCondConfig [2, 3, 4, 5] [6, 7, 8, 9] [10, 11, 12, 13] [14, 15, 16, 17]
    isReal

def branchConfig (isReal : Row → Bool) : Generic.Config ControlFlowTuple :=
  mkControlFlowConfig [0] [1, 2, 3, 4] [23, 24, 25, 26]
    [41, 42, 43, 44] [45, 46, 47, 48] [49, 50, 51, 52] isReal

def jumpConfig (isReal : Row → Bool) : Generic.Config ControlFlowTuple :=
  mkControlFlowConfig [0] [1, 2, 3, 4] [19, 20, 21, 22]
    [37, 38, 39, 40] [41, 42, 43, 44] [45, 46, 47, 48] isReal

def memoryInstrsConfig (isReal : Row → Bool) : Generic.Config MemoryOpTuple :=
  mkMemoryOpConfig 3 [4, 5, 6, 7] [8, 9, 10, 11] [12, 13, 14, 15]
    [57, 58, 59, 60] isReal

def lookupDrivenALUTables : List String :=
  ["mul", "shiftleft", "shiftright", "lt", "bitwise"]

end Ziren

/-! ### Valida -/

namespace Valida

def lt32Config (isReal : Row → Bool) : Generic.Config ValidaLtTuple :=
  mkValidaLtConfig [0, 1, 2, 3] [4, 5, 6, 7] 21 isReal

/-- Valida memory is intentionally skipped: its canonicalizer is not yet
    supported in Zebra. -/
def lookupDrivenALUTables : List String :=
  ["add32", "sub32", "mul32", "div32", "bitwise32", "com32"]

end Valida

/-! ### OpenVM -/

namespace OpenVM

/-- OpenVM examples currently use the lookup-driven ALU final checker. CPU
    tables are intentionally skipped. -/
def lookupDrivenALUTables : List String :=
  ["alu", "bitwise", "branch", "jump", "lt", "mul", "shift"]

end OpenVM

end Examples

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

end ALU
end Zebra
