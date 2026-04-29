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

/-- A tuple is in the ALU canonical form iff it is the projection of some real
    row of the table. -/
lemma mem_canonicalize_iff (cfg : Config) (t : Trace) (tup : Tuple) :
    tup ∈ canonicalize cfg t ↔ ∃ row ∈ t, cfg.isReal row ∧ cfg.projectRow row = tup :=
  Generic.mem_canonicalize_iff cfg.toGeneric t tup

end Zebra.ALU
