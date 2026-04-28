/-
Zebra — memory-operation canonical representation shapes.
-/
import Zebra.Canonicalizer.Generic

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

end Zebra.Memory
