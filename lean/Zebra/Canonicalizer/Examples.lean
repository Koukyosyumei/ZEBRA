/-
Zebra — concrete example table layouts from the Rust experiments.

CPU tables and Valida memory are intentionally skipped because their
canonicalizers are not currently supported by Zebra.
-/
import Zebra.Canonicalizer.ALU
import Zebra.Canonicalizer.ControlFlow
import Zebra.Canonicalizer.Memory

namespace Zebra.Examples

/-- Examples below focus on canonical representation shape and column layout.
    The real-row predicate comes from lookup selector expressions in Rust, so it
    is left as a parameter here. -/
def mkALUConfig (idxB idxC idxA : List Nat) (isReal : Row → Bool) :
    Generic.Config ALU.Tuple where
  isReal := isReal
  projectRow := fun row =>
    { b := row.project idxB, c := row.project idxC, a := row.project idxA }

/-! ### SP1 -/

namespace SP1

def addConfig (isReal : Row → Bool) : Generic.Config ALU.Tuple :=
  mkALUConfig [8, 9, 10, 11] [12, 13, 14, 15] [1, 2, 3, 4] isReal

def subConfig (isReal : Row → Bool) : Generic.Config ALU.Tuple :=
  mkALUConfig [1, 2, 3, 4] [12, 13, 14, 15] [8, 9, 10, 11] isReal

/-- Shared by SP1 branch and jump examples. -/
def controlFlowConfig (isReal : Row → Bool) : Generic.Config ControlFlow.ControlFlowTuple :=
  ControlFlow.mkControlFlowConfig [0, 1, 2, 3] [5, 6, 7, 8] []
    [10, 11, 12, 13] [14, 15, 16, 17] [18, 19, 20, 21] isReal

def memoryInstrsConfig (isReal : Row → Bool) : Generic.Config Memory.MemoryOpTuple :=
  Memory.mkMemoryOpConfig 2 [3, 4, 5, 6] [7, 8, 9, 10] [11, 12, 13, 14]
    [38, 39, 40, 41] isReal

/-- Tables using `generate_alu_final_checker`, whose columns are supplied by
    `GeneralLookupInfo` at extraction time. -/
def lookupDrivenALUTables : List String :=
  ["lt", "bitwise", "divrem", "shiftleft", "mul", "sr"]

end SP1

/-! ### Pico -/

namespace Pico

def addConfig (isReal : Row → Bool) : Generic.Config ALU.Tuple :=
  mkALUConfig [7, 8, 9, 10] [11, 12, 13, 14] [0, 1, 2, 3] isReal

def subConfig (isReal : Row → Bool) : Generic.Config ALU.Tuple :=
  mkALUConfig [0, 1, 2, 3] [11, 12, 13, 14] [7, 8, 9, 10] isReal

def memoryReadWriteConfig (isReal : Row → Bool) : Generic.Config Memory.MemoryOpTuple :=
  Memory.mkMemoryOpConfig 1 [68, 69, 70, 71] [77, 78, 79, 80] [86, 87, 88, 89]
    [28, 29, 30, 31] isReal

def lookupDrivenALUTables : List String :=
  ["sr", "sll", "lessthan", "mul", "bitwise", "divrem"]

end Pico

/-! ### Sphinx -/

namespace Sphinx

def addConfig (isReal : Row → Bool) : Generic.Config ALU.Tuple :=
  mkALUConfig [10, 11, 12, 13] [14, 15, 16, 17] [3, 4, 5, 6] isReal

def subConfig (isReal : Row → Bool) : Generic.Config ALU.Tuple :=
  mkALUConfig [3, 4, 5, 6] [14, 15, 16, 17] [10, 11, 12, 13] isReal

def lookupDrivenALUTables : List String :=
  ["sr", "shiftleft", "mul", "lt", "bitwise", "divrem"]

end Sphinx

/-! ### Ziren -/

namespace Ziren

def addConfig (isReal : Row → Bool) : Generic.Config ALU.Tuple :=
  mkALUConfig [9, 10, 11, 12] [13, 14, 15, 16] [2, 3, 4, 5] isReal

def subConfig (isReal : Row → Bool) : Generic.Config ALU.Tuple :=
  mkALUConfig [2, 3, 4, 5] [13, 14, 15, 16] [9, 10, 11, 12] isReal

def divConfig (isReal : Row → Bool) : Generic.Config ALU.Tuple :=
  mkALUConfig [2, 3, 4, 5] [6, 7, 8, 9] [10, 11, 12, 13] isReal

def remConfig (isReal : Row → Bool) : Generic.Config ALU.Tuple :=
  mkALUConfig [2, 3, 4, 5] [6, 7, 8, 9] [14, 15, 16, 17] isReal

def cloClzConfig (isReal : Row → Bool) : Generic.Config ControlFlow.UnaryTuple :=
  ControlFlow.mkUnaryConfig [6, 7, 8, 9] [2, 3, 4, 5] isReal

def movCondConfig (isReal : Row → Bool) : Generic.Config ControlFlow.MovCondTuple :=
  ControlFlow.mkMovCondConfig [2, 3, 4, 5] [6, 7, 8, 9] [10, 11, 12, 13]
    [14, 15, 16, 17] isReal

def branchConfig (isReal : Row → Bool) : Generic.Config ControlFlow.ControlFlowTuple :=
  ControlFlow.mkControlFlowConfig [0] [1, 2, 3, 4] [23, 24, 25, 26]
    [41, 42, 43, 44] [45, 46, 47, 48] [49, 50, 51, 52] isReal

def jumpConfig (isReal : Row → Bool) : Generic.Config ControlFlow.ControlFlowTuple :=
  ControlFlow.mkControlFlowConfig [0] [1, 2, 3, 4] [19, 20, 21, 22]
    [37, 38, 39, 40] [41, 42, 43, 44] [45, 46, 47, 48] isReal

def memoryInstrsConfig (isReal : Row → Bool) : Generic.Config Memory.MemoryOpTuple :=
  Memory.mkMemoryOpConfig 3 [4, 5, 6, 7] [8, 9, 10, 11] [12, 13, 14, 15]
    [57, 58, 59, 60] isReal

def lookupDrivenALUTables : List String :=
  ["mul", "shiftleft", "shiftright", "lt", "bitwise"]

end Ziren

/-! ### Valida -/

namespace Valida

def lt32Config (isReal : Row → Bool) : Generic.Config ControlFlow.ValidaLtTuple :=
  ControlFlow.mkValidaLtConfig [0, 1, 2, 3] [4, 5, 6, 7] 21 isReal

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

end Zebra.Examples
