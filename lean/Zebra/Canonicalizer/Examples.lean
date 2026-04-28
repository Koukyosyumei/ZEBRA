/-
Zebra — concrete example table layouts from the Rust experiments.

CPU tables and Valida memory are intentionally skipped because their
canonicalizers are not currently supported by Zebra.
-/
import Zebra.Canonicalizer.ALU
import Zebra.Canonicalizer.ControlFlow
import Zebra.Canonicalizer.Generator
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

/-- Shared guarantee for every example config below: if a table generator is
    faithful to the stated config and an injective event encoding, then the
    canonical representation is one-to-one with the original event set. -/
theorem one_to_one_for_config {Event Repr : Type} [DecidableEq Repr]
    (cfg : Generic.Config Repr)
    (enc : Generic.EventEncoding Event Repr)
    (generateTable : Generic.EventSet Event → Trace)
    (hgen : Generic.TableGeneratorFaithful cfg enc generateTable)
    (events₁ events₂ : Generic.EventSet Event) :
    Generic.canonicalize cfg (generateTable events₁) =
      Generic.canonicalize cfg (generateTable events₂) ↔
    events₁ = events₂ :=
  Generic.canonicalize_generated_eq_iff_events_eq cfg enc generateTable hgen events₁ events₂

/-- Guarantee for lookup-driven ALU tables whose concrete operand columns are
    provided by `GeneralLookupInfo` at extraction time rather than hard-coded in
    the example file. -/
theorem lookup_driven_alu_one_to_one {Event : Type}
    (cfg : Generic.Config ALU.Tuple)
    (enc : Generic.EventEncoding Event ALU.Tuple)
    (generateTable : Generic.EventSet Event → Trace)
    (hgen : Generic.TableGeneratorFaithful cfg enc generateTable)
    (events₁ events₂ : Generic.EventSet Event) :
    Generic.canonicalize cfg (generateTable events₁) =
      Generic.canonicalize cfg (generateTable events₂) ↔
    events₁ = events₂ :=
  one_to_one_for_config cfg enc generateTable hgen events₁ events₂

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

/-- SP1 ADD canonicalizer one-to-one guarantee. -/
theorem add_one_to_one {Event : Type} (isReal : Row → Bool)
    (enc : Generic.EventEncoding Event ALU.Tuple)
    (generateTable : Generic.EventSet Event → Trace)
    (hgen : Generic.TableGeneratorFaithful (addConfig isReal) enc generateTable)
    (events₁ events₂ : Generic.EventSet Event) :
    Generic.canonicalize (addConfig isReal) (generateTable events₁) =
      Generic.canonicalize (addConfig isReal) (generateTable events₂) ↔
    events₁ = events₂ :=
  one_to_one_for_config (addConfig isReal) enc generateTable hgen events₁ events₂

/-- SP1 SUB canonicalizer one-to-one guarantee. -/
theorem sub_one_to_one {Event : Type} (isReal : Row → Bool)
    (enc : Generic.EventEncoding Event ALU.Tuple)
    (generateTable : Generic.EventSet Event → Trace)
    (hgen : Generic.TableGeneratorFaithful (subConfig isReal) enc generateTable)
    (events₁ events₂ : Generic.EventSet Event) :
    Generic.canonicalize (subConfig isReal) (generateTable events₁) =
      Generic.canonicalize (subConfig isReal) (generateTable events₂) ↔
    events₁ = events₂ :=
  one_to_one_for_config (subConfig isReal) enc generateTable hgen events₁ events₂

/-- SP1 branch/jump control-flow canonicalizer one-to-one guarantee. -/
theorem controlFlow_one_to_one {Event : Type} (isReal : Row → Bool)
    (enc : Generic.EventEncoding Event ControlFlow.ControlFlowTuple)
    (generateTable : Generic.EventSet Event → Trace)
    (hgen : Generic.TableGeneratorFaithful (controlFlowConfig isReal) enc generateTable)
    (events₁ events₂ : Generic.EventSet Event) :
    Generic.canonicalize (controlFlowConfig isReal) (generateTable events₁) =
      Generic.canonicalize (controlFlowConfig isReal) (generateTable events₂) ↔
    events₁ = events₂ :=
  one_to_one_for_config (controlFlowConfig isReal) enc generateTable hgen events₁ events₂

/-- SP1 memory-instructions canonicalizer one-to-one guarantee. -/
theorem memoryInstrs_one_to_one {Event : Type} (isReal : Row → Bool)
    (enc : Generic.EventEncoding Event Memory.MemoryOpTuple)
    (generateTable : Generic.EventSet Event → Trace)
    (hgen : Generic.TableGeneratorFaithful (memoryInstrsConfig isReal) enc generateTable)
    (events₁ events₂ : Generic.EventSet Event) :
    Generic.canonicalize (memoryInstrsConfig isReal) (generateTable events₁) =
      Generic.canonicalize (memoryInstrsConfig isReal) (generateTable events₂) ↔
    events₁ = events₂ :=
  one_to_one_for_config (memoryInstrsConfig isReal) enc generateTable hgen events₁ events₂

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

/-- Pico ADD canonicalizer one-to-one guarantee. -/
theorem add_one_to_one {Event : Type} (isReal : Row → Bool)
    (enc : Generic.EventEncoding Event ALU.Tuple)
    (generateTable : Generic.EventSet Event → Trace)
    (hgen : Generic.TableGeneratorFaithful (addConfig isReal) enc generateTable)
    (events₁ events₂ : Generic.EventSet Event) :
    Generic.canonicalize (addConfig isReal) (generateTable events₁) =
      Generic.canonicalize (addConfig isReal) (generateTable events₂) ↔
    events₁ = events₂ :=
  one_to_one_for_config (addConfig isReal) enc generateTable hgen events₁ events₂

/-- Pico SUB canonicalizer one-to-one guarantee. -/
theorem sub_one_to_one {Event : Type} (isReal : Row → Bool)
    (enc : Generic.EventEncoding Event ALU.Tuple)
    (generateTable : Generic.EventSet Event → Trace)
    (hgen : Generic.TableGeneratorFaithful (subConfig isReal) enc generateTable)
    (events₁ events₂ : Generic.EventSet Event) :
    Generic.canonicalize (subConfig isReal) (generateTable events₁) =
      Generic.canonicalize (subConfig isReal) (generateTable events₂) ↔
    events₁ = events₂ :=
  one_to_one_for_config (subConfig isReal) enc generateTable hgen events₁ events₂

/-- Pico memory read/write canonicalizer one-to-one guarantee. -/
theorem memoryReadWrite_one_to_one {Event : Type} (isReal : Row → Bool)
    (enc : Generic.EventEncoding Event Memory.MemoryOpTuple)
    (generateTable : Generic.EventSet Event → Trace)
    (hgen : Generic.TableGeneratorFaithful (memoryReadWriteConfig isReal) enc generateTable)
    (events₁ events₂ : Generic.EventSet Event) :
    Generic.canonicalize (memoryReadWriteConfig isReal) (generateTable events₁) =
      Generic.canonicalize (memoryReadWriteConfig isReal) (generateTable events₂) ↔
    events₁ = events₂ :=
  one_to_one_for_config (memoryReadWriteConfig isReal) enc generateTable hgen events₁ events₂

end Pico

/-! ### Sphinx -/

namespace Sphinx

def addConfig (isReal : Row → Bool) : Generic.Config ALU.Tuple :=
  mkALUConfig [10, 11, 12, 13] [14, 15, 16, 17] [3, 4, 5, 6] isReal

def subConfig (isReal : Row → Bool) : Generic.Config ALU.Tuple :=
  mkALUConfig [3, 4, 5, 6] [14, 15, 16, 17] [10, 11, 12, 13] isReal

def lookupDrivenALUTables : List String :=
  ["sr", "shiftleft", "mul", "lt", "bitwise", "divrem"]

/-- Sphinx ADD canonicalizer one-to-one guarantee. -/
theorem add_one_to_one {Event : Type} (isReal : Row → Bool)
    (enc : Generic.EventEncoding Event ALU.Tuple)
    (generateTable : Generic.EventSet Event → Trace)
    (hgen : Generic.TableGeneratorFaithful (addConfig isReal) enc generateTable)
    (events₁ events₂ : Generic.EventSet Event) :
    Generic.canonicalize (addConfig isReal) (generateTable events₁) =
      Generic.canonicalize (addConfig isReal) (generateTable events₂) ↔
    events₁ = events₂ :=
  one_to_one_for_config (addConfig isReal) enc generateTable hgen events₁ events₂

/-- Sphinx SUB canonicalizer one-to-one guarantee. -/
theorem sub_one_to_one {Event : Type} (isReal : Row → Bool)
    (enc : Generic.EventEncoding Event ALU.Tuple)
    (generateTable : Generic.EventSet Event → Trace)
    (hgen : Generic.TableGeneratorFaithful (subConfig isReal) enc generateTable)
    (events₁ events₂ : Generic.EventSet Event) :
    Generic.canonicalize (subConfig isReal) (generateTable events₁) =
      Generic.canonicalize (subConfig isReal) (generateTable events₂) ↔
    events₁ = events₂ :=
  one_to_one_for_config (subConfig isReal) enc generateTable hgen events₁ events₂

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

/-- Ziren ADD canonicalizer one-to-one guarantee. -/
theorem add_one_to_one {Event : Type} (isReal : Row → Bool)
    (enc : Generic.EventEncoding Event ALU.Tuple)
    (generateTable : Generic.EventSet Event → Trace)
    (hgen : Generic.TableGeneratorFaithful (addConfig isReal) enc generateTable)
    (events₁ events₂ : Generic.EventSet Event) :
    Generic.canonicalize (addConfig isReal) (generateTable events₁) =
      Generic.canonicalize (addConfig isReal) (generateTable events₂) ↔
    events₁ = events₂ :=
  one_to_one_for_config (addConfig isReal) enc generateTable hgen events₁ events₂

/-- Ziren SUB canonicalizer one-to-one guarantee. -/
theorem sub_one_to_one {Event : Type} (isReal : Row → Bool)
    (enc : Generic.EventEncoding Event ALU.Tuple)
    (generateTable : Generic.EventSet Event → Trace)
    (hgen : Generic.TableGeneratorFaithful (subConfig isReal) enc generateTable)
    (events₁ events₂ : Generic.EventSet Event) :
    Generic.canonicalize (subConfig isReal) (generateTable events₁) =
      Generic.canonicalize (subConfig isReal) (generateTable events₂) ↔
    events₁ = events₂ :=
  one_to_one_for_config (subConfig isReal) enc generateTable hgen events₁ events₂

/-- Ziren DIV canonicalizer one-to-one guarantee. -/
theorem div_one_to_one {Event : Type} (isReal : Row → Bool)
    (enc : Generic.EventEncoding Event ALU.Tuple)
    (generateTable : Generic.EventSet Event → Trace)
    (hgen : Generic.TableGeneratorFaithful (divConfig isReal) enc generateTable)
    (events₁ events₂ : Generic.EventSet Event) :
    Generic.canonicalize (divConfig isReal) (generateTable events₁) =
      Generic.canonicalize (divConfig isReal) (generateTable events₂) ↔
    events₁ = events₂ :=
  one_to_one_for_config (divConfig isReal) enc generateTable hgen events₁ events₂

/-- Ziren REM canonicalizer one-to-one guarantee. -/
theorem rem_one_to_one {Event : Type} (isReal : Row → Bool)
    (enc : Generic.EventEncoding Event ALU.Tuple)
    (generateTable : Generic.EventSet Event → Trace)
    (hgen : Generic.TableGeneratorFaithful (remConfig isReal) enc generateTable)
    (events₁ events₂ : Generic.EventSet Event) :
    Generic.canonicalize (remConfig isReal) (generateTable events₁) =
      Generic.canonicalize (remConfig isReal) (generateTable events₂) ↔
    events₁ = events₂ :=
  one_to_one_for_config (remConfig isReal) enc generateTable hgen events₁ events₂

/-- Ziren CLO/CLZ canonicalizer one-to-one guarantee. -/
theorem cloClz_one_to_one {Event : Type} (isReal : Row → Bool)
    (enc : Generic.EventEncoding Event ControlFlow.UnaryTuple)
    (generateTable : Generic.EventSet Event → Trace)
    (hgen : Generic.TableGeneratorFaithful (cloClzConfig isReal) enc generateTable)
    (events₁ events₂ : Generic.EventSet Event) :
    Generic.canonicalize (cloClzConfig isReal) (generateTable events₁) =
      Generic.canonicalize (cloClzConfig isReal) (generateTable events₂) ↔
    events₁ = events₂ :=
  one_to_one_for_config (cloClzConfig isReal) enc generateTable hgen events₁ events₂

/-- Ziren MOVCOND canonicalizer one-to-one guarantee. -/
theorem movCond_one_to_one {Event : Type} (isReal : Row → Bool)
    (enc : Generic.EventEncoding Event ControlFlow.MovCondTuple)
    (generateTable : Generic.EventSet Event → Trace)
    (hgen : Generic.TableGeneratorFaithful (movCondConfig isReal) enc generateTable)
    (events₁ events₂ : Generic.EventSet Event) :
    Generic.canonicalize (movCondConfig isReal) (generateTable events₁) =
      Generic.canonicalize (movCondConfig isReal) (generateTable events₂) ↔
    events₁ = events₂ :=
  one_to_one_for_config (movCondConfig isReal) enc generateTable hgen events₁ events₂

/-- Ziren branch canonicalizer one-to-one guarantee. -/
theorem branch_one_to_one {Event : Type} (isReal : Row → Bool)
    (enc : Generic.EventEncoding Event ControlFlow.ControlFlowTuple)
    (generateTable : Generic.EventSet Event → Trace)
    (hgen : Generic.TableGeneratorFaithful (branchConfig isReal) enc generateTable)
    (events₁ events₂ : Generic.EventSet Event) :
    Generic.canonicalize (branchConfig isReal) (generateTable events₁) =
      Generic.canonicalize (branchConfig isReal) (generateTable events₂) ↔
    events₁ = events₂ :=
  one_to_one_for_config (branchConfig isReal) enc generateTable hgen events₁ events₂

/-- Ziren jump canonicalizer one-to-one guarantee. -/
theorem jump_one_to_one {Event : Type} (isReal : Row → Bool)
    (enc : Generic.EventEncoding Event ControlFlow.ControlFlowTuple)
    (generateTable : Generic.EventSet Event → Trace)
    (hgen : Generic.TableGeneratorFaithful (jumpConfig isReal) enc generateTable)
    (events₁ events₂ : Generic.EventSet Event) :
    Generic.canonicalize (jumpConfig isReal) (generateTable events₁) =
      Generic.canonicalize (jumpConfig isReal) (generateTable events₂) ↔
    events₁ = events₂ :=
  one_to_one_for_config (jumpConfig isReal) enc generateTable hgen events₁ events₂

/-- Ziren memory-instructions canonicalizer one-to-one guarantee. -/
theorem memoryInstrs_one_to_one {Event : Type} (isReal : Row → Bool)
    (enc : Generic.EventEncoding Event Memory.MemoryOpTuple)
    (generateTable : Generic.EventSet Event → Trace)
    (hgen : Generic.TableGeneratorFaithful (memoryInstrsConfig isReal) enc generateTable)
    (events₁ events₂ : Generic.EventSet Event) :
    Generic.canonicalize (memoryInstrsConfig isReal) (generateTable events₁) =
      Generic.canonicalize (memoryInstrsConfig isReal) (generateTable events₂) ↔
    events₁ = events₂ :=
  one_to_one_for_config (memoryInstrsConfig isReal) enc generateTable hgen events₁ events₂

end Ziren

/-! ### Valida -/

namespace Valida

def lt32Config (isReal : Row → Bool) : Generic.Config ControlFlow.ValidaLtTuple :=
  ControlFlow.mkValidaLtConfig [0, 1, 2, 3] [4, 5, 6, 7] 21 isReal

/-- Valida memory is intentionally skipped: its canonicalizer is not yet
    supported in Zebra. -/
def lookupDrivenALUTables : List String :=
  ["add32", "sub32", "mul32", "div32", "bitwise32", "com32"]

/-- Valida LT32 canonicalizer one-to-one guarantee. -/
theorem lt32_one_to_one {Event : Type} (isReal : Row → Bool)
    (enc : Generic.EventEncoding Event ControlFlow.ValidaLtTuple)
    (generateTable : Generic.EventSet Event → Trace)
    (hgen : Generic.TableGeneratorFaithful (lt32Config isReal) enc generateTable)
    (events₁ events₂ : Generic.EventSet Event) :
    Generic.canonicalize (lt32Config isReal) (generateTable events₁) =
      Generic.canonicalize (lt32Config isReal) (generateTable events₂) ↔
    events₁ = events₂ :=
  one_to_one_for_config (lt32Config isReal) enc generateTable hgen events₁ events₂

end Valida

/-! ### OpenVM -/

namespace OpenVM

/-- OpenVM examples currently use the lookup-driven ALU final checker. CPU
    tables are intentionally skipped. -/
def lookupDrivenALUTables : List String :=
  ["alu", "bitwise", "branch", "jump", "lt", "mul", "shift"]

end OpenVM

end Zebra.Examples
