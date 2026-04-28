/-
Zebra — ALU canonicalizer model.

This file contains the ALU-specific canonical representation and thin wrappers
around the generic canonicalizer theorem layer.
-/
import Zebra.Canonicalizer.Generic

namespace Zebra.ALU

/-- The canonical operand tuple: byte-limb groups for inputs and result. -/
structure Tuple where
  b : List Interval
  c : List Interval
  a : List Interval
deriving DecidableEq, Repr

/-- A concrete semantic ALU event. `Value` is the VM's concrete value type
    (for example words or byte limbs). -/
structure ALUEvent (Value : Type) where
  input0 : Value
  input1 : Value
  output : Value
deriving DecidableEq, Repr

/-- The semantic ALU events recorded by VM execution. -/
abbrev EventSet (Value : Type) := Finset (ALUEvent Value)

/-- How concrete ALU events are represented by the canonicalizer's tuple
    format. Injectivity is the condition needed for a one-to-one theorem on
    full event sets. -/
abbrev ALUEventEncoding (Value : Type) :=
  Generic.EventEncoding (ALUEvent Value) Tuple

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

/-- Convert the ALU-specific config to the generic canonicalizer config. -/
def Config.toGeneric (cfg : Config) : Generic.Config Tuple where
  isReal := cfg.isReal
  projectRow := cfg.projectRow

/-- The ALU canonical form: deduplicated set of `(b, c, a)` tuples over real rows. -/
def canonicalize (cfg : Config) (t : Trace) : Finset Tuple :=
  Generic.canonicalize cfg.toGeneric t

/-- A table encodes an ALU event set when its real rows, after projection, are
    exactly the canonical tuple representation of those events. -/
def TableEncodesEvents {Value : Type} (cfg : Config)
    (enc : ALUEventEncoding Value)
    (events : EventSet Value) (table : Trace) : Prop :=
  Generic.TableEncodesEvents cfg.toGeneric enc events table

/-- A table generator is faithful when every generated ALU table encodes exactly
    the event set it was generated from. -/
def TableGeneratorFaithful {Value : Type} (cfg : Config)
    (enc : ALUEventEncoding Value)
    (generateTable : EventSet Value → Trace) : Prop :=
  Generic.TableGeneratorFaithful cfg.toGeneric enc generateTable

/-- A VM execution is canonicalized by first extracting its semantic event set,
    then generating the corresponding ALU table. -/
def generatedTableOfExecution {VMExecution Value : Type}
    (execEvents : VMExecution → EventSet Value)
    (generateTable : EventSet Value → Trace)
    (exec : VMExecution) : Trace :=
  generateTable (execEvents exec)

/-- A tuple is in the ALU canonical form iff it is the projection of some real
    row of the table. -/
lemma mem_canonicalize_iff (cfg : Config) (t : Trace) (tup : Tuple) :
    tup ∈ canonicalize cfg t ↔ ∃ row ∈ t, cfg.isReal row ∧ cfg.projectRow row = tup :=
  Generic.mem_canonicalize_iff cfg.toGeneric t tup

lemma canonicalize_eq_eventTupleSet_of_encodes {Value : Type} (cfg : Config)
    (enc : ALUEventEncoding Value)
    {events : EventSet Value} {table : Trace}
    (h : TableEncodesEvents cfg enc events table) :
    canonicalize cfg table = Finset.image enc.toRepr events :=
  Generic.canonicalize_eq_eventReprSet_of_encodes cfg.toGeneric enc h

lemma canonicalize_eq_eventTupleSet_iff {Value : Type} (cfg : Config)
    (enc : ALUEventEncoding Value)
    (events : EventSet Value) (table : Trace) :
    canonicalize cfg table = Finset.image enc.toRepr events ↔
      TableEncodesEvents cfg enc events table := by
  constructor
  · intro h tup
    rw [← h]
    exact mem_canonicalize_iff cfg table tup
  · exact canonicalize_eq_eventTupleSet_of_encodes cfg enc

/-- Main correctness theorem for ALU tables. -/
theorem canonicalize_generated_table_eq_eventTupleSet {Value : Type}
    (cfg : Config)
    (enc : ALUEventEncoding Value)
    (generateTable : EventSet Value → Trace)
    (hgen : TableGeneratorFaithful cfg enc generateTable)
    (events : EventSet Value) :
    canonicalize cfg (generateTable events) = Finset.image enc.toRepr events :=
  Generic.canonicalize_generated_table_eq_eventReprSet cfg.toGeneric enc generateTable hgen events

/-- Main one-to-one theorem for ALU tables. -/
theorem canonicalize_generated_eq_iff_events_eq {Value : Type}
    (cfg : Config)
    (enc : ALUEventEncoding Value)
    (generateTable : EventSet Value → Trace)
    (hgen : TableGeneratorFaithful cfg enc generateTable)
    (events₁ events₂ : EventSet Value) :
    canonicalize cfg (generateTable events₁) =
      canonicalize cfg (generateTable events₂) ↔
    events₁ = events₂ :=
  Generic.canonicalize_generated_eq_iff_events_eq cfg.toGeneric enc generateTable hgen
    events₁ events₂

lemma canonicalize_execution_table_eq_eventTupleSet {VMExecution Value : Type}
    (cfg : Config)
    (enc : ALUEventEncoding Value)
    (execEvents : VMExecution → EventSet Value)
    (generateTable : EventSet Value → Trace)
    (hgen : TableGeneratorFaithful cfg enc generateTable)
    (exec : VMExecution) :
    canonicalize cfg (generatedTableOfExecution execEvents generateTable exec) =
      Finset.image enc.toRepr (execEvents exec) :=
  canonicalize_generated_table_eq_eventTupleSet cfg enc generateTable hgen
    (execEvents exec)

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

/-! ## Helper lemmas -/

lemma canonicalize_nil (cfg : Config) :
    canonicalize cfg [] = ∅ := by simp [canonicalize, Generic.canonicalize]

lemma canonicalize_no_real (cfg : Config) (t : Trace)
    (h : ∀ row ∈ t, cfg.isReal row = false) :
    canonicalize cfg t = ∅ := by
  unfold canonicalize Generic.canonicalize Config.toGeneric
  have hfilt : t.filter cfg.isReal = [] := by
    apply List.filter_eq_nil_iff.mpr
    intro row hrow
    rw [h row hrow]
    decide
  rw [hfilt]
  simp

lemma canonicalize_append (cfg : Config) (t₁ t₂ : Trace) :
    canonicalize cfg (t₁ ++ t₂) = canonicalize cfg t₁ ∪ canonicalize cfg t₂ := by
  unfold canonicalize Generic.canonicalize
  rw [List.filter_append, List.map_append, List.toFinset_append]

lemma canonicalize_append_padding (cfg : Config) (t : Trace) (pad : Row)
    (h : cfg.isReal pad = false) :
    canonicalize cfg (t ++ [pad]) = canonicalize cfg t := by
  rw [canonicalize_append]
  have : canonicalize cfg [pad] = ∅ :=
    canonicalize_no_real cfg [pad] (fun row hrow => by
      rw [List.mem_singleton] at hrow
      rw [hrow]; exact h)
  rw [this, Finset.union_empty]

lemma canonicalize_perm (cfg : Config) {t₁ t₂ : Trace} (h : t₁.Perm t₂) :
    canonicalize cfg t₁ = canonicalize cfg t₂ := by
  unfold canonicalize Generic.canonicalize
  exact List.toFinset_eq_of_perm _ _ ((h.filter _).map _)

/-! ## String presentation -/

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

/-- Printer consistency, forward direction. -/
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
  isReal := fun _ => true

example (t : Trace) (pad : Row) (h : sp1AddConfig.isReal pad = false) :
    canonicalize sp1AddConfig (t ++ [pad]) = canonicalize sp1AddConfig t :=
  canonicalize_append_padding sp1AddConfig t pad h

example (t₁ t₂ : Trace) :
    canonicalize sp1AddConfig (t₁ ++ t₂)
      = canonicalize sp1AddConfig t₁ ∪ canonicalize sp1AddConfig t₂ :=
  canonicalize_append sp1AddConfig t₁ t₂

example (t : Trace) (tup : Tuple) (h : tup ∈ canonicalize sp1AddConfig t) :
    ∃ row ∈ t, sp1AddConfig.isReal row ∧ sp1AddConfig.projectRow row = tup :=
  (mem_canonicalize_iff sp1AddConfig t tup).mp h

end Zebra.ALU
