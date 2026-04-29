/-
Zebra — concrete example table layouts from the Rust experiments.

This file includes only the canonical representation shape and column layout;
faithfulness of the corresponding Rust table generators remains an assumption.
-/
import Zebra.Canonicalizer.ALU
import Zebra.Canonicalizer.ControlFlow
import Zebra.Canonicalizer.CPU
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
    faithful to the stated config and an injective record encoding, then the
    canonical representation is one-to-one with the original record set. -/
theorem one_to_one_for_config {Record Repr : Type} [DecidableEq Repr]
    (cfg : Generic.Config Repr)
    (enc : Generic.RecordIdentity Record Repr)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : Generic.TableGeneratorFaithful cfg enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    Generic.canonicalize cfg (generateTable records₁) =
      Generic.canonicalize cfg (generateTable records₂) ↔
    records₁ = records₂ :=
  Generic.canonicalize_generated_eq_iff_records_eq cfg enc generateTable hgen records₁ records₂

/-- Guarantee for lookup-driven ALU tables whose concrete operand columns are
    provided by `GeneralLookupInfo` at extraction time rather than hard-coded in
    the example file. -/
theorem lookup_driven_alu_one_to_one {Record : Type}
    (cfg : Generic.Config ALU.Tuple)
    (enc : Generic.RecordIdentity Record ALU.Tuple)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : Generic.TableGeneratorFaithful cfg enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    Generic.canonicalize cfg (generateTable records₁) =
      Generic.canonicalize cfg (generateTable records₂) ↔
    records₁ = records₂ :=
  one_to_one_for_config cfg enc generateTable hgen records₁ records₂

/-- Shared guarantee for CPU configs. A CPU row can emit multiple canonical
    records, so it uses the CPU-specific canonicalizer rather than
    `Generic.canonicalize`. -/
theorem cpu_one_to_one_for_config {Record : Type} [DecidableEq Record]
    (cfg : CPU.Config)
    (enc : Generic.RecordIdentity Record CPU.RecordRepr)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : CPU.TableGeneratorFaithful cfg enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    CPU.canonicalize cfg (generateTable records₁) =
      CPU.canonicalize cfg (generateTable records₂) ↔
    records₁ = records₂ :=
  CPU.canonicalize_generated_eq_iff_records_eq cfg enc generateTable hgen records₁ records₂

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

def cpuConfig (isReal : Row → Bool) (isOpAWrite : Row → Bool) : CPU.Config where
  isReal := isReal
  clk := [1, 2]
  pc := [5]
  writes := [{ isWrite := isOpAWrite, addr := [8], value := [29, 30, 31, 32] }]

/-- Tables using `generate_alu_final_checker`, whose columns are supplied by
    `GeneralLookupInfo` at extraction time. -/
def lookupDrivenALUTables : List String :=
  ["lt", "bitwise", "divrem", "shiftleft", "mul", "sr"]

/-- SP1 ADD canonicalizer one-to-one guarantee. -/
theorem add_one_to_one {Record : Type} (isReal : Row → Bool)
    (enc : Generic.RecordIdentity Record ALU.Tuple)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : Generic.TableGeneratorFaithful (addConfig isReal) enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    Generic.canonicalize (addConfig isReal) (generateTable records₁) =
      Generic.canonicalize (addConfig isReal) (generateTable records₂) ↔
    records₁ = records₂ :=
  one_to_one_for_config (addConfig isReal) enc generateTable hgen records₁ records₂

/-- SP1 SUB canonicalizer one-to-one guarantee. -/
theorem sub_one_to_one {Record : Type} (isReal : Row → Bool)
    (enc : Generic.RecordIdentity Record ALU.Tuple)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : Generic.TableGeneratorFaithful (subConfig isReal) enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    Generic.canonicalize (subConfig isReal) (generateTable records₁) =
      Generic.canonicalize (subConfig isReal) (generateTable records₂) ↔
    records₁ = records₂ :=
  one_to_one_for_config (subConfig isReal) enc generateTable hgen records₁ records₂

/-- SP1 branch/jump control-flow canonicalizer one-to-one guarantee. -/
theorem controlFlow_one_to_one {Record : Type} (isReal : Row → Bool)
    (enc : Generic.RecordIdentity Record ControlFlow.ControlFlowTuple)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : Generic.TableGeneratorFaithful (controlFlowConfig isReal) enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    Generic.canonicalize (controlFlowConfig isReal) (generateTable records₁) =
      Generic.canonicalize (controlFlowConfig isReal) (generateTable records₂) ↔
    records₁ = records₂ :=
  one_to_one_for_config (controlFlowConfig isReal) enc generateTable hgen records₁ records₂

/-- SP1 memory-instructions canonicalizer one-to-one guarantee. -/
theorem memoryInstrs_one_to_one {Record : Type} (isReal : Row → Bool)
    (enc : Generic.RecordIdentity Record Memory.MemoryOpTuple)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : Generic.TableGeneratorFaithful (memoryInstrsConfig isReal) enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    Generic.canonicalize (memoryInstrsConfig isReal) (generateTable records₁) =
      Generic.canonicalize (memoryInstrsConfig isReal) (generateTable records₂) ↔
    records₁ = records₂ :=
  one_to_one_for_config (memoryInstrsConfig isReal) enc generateTable hgen records₁ records₂

/-- SP1 CPU canonicalizer one-to-one guarantee. -/
theorem cpu_one_to_one {Record : Type} [DecidableEq Record]
    (isReal isOpAWrite : Row → Bool)
    (enc : Generic.RecordIdentity Record CPU.RecordRepr)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : CPU.TableGeneratorFaithful (cpuConfig isReal isOpAWrite) enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    CPU.canonicalize (cpuConfig isReal isOpAWrite) (generateTable records₁) =
      CPU.canonicalize (cpuConfig isReal isOpAWrite) (generateTable records₂) ↔
    records₁ = records₂ :=
  cpu_one_to_one_for_config (cpuConfig isReal isOpAWrite) enc generateTable hgen records₁ records₂

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

def cpuConfig (isReal : Row → Bool) (isOpAWrite : Row → Bool) : CPU.Config where
  isReal := isReal
  clk := [2, 3]
  pc := [4]
  writes := [{ isWrite := isOpAWrite, addr := [7], value := [46, 47, 48, 49] }]

def lookupDrivenALUTables : List String :=
  ["sr", "sll", "lessthan", "mul", "bitwise", "divrem"]

/-- Pico ADD canonicalizer one-to-one guarantee. -/
theorem add_one_to_one {Record : Type} (isReal : Row → Bool)
    (enc : Generic.RecordIdentity Record ALU.Tuple)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : Generic.TableGeneratorFaithful (addConfig isReal) enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    Generic.canonicalize (addConfig isReal) (generateTable records₁) =
      Generic.canonicalize (addConfig isReal) (generateTable records₂) ↔
    records₁ = records₂ :=
  one_to_one_for_config (addConfig isReal) enc generateTable hgen records₁ records₂

/-- Pico SUB canonicalizer one-to-one guarantee. -/
theorem sub_one_to_one {Record : Type} (isReal : Row → Bool)
    (enc : Generic.RecordIdentity Record ALU.Tuple)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : Generic.TableGeneratorFaithful (subConfig isReal) enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    Generic.canonicalize (subConfig isReal) (generateTable records₁) =
      Generic.canonicalize (subConfig isReal) (generateTable records₂) ↔
    records₁ = records₂ :=
  one_to_one_for_config (subConfig isReal) enc generateTable hgen records₁ records₂

/-- Pico memory read/write canonicalizer one-to-one guarantee. -/
theorem memoryReadWrite_one_to_one {Record : Type} (isReal : Row → Bool)
    (enc : Generic.RecordIdentity Record Memory.MemoryOpTuple)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : Generic.TableGeneratorFaithful (memoryReadWriteConfig isReal) enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    Generic.canonicalize (memoryReadWriteConfig isReal) (generateTable records₁) =
      Generic.canonicalize (memoryReadWriteConfig isReal) (generateTable records₂) ↔
    records₁ = records₂ :=
  one_to_one_for_config (memoryReadWriteConfig isReal) enc generateTable hgen records₁ records₂

/-- Pico CPU canonicalizer one-to-one guarantee. -/
theorem cpu_one_to_one {Record : Type} [DecidableEq Record]
    (isReal isOpAWrite : Row → Bool)
    (enc : Generic.RecordIdentity Record CPU.RecordRepr)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : CPU.TableGeneratorFaithful (cpuConfig isReal isOpAWrite) enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    CPU.canonicalize (cpuConfig isReal isOpAWrite) (generateTable records₁) =
      CPU.canonicalize (cpuConfig isReal isOpAWrite) (generateTable records₂) ↔
    records₁ = records₂ :=
  cpu_one_to_one_for_config (cpuConfig isReal isOpAWrite) enc generateTable hgen records₁ records₂

end Pico

/-! ### Sphinx -/

namespace Sphinx

def addConfig (isReal : Row → Bool) : Generic.Config ALU.Tuple :=
  mkALUConfig [10, 11, 12, 13] [14, 15, 16, 17] [3, 4, 5, 6] isReal

def subConfig (isReal : Row → Bool) : Generic.Config ALU.Tuple :=
  mkALUConfig [3, 4, 5, 6] [14, 15, 16, 17] [10, 11, 12, 13] isReal

def cpuConfig (isReal : Row → Bool) : CPU.Config where
  isReal := isReal
  clk := [4, 5]
  pc := [6]
  writes := [{ isWrite := fun _ => true, addr := [9, 10, 11, 12], value := [64, 65, 66, 67] }]

def lookupDrivenALUTables : List String :=
  ["sr", "shiftleft", "mul", "lt", "bitwise", "divrem"]

/-- Sphinx ADD canonicalizer one-to-one guarantee. -/
theorem add_one_to_one {Record : Type} (isReal : Row → Bool)
    (enc : Generic.RecordIdentity Record ALU.Tuple)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : Generic.TableGeneratorFaithful (addConfig isReal) enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    Generic.canonicalize (addConfig isReal) (generateTable records₁) =
      Generic.canonicalize (addConfig isReal) (generateTable records₂) ↔
    records₁ = records₂ :=
  one_to_one_for_config (addConfig isReal) enc generateTable hgen records₁ records₂

/-- Sphinx SUB canonicalizer one-to-one guarantee. -/
theorem sub_one_to_one {Record : Type} (isReal : Row → Bool)
    (enc : Generic.RecordIdentity Record ALU.Tuple)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : Generic.TableGeneratorFaithful (subConfig isReal) enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    Generic.canonicalize (subConfig isReal) (generateTable records₁) =
      Generic.canonicalize (subConfig isReal) (generateTable records₂) ↔
    records₁ = records₂ :=
  one_to_one_for_config (subConfig isReal) enc generateTable hgen records₁ records₂

/-- Sphinx CPU canonicalizer one-to-one guarantee. -/
theorem cpu_one_to_one {Record : Type} [DecidableEq Record]
    (isReal : Row → Bool)
    (enc : Generic.RecordIdentity Record CPU.RecordRepr)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : CPU.TableGeneratorFaithful (cpuConfig isReal) enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    CPU.canonicalize (cpuConfig isReal) (generateTable records₁) =
      CPU.canonicalize (cpuConfig isReal) (generateTable records₂) ↔
    records₁ = records₂ :=
  cpu_one_to_one_for_config (cpuConfig isReal) enc generateTable hgen records₁ records₂

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

def cpuConfig (isReal : Row → Bool) (isOpAWrite : Row → Bool) : CPU.Config where
  isReal := isReal
  clk := [1, 2]
  pc := [5]
  writes := [{ isWrite := isOpAWrite, addr := [9], value := [26, 27, 28, 29] }]

def lookupDrivenALUTables : List String :=
  ["mul", "shiftleft", "shiftright", "lt", "bitwise"]

/-- Ziren ADD canonicalizer one-to-one guarantee. -/
theorem add_one_to_one {Record : Type} (isReal : Row → Bool)
    (enc : Generic.RecordIdentity Record ALU.Tuple)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : Generic.TableGeneratorFaithful (addConfig isReal) enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    Generic.canonicalize (addConfig isReal) (generateTable records₁) =
      Generic.canonicalize (addConfig isReal) (generateTable records₂) ↔
    records₁ = records₂ :=
  one_to_one_for_config (addConfig isReal) enc generateTable hgen records₁ records₂

/-- Ziren SUB canonicalizer one-to-one guarantee. -/
theorem sub_one_to_one {Record : Type} (isReal : Row → Bool)
    (enc : Generic.RecordIdentity Record ALU.Tuple)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : Generic.TableGeneratorFaithful (subConfig isReal) enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    Generic.canonicalize (subConfig isReal) (generateTable records₁) =
      Generic.canonicalize (subConfig isReal) (generateTable records₂) ↔
    records₁ = records₂ :=
  one_to_one_for_config (subConfig isReal) enc generateTable hgen records₁ records₂

/-- Ziren DIV canonicalizer one-to-one guarantee. -/
theorem div_one_to_one {Record : Type} (isReal : Row → Bool)
    (enc : Generic.RecordIdentity Record ALU.Tuple)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : Generic.TableGeneratorFaithful (divConfig isReal) enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    Generic.canonicalize (divConfig isReal) (generateTable records₁) =
      Generic.canonicalize (divConfig isReal) (generateTable records₂) ↔
    records₁ = records₂ :=
  one_to_one_for_config (divConfig isReal) enc generateTable hgen records₁ records₂

/-- Ziren REM canonicalizer one-to-one guarantee. -/
theorem rem_one_to_one {Record : Type} (isReal : Row → Bool)
    (enc : Generic.RecordIdentity Record ALU.Tuple)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : Generic.TableGeneratorFaithful (remConfig isReal) enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    Generic.canonicalize (remConfig isReal) (generateTable records₁) =
      Generic.canonicalize (remConfig isReal) (generateTable records₂) ↔
    records₁ = records₂ :=
  one_to_one_for_config (remConfig isReal) enc generateTable hgen records₁ records₂

/-- Ziren CLO/CLZ canonicalizer one-to-one guarantee. -/
theorem cloClz_one_to_one {Record : Type} (isReal : Row → Bool)
    (enc : Generic.RecordIdentity Record ControlFlow.UnaryTuple)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : Generic.TableGeneratorFaithful (cloClzConfig isReal) enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    Generic.canonicalize (cloClzConfig isReal) (generateTable records₁) =
      Generic.canonicalize (cloClzConfig isReal) (generateTable records₂) ↔
    records₁ = records₂ :=
  one_to_one_for_config (cloClzConfig isReal) enc generateTable hgen records₁ records₂

/-- Ziren MOVCOND canonicalizer one-to-one guarantee. -/
theorem movCond_one_to_one {Record : Type} (isReal : Row → Bool)
    (enc : Generic.RecordIdentity Record ControlFlow.MovCondTuple)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : Generic.TableGeneratorFaithful (movCondConfig isReal) enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    Generic.canonicalize (movCondConfig isReal) (generateTable records₁) =
      Generic.canonicalize (movCondConfig isReal) (generateTable records₂) ↔
    records₁ = records₂ :=
  one_to_one_for_config (movCondConfig isReal) enc generateTable hgen records₁ records₂

/-- Ziren branch canonicalizer one-to-one guarantee. -/
theorem branch_one_to_one {Record : Type} (isReal : Row → Bool)
    (enc : Generic.RecordIdentity Record ControlFlow.ControlFlowTuple)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : Generic.TableGeneratorFaithful (branchConfig isReal) enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    Generic.canonicalize (branchConfig isReal) (generateTable records₁) =
      Generic.canonicalize (branchConfig isReal) (generateTable records₂) ↔
    records₁ = records₂ :=
  one_to_one_for_config (branchConfig isReal) enc generateTable hgen records₁ records₂

/-- Ziren jump canonicalizer one-to-one guarantee. -/
theorem jump_one_to_one {Record : Type} (isReal : Row → Bool)
    (enc : Generic.RecordIdentity Record ControlFlow.ControlFlowTuple)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : Generic.TableGeneratorFaithful (jumpConfig isReal) enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    Generic.canonicalize (jumpConfig isReal) (generateTable records₁) =
      Generic.canonicalize (jumpConfig isReal) (generateTable records₂) ↔
    records₁ = records₂ :=
  one_to_one_for_config (jumpConfig isReal) enc generateTable hgen records₁ records₂

/-- Ziren memory-instructions canonicalizer one-to-one guarantee. -/
theorem memoryInstrs_one_to_one {Record : Type} (isReal : Row → Bool)
    (enc : Generic.RecordIdentity Record Memory.MemoryOpTuple)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : Generic.TableGeneratorFaithful (memoryInstrsConfig isReal) enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    Generic.canonicalize (memoryInstrsConfig isReal) (generateTable records₁) =
      Generic.canonicalize (memoryInstrsConfig isReal) (generateTable records₂) ↔
    records₁ = records₂ :=
  one_to_one_for_config (memoryInstrsConfig isReal) enc generateTable hgen records₁ records₂

/-- Ziren CPU canonicalizer one-to-one guarantee. -/
theorem cpu_one_to_one {Record : Type} [DecidableEq Record]
    (isReal isOpAWrite : Row → Bool)
    (enc : Generic.RecordIdentity Record CPU.RecordRepr)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : CPU.TableGeneratorFaithful (cpuConfig isReal isOpAWrite) enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    CPU.canonicalize (cpuConfig isReal isOpAWrite) (generateTable records₁) =
      CPU.canonicalize (cpuConfig isReal isOpAWrite) (generateTable records₂) ↔
    records₁ = records₂ :=
  cpu_one_to_one_for_config (cpuConfig isReal isOpAWrite) enc generateTable hgen records₁ records₂

end Ziren

/-! ### Valida -/

namespace Valida

def lt32Config (isReal : Row → Bool) : Generic.Config ControlFlow.ValidaLtTuple :=
  ControlFlow.mkValidaLtConfig [0, 1, 2, 3] [4, 5, 6, 7] 21 isReal

def memoryConfig (isRead isWrite : Row → Bool) : Memory.AccessConfig where
  isRead := isRead
  isWrite := isWrite
  clk := 13
  addr := 12
  value := [4, 5, 6, 7]

def lookupDrivenALUTables : List String :=
  ["add32", "sub32", "mul32", "div32", "bitwise32", "com32"]

/-- Valida LT32 canonicalizer one-to-one guarantee. -/
theorem lt32_one_to_one {Record : Type} (isReal : Row → Bool)
    (enc : Generic.RecordIdentity Record ControlFlow.ValidaLtTuple)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : Generic.TableGeneratorFaithful (lt32Config isReal) enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    Generic.canonicalize (lt32Config isReal) (generateTable records₁) =
      Generic.canonicalize (lt32Config isReal) (generateTable records₂) ↔
    records₁ = records₂ :=
  one_to_one_for_config (lt32Config isReal) enc generateTable hgen records₁ records₂

/-- Valida memory-table canonicalizer one-to-one guarantee. The canonical record
    is `(clk, addr, value, read_or_write)`, with read/write selection supplied by
    the extracted row predicates. -/
theorem memory_one_to_one {Record : Type} [DecidableEq Record]
    (isRead isWrite : Row → Bool)
    (enc : Generic.RecordIdentity Record Memory.AccessTuple)
    (generateTable : Generic.RecordSet Record → Trace)
    (hgen : Memory.AccessTableGeneratorFaithful (memoryConfig isRead isWrite) enc generateTable)
    (records₁ records₂ : Generic.RecordSet Record) :
    Memory.canonicalizeAccesses (memoryConfig isRead isWrite) (generateTable records₁) =
      Memory.canonicalizeAccesses (memoryConfig isRead isWrite) (generateTable records₂) ↔
    records₁ = records₂ :=
  Memory.canonicalizeAccesses_generated_eq_iff_records_eq
    (memoryConfig isRead isWrite) enc generateTable hgen records₁ records₂

end Valida

/-! ### OpenVM -/

namespace OpenVM

/-- OpenVM examples currently use the lookup-driven ALU final checker. -/
def lookupDrivenALUTables : List String :=
  ["alu", "bitwise", "branch", "jump", "lt", "mul", "shift"]

end OpenVM

end Zebra.Examples
