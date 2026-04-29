/-
Zebra — generic table canonicalizer core.
-/
import Mathlib.Data.Finset.Basic
import Mathlib.Data.Finset.Dedup
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

end Generic
end Zebra
