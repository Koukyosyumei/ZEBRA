/-
Zebra — control-flow and miscellaneous hand-written canonical representation
shapes used by example final checkers.
-/
import Zebra.Canonicalizer.Generic

namespace Zebra.ControlFlow

/-- Canonical representation shape for control-flow tables: current program
    counter, next program counter, optional next-next program counter, and
    operand groups used by branch or jump logic. -/
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

def mkUnaryConfig (input output : List Nat) (isReal : Row → Bool) :
    Generic.Config UnaryTuple where
  isReal := isReal
  projectRow := fun row =>
    { input0 := row.project input, output := row.project output }

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

end Zebra.ControlFlow
