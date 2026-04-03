use std::collections::HashSet;

use openvm_circuit::arch::{
    testing::{test_adapter::TestAdapterAir, BITWISE_OP_LOOKUP_BUS, RANGE_TUPLE_CHECKER_BUS},
    ExecutionBus, ExecutionBridge, VmAirWrapper,
};
use openvm_circuit::system::program::ProgramBus;
use openvm_circuit_primitives::{
    bitwise_op_lookup::{BitwiseOperationLookupBus, SharedBitwiseOperationLookupChip},
    range_tuple::{RangeTupleCheckerBus, SharedRangeTupleCheckerChip},
    var_range::{SharedVariableRangeCheckerChip, VariableRangeCheckerBus},
};
use openvm_instructions::LocalOpcode;
use openvm_rv32im_circuit::{
    adapters::{RV32_CELL_BITS, RV32_REGISTER_NUM_LIMBS},
    BaseAluCoreChip, BranchEqualCoreChip, BranchLessThanCoreChip, LessThanCoreChip,
    MultiplicationCoreChip, Rv32JalLuiCoreChip, ShiftCoreChip,
};
use openvm_rv32im_transpiler::{
    BaseAluOpcode, BranchEqualOpcode, BranchLessThanOpcode, LessThanOpcode, MulOpcode, ShiftOpcode,
};
use openvm_stark_backend::p3_field::PrimeField32;

use crate::p3_to_tv::{contains_permutation, convert_openvm_expr, get_symbolic_constraints_openvm};

use zebra::constraint::ZEBRAConstraints;
use zebra::interval::AbstractInterval;
use zebra::solver::{prepare_constraints_and_range_type, ConstraintInfo};
use zebra::symbolic::{
    make_impl_constraint, GeneralLookupInfo, ZEBRASymbolicEntry, ZEBRASymbolicExpr as LVSExpr,
    ZEBRASymbolicVal,
};
use zebra::wordop::{get_alu_constraint, WordOp};

// ── Column layout: BaseAlu (ADD / SUB / XOR / OR / AND) ───────────────────────
//
//  [0]        from_pc           ← TestAdapterCols
//  [1..=7]    operands[7]       ← TestAdapterCols
//  [8..=11]   a[4]              ← BaseAluCoreCols  (result, LE 8-bit limbs)
//  [12..=15]  b[4]              ← BaseAluCoreCols  (first operand)
//  [16..=19]  c[4]              ← BaseAluCoreCols  (second operand)
//  [20]       opcode_add_flag   ← BaseAluCoreCols
//  [21]       opcode_sub_flag   ← BaseAluCoreCols
//  [22]       opcode_xor_flag   ← BaseAluCoreCols
//  [23]       opcode_or_flag    ← BaseAluCoreCols
//  [24]       opcode_and_flag   ← BaseAluCoreCols

pub const ADAPTER_WIDTH: usize = 8;

pub const NUM_BASE_ALU_COLS: usize = 25;
pub const COL_A_START: usize = ADAPTER_WIDTH;       // 8
pub const COL_B_START: usize = ADAPTER_WIDTH + 4;   // 12
pub const COL_C_START: usize = ADAPTER_WIDTH + 8;   // 16
pub const COL_ADD_FLAG: usize = ADAPTER_WIDTH + 12; // 20
pub const COL_SUB_FLAG: usize = ADAPTER_WIDTH + 13; // 21
pub const COL_XOR_FLAG: usize = ADAPTER_WIDTH + 14; // 22
pub const COL_OR_FLAG: usize  = ADAPTER_WIDTH + 15; // 23
pub const COL_AND_FLAG: usize = ADAPTER_WIDTH + 16; // 24

// ── Column layout: Shift (SRL only) ───────────────────────────────────────────
//
// ShiftCoreCols<4,8> layout (core columns, offset by ADAPTER_WIDTH=8):
//  [8..=11]   a[4]              (result)
//  [12..=15]  b[4]              (operand to shift)
//  [16..=19]  c[4]              (shift amount)
//  [20]       opcode_sll_flag
//  [21]       opcode_srl_flag
//  [22]       opcode_sra_flag
//  [23]       bit_multiplier_left
//  [24]       bit_multiplier_right
//  [25]       b_sign
//  [26..=33]  bit_shift_marker[8]
//  [34..=37]  limb_shift_marker[4]
//  [38..=41]  bit_shift_carry[4]

pub const NUM_SHIFT_COLS: usize = 42;
pub const COL_SHIFT_A_START: usize = ADAPTER_WIDTH;       // 8
pub const COL_SHIFT_B_START: usize = ADAPTER_WIDTH + 4;   // 12
pub const COL_SHIFT_C_START: usize = ADAPTER_WIDTH + 8;   // 16
pub const COL_SHIFT_SRL_FLAG: usize = ADAPTER_WIDTH + 13; // 21

// ── Column layout: Mul (MUL only) ─────────────────────────────────────────────
//
// MultiplicationCoreCols<4,8>:
//  [8..=11]   a[4]     (result = low 32 bits of b*c)
//  [12..=15]  b[4]
//  [16..=19]  c[4]
//  [20]       is_valid

pub const NUM_MUL_COLS: usize = 21;
pub const COL_MUL_A_START: usize = ADAPTER_WIDTH;       // 8
pub const COL_MUL_B_START: usize = ADAPTER_WIDTH + 4;   // 12
pub const COL_MUL_C_START: usize = ADAPTER_WIDTH + 8;   // 16
pub const COL_MUL_IS_VALID: usize = ADAPTER_WIDTH + 12; // 20

// ── Column layout: LessThan (SLT / SLTU) ──────────────────────────────────────
//
// LessThanCoreCols<4,8>:
//  [8..=11]   b[4]              (first operand)
//  [12..=15]  c[4]              (second operand)
//  [16]       cmp_result        (0 or 1)
//  [17]       opcode_slt_flag
//  [18]       opcode_sltu_flag
//  [19]       b_msb_f
//  [20]       c_msb_f
//  [21..=24]  diff_marker[4]
//  [25]       diff_val

pub const NUM_LT_COLS: usize = 26;
pub const COL_LT_B_START: usize = ADAPTER_WIDTH;        // 8
pub const COL_LT_C_START: usize = ADAPTER_WIDTH + 4;    // 12
pub const COL_LT_CMP_RESULT: usize = ADAPTER_WIDTH + 8; // 16
pub const COL_LT_SLT_FLAG: usize = ADAPTER_WIDTH + 9;   // 17
pub const COL_LT_SLTU_FLAG: usize = ADAPTER_WIDTH + 10; // 18
pub const COL_LT_B_MSB_F: usize = ADAPTER_WIDTH + 11;   // 19
pub const COL_LT_C_MSB_F: usize = ADAPTER_WIDTH + 12;   // 20
pub const COL_LT_DIFF_MARKER_START: usize = ADAPTER_WIDTH + 13; // 21
pub const COL_LT_DIFF_VAL: usize = ADAPTER_WIDTH + 17;  // 25

// ── Column layout: BranchEqual (BEQ / BNE) ────────────────────────────────────
//
// BranchEqualCoreCols<4>:
//  [8..=11]   a[4]                 (first operand)
//  [12..=15]  b[4]                 (second operand)
//  [16]       cmp_result           (branch taken iff 1)
//  [17]       imm                  (branch offset)
//  [18]       opcode_beq_flag
//  [19]       opcode_bne_flag
//  [20..=23]  diff_inv_marker[4]

pub const NUM_BREQ_COLS: usize = 24;
pub const COL_BREQ_A_START: usize = ADAPTER_WIDTH;           // 8
pub const COL_BREQ_B_START: usize = ADAPTER_WIDTH + 4;       // 12
pub const COL_BREQ_CMP_RESULT: usize = ADAPTER_WIDTH + 8;    // 16
pub const COL_BREQ_IMM: usize = ADAPTER_WIDTH + 9;           // 17
pub const COL_BREQ_BEQ_FLAG: usize = ADAPTER_WIDTH + 10;     // 18
pub const COL_BREQ_BNE_FLAG: usize = ADAPTER_WIDTH + 11;     // 19
pub const COL_BREQ_DIFF_INV_START: usize = ADAPTER_WIDTH + 12; // 20

// ── Column layout: BranchLessThan (BLT / BLTU / BGE / BGEU) ──────────────────
//
// BranchLessThanCoreCols<4,8>:
//  [8..=11]   a[4]
//  [12..=15]  b[4]
//  [16]       cmp_result
//  [17]       imm
//  [18]       opcode_blt_flag
//  [19]       opcode_bltu_flag
//  [20]       opcode_bge_flag
//  [21]       opcode_bgeu_flag
//  [22]       a_msb_f
//  [23]       b_msb_f
//  [24]       cmp_lt
//  [25..=28]  diff_marker[4]
//  [29]       diff_val

pub const NUM_BLT_COLS: usize = 30;
pub const COL_BLT_A_START: usize = ADAPTER_WIDTH;           // 8
pub const COL_BLT_B_START: usize = ADAPTER_WIDTH + 4;       // 12
pub const COL_BLT_CMP_RESULT: usize = ADAPTER_WIDTH + 8;    // 16
pub const COL_BLT_IMM: usize = ADAPTER_WIDTH + 9;           // 17
pub const COL_BLT_BLT_FLAG: usize = ADAPTER_WIDTH + 10;     // 18
pub const COL_BLT_BLTU_FLAG: usize = ADAPTER_WIDTH + 11;    // 19
pub const COL_BLT_BGE_FLAG: usize = ADAPTER_WIDTH + 12;     // 20
pub const COL_BLT_BGEU_FLAG: usize = ADAPTER_WIDTH + 13;    // 21
pub const COL_BLT_A_MSB_F: usize = ADAPTER_WIDTH + 14;      // 22
pub const COL_BLT_B_MSB_F: usize = ADAPTER_WIDTH + 15;      // 23
pub const COL_BLT_CMP_LT: usize = ADAPTER_WIDTH + 16;       // 24
pub const COL_BLT_DIFF_MARKER_START: usize = ADAPTER_WIDTH + 17; // 25
pub const COL_BLT_DIFF_VAL: usize = ADAPTER_WIDTH + 21;     // 29

// ── Column layout: JalLui (JAL / LUI) ────────────────────────────────────────
//
// Rv32JalLuiCoreCols:
//  [8]        imm
//  [9..=12]   rd_data[4]
//  [13]       is_jal
//  [14]       is_lui

pub const NUM_JAL_COLS: usize = 15;
pub const COL_JAL_IMM: usize = ADAPTER_WIDTH;             // 8
pub const COL_JAL_RD_START: usize = ADAPTER_WIDTH + 1;   // 9
pub const COL_JAL_IS_JAL: usize = ADAPTER_WIDTH + 5;     // 13
pub const COL_JAL_IS_LUI: usize = ADAPTER_WIDTH + 6;     // 14

// ── Column layout: Jalr (JALR) ────────────────────────────────────────────────
//
// Rv32JalrCoreCols:
//  [8]        imm
//  [9..=12]   rs1_data[4]
//  [13..=15]  rd_data[3]  (3 MSB limbs of from_pc+4)
//  [16]       is_valid
//  [17]       to_pc_least_sig_bit
//  [18..=19]  to_pc_limbs[2]
//  [20]       imm_sign

pub const NUM_JALR_COLS: usize = 21;
pub const COL_JALR_IMM: usize = ADAPTER_WIDTH;                   // 8
pub const COL_JALR_RS1_START: usize = ADAPTER_WIDTH + 1;         // 9
pub const COL_JALR_RD_START: usize = ADAPTER_WIDTH + 5;          // 13
pub const COL_JALR_IS_VALID: usize = ADAPTER_WIDTH + 8;          // 16
pub const COL_JALR_TO_PC_LSB: usize = ADAPTER_WIDTH + 9;         // 17
pub const COL_JALR_TO_PC_LIMBS_START: usize = ADAPTER_WIDTH + 10; // 18
pub const COL_JALR_IMM_SIGN: usize = ADAPTER_WIDTH + 12;         // 20

/// BabyBear prime: 2^31 - 2^27 + 1.
pub const BABY_BEAR_PRIME: u32 = (1 << 31) - (1 << 27) + 1;

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Construct a `TestAdapterAir` with dummy bus indices (fine for symbolic extraction
/// since bus interactions are filtered out by `contains_permutation`).
fn make_test_adapter_air() -> TestAdapterAir {
    TestAdapterAir {
        execution_bridge: ExecutionBridge::new(ExecutionBus::new(0), ProgramBus::new(1)),
    }
}

fn col(idx: usize) -> LVSExpr {
    LVSExpr::Variable(ZEBRASymbolicVal {
        entry: ZEBRASymbolicEntry::Main { is_curr: true },
        index: idx,
    })
}

fn cols4(start: usize) -> [LVSExpr; 4] {
    [col(start), col(start + 1), col(start + 2), col(start + 3)]
}

// ── BaseAlu constraints ───────────────────────────────────────────────────────

/// Build ZEBRA constraints for the OpenVM `BaseAluCoreAir` chip
/// (ADD / SUB / XOR / OR / AND over 32-bit RISC-V words).
pub fn extract_base_alu_constraints<F: PrimeField32>(
    prime: u32,
) -> (ConstraintInfo, GeneralLookupInfo) {
    let a = cols4(COL_A_START);
    let b = cols4(COL_B_START);
    let c = cols4(COL_C_START);
    let empty_hi = cols4(0);

    let mut lookup_constraints: Vec<LVSExpr> = Vec::new();
    let mut u8_cols: Vec<usize> = Vec::new();
    let u16_cols: Vec<usize> = Vec::new();

    for i in 0..4 {
        u8_cols.push(COL_A_START + i);
        u8_cols.push(COL_B_START + i);
        u8_cols.push(COL_C_START + i);
    }

    let ops: &[(usize, WordOp)] = &[
        (COL_ADD_FLAG, WordOp::AddU),
        (COL_SUB_FLAG, WordOp::SubU),
        (COL_XOR_FLAG, WordOp::Xor),
        (COL_OR_FLAG,  WordOp::Or),
        (COL_AND_FLAG, WordOp::And),
    ];

    for (flag_col, op) in ops {
        let alu_c = get_alu_constraint(&a, &b, &c, &empty_hi, op);
        if let Some(impl_c) = make_impl_constraint(1, &col(*flag_col), alu_c, prime) {
            lookup_constraints.push(impl_c);
        }
    }

    let multiplicities: HashSet<usize> = [
        COL_ADD_FLAG, COL_SUB_FLAG, COL_XOR_FLAG, COL_OR_FLAG, COL_AND_FLAG,
    ]
    .into_iter()
    .collect();
    let received_vars: HashSet<usize> = HashSet::new();

    // Extract polynomial AIR constraints from the OpenVM BaseAluCoreAir.
    let bitwise_bus = BitwiseOperationLookupBus::new(BITWISE_OP_LOOKUP_BUS);
    let bitwise_chip = SharedBitwiseOperationLookupChip::<RV32_CELL_BITS>::new(bitwise_bus);
    let core_air = BaseAluCoreChip::<RV32_REGISTER_NUM_LIMBS, RV32_CELL_BITS>::new(
        bitwise_chip,
        BaseAluOpcode::CLASS_OFFSET,
    )
    .air;
    let air = VmAirWrapper {
        adapter: make_test_adapter_air(),
        core: core_air,
    };
    let symbolic = get_symbolic_constraints_openvm::<F, _>(&air);
    let mut air_constraints: Vec<LVSExpr> = symbolic
        .constraints
        .iter()
        .filter(|e| !contains_permutation(e))
        .map(|e| convert_openvm_expr::<F>(e))
        .collect();

    let (refinable_cols, range_types) = prepare_constraints_and_range_type(
        NUM_BASE_ALU_COLS,
        &u8_cols,
        &u16_cols,
        &multiplicities,
        &received_vars,
        &mut air_constraints,
        &lookup_constraints,
        prime,
        true,
    );

    let constraint_info = ConstraintInfo {
        constraints: ZEBRAConstraints::new(air_constraints, lookup_constraints),
        num_total_columns: NUM_BASE_ALU_COLS,
        num_pv_columns: 0,
        output_columns: vec![],
        refinable_cols,
        range_types,
        prime,
    };

    let general_lookup_info = GeneralLookupInfo {
        op_a: (COL_A_START..COL_A_START + 4).collect(),
        op_b: (COL_B_START..COL_B_START + 4).collect(),
        op_c: (COL_C_START..COL_C_START + 4).collect(),
        is_real: vec![
            col(COL_ADD_FLAG),
            col(COL_SUB_FLAG),
            col(COL_XOR_FLAG),
            col(COL_OR_FLAG),
            col(COL_AND_FLAG),
        ],
    };

    (constraint_info, general_lookup_info)
}

// ── Shift constraints (SRL only) ──────────────────────────────────────────────

/// Build ZEBRA constraints for the OpenVM `ShiftCoreAir` chip (SRL only).
pub fn extract_shift_constraints<F: PrimeField32>(
    prime: u32,
) -> (ConstraintInfo, GeneralLookupInfo) {
    let a = cols4(COL_SHIFT_A_START);
    let b = cols4(COL_SHIFT_B_START);
    let c = cols4(COL_SHIFT_C_START);
    let empty_hi = cols4(0);

    let mut lookup_constraints: Vec<LVSExpr> = Vec::new();
    let mut u8_cols: Vec<usize> = Vec::new();
    let u16_cols: Vec<usize> = Vec::new();

    for i in 0..4 {
        u8_cols.push(COL_SHIFT_A_START + i);
        u8_cols.push(COL_SHIFT_B_START + i);
        u8_cols.push(COL_SHIFT_C_START + i);
    }

    let alu_c = get_alu_constraint(&a, &b, &c, &empty_hi, &WordOp::SRL);
    if let Some(impl_c) = make_impl_constraint(1, &col(COL_SHIFT_SRL_FLAG), alu_c, prime) {
        lookup_constraints.push(impl_c);
    }

    let multiplicities: HashSet<usize> = [COL_SHIFT_SRL_FLAG].into_iter().collect();
    let received_vars: HashSet<usize> = HashSet::new();

    // Extract polynomial AIR constraints from the OpenVM ShiftCoreAir.
    let bitwise_bus = BitwiseOperationLookupBus::new(BITWISE_OP_LOOKUP_BUS);
    let bitwise_chip = SharedBitwiseOperationLookupChip::<RV32_CELL_BITS>::new(bitwise_bus);
    let range_bus = VariableRangeCheckerBus::new(5, RV32_CELL_BITS);
    let range_chip = SharedVariableRangeCheckerChip::new(range_bus);
    let core_air = ShiftCoreChip::<RV32_REGISTER_NUM_LIMBS, RV32_CELL_BITS>::new(
        bitwise_chip,
        range_chip,
        ShiftOpcode::CLASS_OFFSET,
    )
    .air;
    let air = VmAirWrapper {
        adapter: make_test_adapter_air(),
        core: core_air,
    };
    let symbolic = get_symbolic_constraints_openvm::<F, _>(&air);
    let mut air_constraints: Vec<LVSExpr> = symbolic
        .constraints
        .iter()
        .filter(|e| !contains_permutation(e))
        .map(|e| convert_openvm_expr::<F>(e))
        .collect();

    let (refinable_cols, range_types) = prepare_constraints_and_range_type(
        NUM_SHIFT_COLS,
        &u8_cols,
        &u16_cols,
        &multiplicities,
        &received_vars,
        &mut air_constraints,
        &lookup_constraints,
        prime,
        true,
    );

    let constraint_info = ConstraintInfo {
        constraints: ZEBRAConstraints::new(air_constraints, lookup_constraints),
        num_total_columns: NUM_SHIFT_COLS,
        num_pv_columns: 0,
        output_columns: vec![],
        refinable_cols,
        range_types,
        prime,
    };

    let general_lookup_info = GeneralLookupInfo {
        op_a: (COL_SHIFT_A_START..COL_SHIFT_A_START + 4).collect(),
        op_b: (COL_SHIFT_B_START..COL_SHIFT_B_START + 4).collect(),
        op_c: (COL_SHIFT_C_START..COL_SHIFT_C_START + 4).collect(),
        is_real: vec![col(COL_SHIFT_SRL_FLAG)],
    };

    (constraint_info, general_lookup_info)
}

// ── Mul constraints ───────────────────────────────────────────────────────────

/// Build ZEBRA constraints for the OpenVM `MultiplicationCoreAir` chip (MUL).
pub fn extract_mul_constraints<F: PrimeField32>(
    prime: u32,
) -> (ConstraintInfo, GeneralLookupInfo) {
    let a = cols4(COL_MUL_A_START);
    let b = cols4(COL_MUL_B_START);
    let c = cols4(COL_MUL_C_START);
    let empty_hi = cols4(0);

    let mut lookup_constraints: Vec<LVSExpr> = Vec::new();
    let mut u8_cols: Vec<usize> = Vec::new();
    let u16_cols: Vec<usize> = Vec::new();

    for i in 0..4 {
        u8_cols.push(COL_MUL_A_START + i);
        u8_cols.push(COL_MUL_B_START + i);
        u8_cols.push(COL_MUL_C_START + i);
    }

    let alu_c = get_alu_constraint(&a, &b, &c, &empty_hi, &WordOp::Mul);
    if let Some(impl_c) = make_impl_constraint(1, &col(COL_MUL_IS_VALID), alu_c, prime) {
        lookup_constraints.push(impl_c);
    }

    let multiplicities: HashSet<usize> = [COL_MUL_IS_VALID].into_iter().collect();
    let received_vars: HashSet<usize> = HashSet::new();

    // Extract polynomial AIR constraints from the OpenVM MultiplicationCoreAir.
    // sizes[0] must be 2^LIMB_BITS, sizes[1] must be >= 2^LIMB_BITS * NUM_LIMBS.
    let range_tuple_bus = RangeTupleCheckerBus::<2>::new(
        RANGE_TUPLE_CHECKER_BUS,
        [1 << RV32_CELL_BITS, (1 << RV32_CELL_BITS) * RV32_REGISTER_NUM_LIMBS as u32],
    );
    let range_tuple_chip = SharedRangeTupleCheckerChip::<2>::new(range_tuple_bus);
    let core_air = MultiplicationCoreChip::<RV32_REGISTER_NUM_LIMBS, RV32_CELL_BITS>::new(
        range_tuple_chip,
        MulOpcode::CLASS_OFFSET,
    )
    .air;
    let air = VmAirWrapper {
        adapter: make_test_adapter_air(),
        core: core_air,
    };
    let symbolic = get_symbolic_constraints_openvm::<F, _>(&air);
    let mut air_constraints: Vec<LVSExpr> = symbolic
        .constraints
        .iter()
        .filter(|e| !contains_permutation(e))
        .map(|e| convert_openvm_expr::<F>(e))
        .collect();

    let (refinable_cols, range_types) = prepare_constraints_and_range_type(
        NUM_MUL_COLS,
        &u8_cols,
        &u16_cols,
        &multiplicities,
        &received_vars,
        &mut air_constraints,
        &lookup_constraints,
        prime,
        true,
    );

    let constraint_info = ConstraintInfo {
        constraints: ZEBRAConstraints::new(air_constraints, lookup_constraints),
        num_total_columns: NUM_MUL_COLS,
        num_pv_columns: 0,
        output_columns: vec![],
        refinable_cols,
        range_types,
        prime,
    };

    let general_lookup_info = GeneralLookupInfo {
        op_a: (COL_MUL_A_START..COL_MUL_A_START + 4).collect(),
        op_b: (COL_MUL_B_START..COL_MUL_B_START + 4).collect(),
        op_c: (COL_MUL_C_START..COL_MUL_C_START + 4).collect(),
        is_real: vec![col(COL_MUL_IS_VALID)],
    };

    (constraint_info, general_lookup_info)
}

// ── LessThan constraints (SLT / SLTU) ─────────────────────────────────────────

/// Build ZEBRA constraints for the OpenVM `LessThanCoreAir` chip (SLT / SLTU).
///
/// Unlike the other chips, the result is a single `cmp_result` column (0 or 1),
/// not a 4-limb word.  We directly constrain:
///   slt_flag  == 1  →  cmp_result == WordSLt(b, c)
///   sltu_flag == 1  →  cmp_result == WordLt(b, c)
pub fn extract_lt_constraints<F: PrimeField32>(
    prime: u32,
) -> (ConstraintInfo, GeneralLookupInfo) {
    let b = cols4(COL_LT_B_START);
    let c = cols4(COL_LT_C_START);

    let mut lookup_constraints: Vec<LVSExpr> = Vec::new();
    let mut u8_cols: Vec<usize> = Vec::new();
    let u16_cols: Vec<usize> = Vec::new();

    for i in 0..4 {
        u8_cols.push(COL_LT_B_START + i);
        u8_cols.push(COL_LT_C_START + i);
    }
    // cmp_result is boolean (0 or 1)
    u8_cols.push(COL_LT_CMP_RESULT);

    // SLT: cmp_result - WordSLt(b, c) == 0
    let slt_c = LVSExpr::Sub(
        Box::new(col(COL_LT_CMP_RESULT)),
        Box::new(LVSExpr::WordSLt(
            b.clone().map(|f| Box::new(f)),
            c.clone().map(|f| Box::new(f)),
        )),
    );
    if let Some(impl_c) = make_impl_constraint(1, &col(COL_LT_SLT_FLAG), slt_c, prime) {
        lookup_constraints.push(impl_c);
    }

    // SLTU: cmp_result - WordLt(b, c) == 0
    let sltu_c = LVSExpr::Sub(
        Box::new(col(COL_LT_CMP_RESULT)),
        Box::new(LVSExpr::WordLt(
            b.clone().map(|f| Box::new(f)),
            c.clone().map(|f| Box::new(f)),
        )),
    );
    if let Some(impl_c) = make_impl_constraint(1, &col(COL_LT_SLTU_FLAG), sltu_c, prime) {
        lookup_constraints.push(impl_c);
    }

    let multiplicities: HashSet<usize> = [COL_LT_SLT_FLAG, COL_LT_SLTU_FLAG]
        .into_iter()
        .collect();
    let received_vars: HashSet<usize> = HashSet::new();

    // Extract polynomial AIR constraints from the OpenVM LessThanCoreAir.
    let bitwise_bus = BitwiseOperationLookupBus::new(BITWISE_OP_LOOKUP_BUS);
    let bitwise_chip = SharedBitwiseOperationLookupChip::<RV32_CELL_BITS>::new(bitwise_bus);
    let core_air = LessThanCoreChip::<RV32_REGISTER_NUM_LIMBS, RV32_CELL_BITS>::new(
        bitwise_chip,
        LessThanOpcode::CLASS_OFFSET,
    )
    .air;
    let air = VmAirWrapper {
        adapter: make_test_adapter_air(),
        core: core_air,
    };
    let symbolic = get_symbolic_constraints_openvm::<F, _>(&air);
    let mut air_constraints: Vec<LVSExpr> = symbolic
        .constraints
        .iter()
        .filter(|e| !contains_permutation(e))
        .map(|e| convert_openvm_expr::<F>(e))
        .collect();

    let (refinable_cols, range_types) = prepare_constraints_and_range_type(
        NUM_LT_COLS,
        &u8_cols,
        &u16_cols,
        &multiplicities,
        &received_vars,
        &mut air_constraints,
        &lookup_constraints,
        prime,
        true,
    );

    let constraint_info = ConstraintInfo {
        constraints: ZEBRAConstraints::new(air_constraints, lookup_constraints),
        num_total_columns: NUM_LT_COLS,
        num_pv_columns: 0,
        output_columns: vec![],
        refinable_cols,
        range_types,
        prime,
    };

    let general_lookup_info = GeneralLookupInfo {
        op_a: vec![COL_LT_CMP_RESULT],
        op_b: (COL_LT_B_START..COL_LT_B_START + 4).collect(),
        op_c: (COL_LT_C_START..COL_LT_C_START + 4).collect(),
        is_real: vec![col(COL_LT_SLT_FLAG), col(COL_LT_SLTU_FLAG)],
    };

    (constraint_info, general_lookup_info)
}

// ── Trace helpers ─────────────────────────────────────────────────────────────

/// Decompose a 32-bit value into 4 LE 8-bit limbs.
pub fn to_limbs(x: u32) -> [u32; 4] {
    [x & 0xFF, (x >> 8) & 0xFF, (x >> 16) & 0xFF, (x >> 24) & 0xFF]
}

/// Execute a BaseALU operation and return the 4-limb result (LE 8-bit limbs).
pub fn run_base_alu(op: BaseAluOpcode, b: &[u32; 4], c: &[u32; 4]) -> [u32; 4] {
    let mask = (1u32 << RV32_CELL_BITS) - 1;
    match op {
        BaseAluOpcode::ADD => {
            let mut a = [0u32; 4];
            let mut carry = 0u32;
            for i in 0..4 {
                let s = b[i] + c[i] + carry;
                a[i] = s & mask;
                carry = s >> RV32_CELL_BITS;
            }
            a
        }
        BaseAluOpcode::SUB => {
            let mut a = [0u32; 4];
            let mut carry = 0u32;
            for i in 0..4 {
                let rhs = c[i] + carry;
                if b[i] >= rhs {
                    a[i] = b[i] - rhs;
                    carry = 0;
                } else {
                    a[i] = b[i] + (1 << RV32_CELL_BITS) - rhs;
                    carry = 1;
                }
            }
            a
        }
        BaseAluOpcode::XOR => [b[0] ^ c[0], b[1] ^ c[1], b[2] ^ c[2], b[3] ^ c[3]],
        BaseAluOpcode::OR  => [b[0] | c[0], b[1] | c[1], b[2] | c[2], b[3] | c[3]],
        BaseAluOpcode::AND => [b[0] & c[0], b[1] & c[1], b[2] & c[2], b[3] & c[3]],
    }
}

/// Build a single abstract trace row for the given BaseALU operation.
pub fn make_base_alu_row(
    b_limbs: [u32; 4],
    c_limbs: [u32; 4],
    op: BaseAluOpcode,
) -> Vec<AbstractInterval> {
    let a_limbs = run_base_alu(op, &b_limbs, &c_limbs);
    let mut row = vec![AbstractInterval::zero(); NUM_BASE_ALU_COLS];

    for i in 0..4 {
        row[COL_A_START + i] = AbstractInterval::from_i128(a_limbs[i] as i128);
        row[COL_B_START + i] = AbstractInterval::from_i128(b_limbs[i] as i128);
        row[COL_C_START + i] = AbstractInterval::from_i128(c_limbs[i] as i128);
    }

    row[COL_ADD_FLAG] = AbstractInterval::from_i128((op == BaseAluOpcode::ADD) as i128);
    row[COL_SUB_FLAG] = AbstractInterval::from_i128((op == BaseAluOpcode::SUB) as i128);
    row[COL_XOR_FLAG] = AbstractInterval::from_i128((op == BaseAluOpcode::XOR) as i128);
    row[COL_OR_FLAG]  = AbstractInterval::from_i128((op == BaseAluOpcode::OR)  as i128);
    row[COL_AND_FLAG] = AbstractInterval::from_i128((op == BaseAluOpcode::AND) as i128);

    row
}

/// Execute a logical right shift and return the 4-limb result.
pub fn run_shift_srl(b: &[u32; 4], c: &[u32; 4]) -> [u32; 4] {
    let b_word = b[0] | (b[1] << 8) | (b[2] << 16) | (b[3] << 24);
    // Only the lower 5 bits of the shift amount are used for a 32-bit shift.
    let shift = c[0] & 0x1F;
    let result = b_word >> shift;
    to_limbs(result)
}

/// Build a single abstract trace row for SRL.
pub fn make_shift_row(
    b_limbs: [u32; 4],
    c_limbs: [u32; 4],
    op: ShiftOpcode,
) -> Vec<AbstractInterval> {
    let mut row = vec![AbstractInterval::zero(); NUM_SHIFT_COLS];

    let a_limbs = match op {
        ShiftOpcode::SRL => run_shift_srl(&b_limbs, &c_limbs),
        // SLL / SRA not yet supported via WordOp; fill with zeros.
        _ => [0u32; 4],
    };

    for i in 0..4 {
        row[COL_SHIFT_A_START + i] = AbstractInterval::from_i128(a_limbs[i] as i128);
        row[COL_SHIFT_B_START + i] = AbstractInterval::from_i128(b_limbs[i] as i128);
        row[COL_SHIFT_C_START + i] = AbstractInterval::from_i128(c_limbs[i] as i128);
    }

    // opcode flags: SLL=20, SRL=21, SRA=22
    row[ADAPTER_WIDTH + 12] = AbstractInterval::from_i128((op == ShiftOpcode::SLL) as i128);
    row[COL_SHIFT_SRL_FLAG]  = AbstractInterval::from_i128((op == ShiftOpcode::SRL) as i128);
    row[ADAPTER_WIDTH + 14] = AbstractInterval::from_i128((op == ShiftOpcode::SRA) as i128);

    row
}

/// Execute MUL and return the 4-limb result (low 32 bits of b*c).
pub fn run_mul(b: &[u32; 4], c: &[u32; 4]) -> [u32; 4] {
    let b_word = b[0] | (b[1] << 8) | (b[2] << 16) | (b[3] << 24);
    let c_word = c[0] | (c[1] << 8) | (c[2] << 16) | (c[3] << 24);
    to_limbs(b_word.wrapping_mul(c_word))
}

/// Build a single abstract trace row for MUL.
pub fn make_mul_row(b_limbs: [u32; 4], c_limbs: [u32; 4]) -> Vec<AbstractInterval> {
    let a_limbs = run_mul(&b_limbs, &c_limbs);
    let mut row = vec![AbstractInterval::zero(); NUM_MUL_COLS];

    for i in 0..4 {
        row[COL_MUL_A_START + i] = AbstractInterval::from_i128(a_limbs[i] as i128);
        row[COL_MUL_B_START + i] = AbstractInterval::from_i128(b_limbs[i] as i128);
        row[COL_MUL_C_START + i] = AbstractInterval::from_i128(c_limbs[i] as i128);
    }
    row[COL_MUL_IS_VALID] = AbstractInterval::from_i128(1);

    row
}

/// Execute a LessThan comparison and return cmp_result (0 or 1).
pub fn run_lt(op: LessThanOpcode, b: &[u32; 4], c: &[u32; 4]) -> u32 {
    let b_word = b[0] | (b[1] << 8) | (b[2] << 16) | (b[3] << 24);
    let c_word = c[0] | (c[1] << 8) | (c[2] << 16) | (c[3] << 24);
    match op {
        LessThanOpcode::SLT  => ((b_word as i32) < (c_word as i32)) as u32,
        LessThanOpcode::SLTU => (b_word < c_word) as u32,
    }
}

/// Build a single abstract trace row for SLT / SLTU.
///
/// Computes all witness fields including b_msb_f, c_msb_f, diff_marker, diff_val.
pub fn make_lt_row(
    b_limbs: [u32; 4],
    c_limbs: [u32; 4],
    op: LessThanOpcode,
) -> Vec<AbstractInterval> {
    let cmp = run_lt(op, &b_limbs, &c_limbs);
    let cmp_result = cmp == 1;
    let signed = op == LessThanOpcode::SLT;

    // Signed MSB field values: negative i128 for limbs >= 128 when signed.
    let b_sign = signed && b_limbs[3] >= 128;
    let c_sign = signed && c_limbs[3] >= 128;
    let b_msb_f: i128 = if b_sign { b_limbs[3] as i128 - 256 } else { b_limbs[3] as i128 };
    let c_msb_f: i128 = if c_sign { c_limbs[3] as i128 - 256 } else { c_limbs[3] as i128 };

    // Find the most-significant index where b and c differ (scanning from MSB).
    let mut diff_idx = 4usize; // 4 == all limbs equal
    for i in (0..4).rev() {
        if b_limbs[i] != c_limbs[i] {
            diff_idx = i;
            break;
        }
    }

    // diff_val = positive difference at diff_idx, using signed MSB values at limb 3.
    let diff_val: i128 = if diff_idx == 4 {
        0
    } else if diff_idx == 3 {
        // MSB position uses signed field elements.
        let raw = if cmp_result { c_msb_f - b_msb_f } else { b_msb_f - c_msb_f };
        field_as_canonical_u32(raw, BABY_BEAR_PRIME) as i128
    } else if cmp_result {
        c_limbs[diff_idx] as i128 - b_limbs[diff_idx] as i128
    } else {
        b_limbs[diff_idx] as i128 - c_limbs[diff_idx] as i128
    };

    let mut row = vec![AbstractInterval::zero(); NUM_LT_COLS];
    for i in 0..4 {
        row[COL_LT_B_START + i] = AbstractInterval::from_i128(b_limbs[i] as i128);
        row[COL_LT_C_START + i] = AbstractInterval::from_i128(c_limbs[i] as i128);
    }
    row[COL_LT_CMP_RESULT] = AbstractInterval::from_i128(cmp as i128);
    row[COL_LT_SLT_FLAG]   = AbstractInterval::from_i128((op == LessThanOpcode::SLT)  as i128);
    row[COL_LT_SLTU_FLAG]  = AbstractInterval::from_i128((op == LessThanOpcode::SLTU) as i128);
    row[COL_LT_B_MSB_F]    = AbstractInterval::from_i128(b_msb_f);
    row[COL_LT_C_MSB_F]    = AbstractInterval::from_i128(c_msb_f);
    for i in 0..4 {
        row[COL_LT_DIFF_MARKER_START + i] = AbstractInterval::from_i128((i == diff_idx) as i128);
    }
    row[COL_LT_DIFF_VAL] = AbstractInterval::from_i128(diff_val);

    row
}

// ── Field arithmetic helpers ──────────────────────────────────────────────────

/// Return `base^exp mod modulus` using binary exponentiation.
fn mod_pow_u64(mut base: u64, mut exp: u64, modulus: u64) -> u64 {
    let mut result = 1u64;
    base %= modulus;
    while exp > 0 {
        if exp & 1 == 1 {
            result = (result as u128 * base as u128 % modulus as u128) as u64;
        }
        base = (base as u128 * base as u128 % modulus as u128) as u64;
        exp >>= 1;
    }
    result
}

/// Return the multiplicative inverse of `x` in BabyBear (Fermat's little theorem).
///
/// Stores the result as a canonical positive representative in [1, prime-1],
/// suitable for direct use in i128 interval arithmetic.
pub fn baby_bear_field_inverse(x: i128) -> i128 {
    let p = BABY_BEAR_PRIME as u64;
    let xc = ((x % p as i128 + p as i128) % p as i128) as u64;
    mod_pow_u64(xc, p - 2, p) as i128
}

/// Reduce an i128 value to the canonical representative in [0, prime).
fn field_as_canonical_u32(x: i128, prime: u32) -> u32 {
    let p = prime as i128;
    ((x % p + p) % p) as u32
}

// ── BranchEqual constraints (BEQ / BNE) ──────────────────────────────────────

/// Build ZEBRA constraints for the OpenVM `BranchEqualCoreAir` (BEQ / BNE).
pub fn extract_branch_eq_constraints<F: PrimeField32>(
    prime: u32,
) -> (ConstraintInfo, GeneralLookupInfo) {
    let a = cols4(COL_BREQ_A_START);
    let b = cols4(COL_BREQ_B_START);

    let mut lookup_constraints: Vec<LVSExpr> = Vec::new();
    let mut u8_cols: Vec<usize> = Vec::new();
    let u16_cols: Vec<usize> = Vec::new();

    for i in 0..4 {
        u8_cols.push(COL_BREQ_A_START + i);
        u8_cols.push(COL_BREQ_B_START + i);
    }

    // BEQ: cmp_result == WordEq(a, b)
    let beq_c = LVSExpr::Sub(
        Box::new(col(COL_BREQ_CMP_RESULT)),
        Box::new(LVSExpr::WordEq(
            a.clone().map(|f| Box::new(f)),
            b.clone().map(|f| Box::new(f)),
        )),
    );
    if let Some(impl_c) = make_impl_constraint(1, &col(COL_BREQ_BEQ_FLAG), beq_c, prime) {
        lookup_constraints.push(impl_c);
    }

    // BNE: cmp_result == WordNEq(a, b)
    let bne_c = LVSExpr::Sub(
        Box::new(col(COL_BREQ_CMP_RESULT)),
        Box::new(LVSExpr::WordNEq(
            a.clone().map(|f| Box::new(f)),
            b.clone().map(|f| Box::new(f)),
        )),
    );
    if let Some(impl_c) = make_impl_constraint(1, &col(COL_BREQ_BNE_FLAG), bne_c, prime) {
        lookup_constraints.push(impl_c);
    }

    let multiplicities: HashSet<usize> = [COL_BREQ_BEQ_FLAG, COL_BREQ_BNE_FLAG]
        .into_iter()
        .collect();
    let received_vars: HashSet<usize> = HashSet::new();

    let core_air =
        BranchEqualCoreChip::<RV32_REGISTER_NUM_LIMBS>::new(BranchEqualOpcode::CLASS_OFFSET, 4)
            .air;
    let air = VmAirWrapper {
        adapter: make_test_adapter_air(),
        core: core_air,
    };
    let symbolic = get_symbolic_constraints_openvm::<F, _>(&air);
    let mut air_constraints: Vec<LVSExpr> = symbolic
        .constraints
        .iter()
        .filter(|e| !contains_permutation(e))
        .map(|e| convert_openvm_expr::<F>(e))
        .collect();

    let (refinable_cols, range_types) = prepare_constraints_and_range_type(
        NUM_BREQ_COLS,
        &u8_cols,
        &u16_cols,
        &multiplicities,
        &received_vars,
        &mut air_constraints,
        &lookup_constraints,
        prime,
        true,
    );

    let constraint_info = ConstraintInfo {
        constraints: ZEBRAConstraints::new(air_constraints, lookup_constraints),
        num_total_columns: NUM_BREQ_COLS,
        num_pv_columns: 0,
        output_columns: vec![],
        refinable_cols,
        range_types,
        prime,
    };

    let general_lookup_info = GeneralLookupInfo {
        op_a: (COL_BREQ_A_START..COL_BREQ_A_START + 4).collect(),
        op_b: (COL_BREQ_B_START..COL_BREQ_B_START + 4).collect(),
        op_c: vec![],
        is_real: vec![col(COL_BREQ_BEQ_FLAG), col(COL_BREQ_BNE_FLAG)],
    };

    (constraint_info, general_lookup_info)
}

/// Execute a BEQ / BNE comparison and return cmp_result.
pub fn run_branch_eq(op: BranchEqualOpcode, a: &[u32; 4], b: &[u32; 4]) -> u32 {
    let eq = a == b;
    match op {
        BranchEqualOpcode::BEQ => eq as u32,
        BranchEqualOpcode::BNE => (!eq) as u32,
    }
}

/// Build a single abstract trace row for BEQ / BNE.
///
/// Fills the `diff_inv_marker` witness: sets the inverse of `a[i] - b[i]` at the first
/// differing limb index, zero elsewhere.
pub fn make_branch_eq_row(
    a_limbs: [u32; 4],
    b_limbs: [u32; 4],
    op: BranchEqualOpcode,
) -> Vec<AbstractInterval> {
    let cmp = run_branch_eq(op, &a_limbs, &b_limbs);
    let mut row = vec![AbstractInterval::zero(); NUM_BREQ_COLS];

    for i in 0..4 {
        row[COL_BREQ_A_START + i] = AbstractInterval::from_i128(a_limbs[i] as i128);
        row[COL_BREQ_B_START + i] = AbstractInterval::from_i128(b_limbs[i] as i128);
    }
    row[COL_BREQ_CMP_RESULT] = AbstractInterval::from_i128(cmp as i128);
    row[COL_BREQ_BEQ_FLAG] = AbstractInterval::from_i128((op == BranchEqualOpcode::BEQ) as i128);
    row[COL_BREQ_BNE_FLAG] = AbstractInterval::from_i128((op == BranchEqualOpcode::BNE) as i128);

    // diff_inv_marker: set at the first position where a[i] != b[i].
    for i in 0..4 {
        if a_limbs[i] != b_limbs[i] {
            let diff = a_limbs[i] as i128 - b_limbs[i] as i128;
            let inv = baby_bear_field_inverse(diff);
            row[COL_BREQ_DIFF_INV_START + i] = AbstractInterval::from_i128(inv);
            break;
        }
    }

    row
}

// ── BranchLessThan constraints (BLT / BLTU / BGE / BGEU) ─────────────────────

/// Build ZEBRA constraints for the OpenVM `BranchLessThanCoreAir` (BLT/BLTU/BGE/BGEU).
pub fn extract_branch_lt_constraints<F: PrimeField32>(
    prime: u32,
) -> (ConstraintInfo, GeneralLookupInfo) {
    let a = cols4(COL_BLT_A_START);
    let b = cols4(COL_BLT_B_START);

    let mut lookup_constraints: Vec<LVSExpr> = Vec::new();
    let mut u8_cols: Vec<usize> = Vec::new();
    let u16_cols: Vec<usize> = Vec::new();

    for i in 0..4 {
        u8_cols.push(COL_BLT_A_START + i);
        u8_cols.push(COL_BLT_B_START + i);
    }

    // BLT:  cmp_result == WordSLt(a, b)
    let blt_c = LVSExpr::Sub(
        Box::new(col(COL_BLT_CMP_RESULT)),
        Box::new(LVSExpr::WordSLt(
            a.clone().map(|f| Box::new(f)),
            b.clone().map(|f| Box::new(f)),
        )),
    );
    if let Some(c) = make_impl_constraint(1, &col(COL_BLT_BLT_FLAG), blt_c, prime) {
        lookup_constraints.push(c);
    }

    // BLTU: cmp_result == WordLt(a, b)
    let bltu_c = LVSExpr::Sub(
        Box::new(col(COL_BLT_CMP_RESULT)),
        Box::new(LVSExpr::WordLt(
            a.clone().map(|f| Box::new(f)),
            b.clone().map(|f| Box::new(f)),
        )),
    );
    if let Some(c) = make_impl_constraint(1, &col(COL_BLT_BLTU_FLAG), bltu_c, prime) {
        lookup_constraints.push(c);
    }

    // BGE: cmp_result == (1 - WordSLt(a, b)) but only when a != b
    //      More precisely: cmp_result == WordSLt(b, a) || a == b
    //      Equivalently: cmp_result == 1 - WordSLt(a, b)
    let bge_c = LVSExpr::Sub(
        Box::new(col(COL_BLT_CMP_RESULT)),
        Box::new(LVSExpr::Sub(
            Box::new(LVSExpr::Constant(AbstractInterval::one())),
            Box::new(LVSExpr::WordSLt(
                a.clone().map(|f| Box::new(f)),
                b.clone().map(|f| Box::new(f)),
            )),
        )),
    );
    if let Some(c) = make_impl_constraint(1, &col(COL_BLT_BGE_FLAG), bge_c, prime) {
        lookup_constraints.push(c);
    }

    // BGEU: cmp_result == 1 - WordLt(a, b)
    let bgeu_c = LVSExpr::Sub(
        Box::new(col(COL_BLT_CMP_RESULT)),
        Box::new(LVSExpr::Sub(
            Box::new(LVSExpr::Constant(AbstractInterval::one())),
            Box::new(LVSExpr::WordLt(
                a.clone().map(|f| Box::new(f)),
                b.clone().map(|f| Box::new(f)),
            )),
        )),
    );
    if let Some(c) = make_impl_constraint(1, &col(COL_BLT_BGEU_FLAG), bgeu_c, prime) {
        lookup_constraints.push(c);
    }

    let multiplicities: HashSet<usize> = [
        COL_BLT_BLT_FLAG,
        COL_BLT_BLTU_FLAG,
        COL_BLT_BGE_FLAG,
        COL_BLT_BGEU_FLAG,
    ]
    .into_iter()
    .collect();
    let received_vars: HashSet<usize> = HashSet::new();

    let bitwise_bus = BitwiseOperationLookupBus::new(BITWISE_OP_LOOKUP_BUS);
    let bitwise_chip = SharedBitwiseOperationLookupChip::<RV32_CELL_BITS>::new(bitwise_bus);
    let core_air = BranchLessThanCoreChip::<RV32_REGISTER_NUM_LIMBS, RV32_CELL_BITS>::new(
        bitwise_chip,
        BranchLessThanOpcode::CLASS_OFFSET,
    )
    .air;
    let air = VmAirWrapper {
        adapter: make_test_adapter_air(),
        core: core_air,
    };
    let symbolic = get_symbolic_constraints_openvm::<F, _>(&air);
    let mut air_constraints: Vec<LVSExpr> = symbolic
        .constraints
        .iter()
        .filter(|e| !contains_permutation(e))
        .map(|e| convert_openvm_expr::<F>(e))
        .collect();

    let (refinable_cols, range_types) = prepare_constraints_and_range_type(
        NUM_BLT_COLS,
        &u8_cols,
        &u16_cols,
        &multiplicities,
        &received_vars,
        &mut air_constraints,
        &lookup_constraints,
        prime,
        true,
    );

    let constraint_info = ConstraintInfo {
        constraints: ZEBRAConstraints::new(air_constraints, lookup_constraints),
        num_total_columns: NUM_BLT_COLS,
        num_pv_columns: 0,
        output_columns: vec![],
        refinable_cols,
        range_types,
        prime,
    };

    let general_lookup_info = GeneralLookupInfo {
        op_a: (COL_BLT_A_START..COL_BLT_A_START + 4).collect(),
        op_b: (COL_BLT_B_START..COL_BLT_B_START + 4).collect(),
        op_c: vec![],
        is_real: vec![
            col(COL_BLT_BLT_FLAG),
            col(COL_BLT_BLTU_FLAG),
            col(COL_BLT_BGE_FLAG),
            col(COL_BLT_BGEU_FLAG),
        ],
    };

    (constraint_info, general_lookup_info)
}

/// Execute a BLT / BLTU / BGE / BGEU comparison and return cmp_result (branch taken = 1).
pub fn run_branch_lt(op: BranchLessThanOpcode, a: &[u32; 4], b: &[u32; 4]) -> u32 {
    let a_word = a[0] | (a[1] << 8) | (a[2] << 16) | (a[3] << 24);
    let b_word = b[0] | (b[1] << 8) | (b[2] << 16) | (b[3] << 24);
    match op {
        BranchLessThanOpcode::BLT  => ((a_word as i32) < (b_word as i32)) as u32,
        BranchLessThanOpcode::BLTU => (a_word < b_word) as u32,
        BranchLessThanOpcode::BGE  => ((a_word as i32) >= (b_word as i32)) as u32,
        BranchLessThanOpcode::BGEU => (a_word >= b_word) as u32,
    }
}

/// Build a single abstract trace row for BLT / BLTU / BGE / BGEU.
///
/// Fills a_msb_f, b_msb_f, cmp_lt, diff_marker, diff_val witness columns.
pub fn make_branch_lt_row(
    a_limbs: [u32; 4],
    b_limbs: [u32; 4],
    op: BranchLessThanOpcode,
) -> Vec<AbstractInterval> {
    let cmp_result = run_branch_lt(op, &a_limbs, &b_limbs);
    let signed = matches!(op, BranchLessThanOpcode::BLT | BranchLessThanOpcode::BGE);
    let ge_op = matches!(op, BranchLessThanOpcode::BGE | BranchLessThanOpcode::BGEU);

    let a_sign = signed && a_limbs[3] >= 128;
    let b_sign = signed && b_limbs[3] >= 128;
    let a_msb_f: i128 = if a_sign { a_limbs[3] as i128 - 256 } else { a_limbs[3] as i128 };
    let b_msb_f: i128 = if b_sign { b_limbs[3] as i128 - 256 } else { b_limbs[3] as i128 };

    // cmp_lt = (a < b) before applying ge inversion.
    let cmp_lt = cmp_result as u32 ^ ge_op as u32;

    // Find diff_idx: most significant position where a[i] != b[i].
    let mut diff_idx = 4usize;
    for i in (0..4).rev() {
        if a_limbs[i] != b_limbs[i] {
            diff_idx = i;
            break;
        }
    }

    // diff_val = positive difference at diff_idx under the BranchLessThan convention.
    let diff_val: i128 = if diff_idx == 4 {
        0
    } else if diff_idx == 3 {
        // Use signed MSB field elements.
        let raw = if cmp_lt == 1 { b_msb_f - a_msb_f } else { a_msb_f - b_msb_f };
        field_as_canonical_u32(raw, BABY_BEAR_PRIME) as i128
    } else if cmp_lt == 1 {
        b_limbs[diff_idx] as i128 - a_limbs[diff_idx] as i128
    } else {
        a_limbs[diff_idx] as i128 - b_limbs[diff_idx] as i128
    };

    let mut row = vec![AbstractInterval::zero(); NUM_BLT_COLS];
    for i in 0..4 {
        row[COL_BLT_A_START + i] = AbstractInterval::from_i128(a_limbs[i] as i128);
        row[COL_BLT_B_START + i] = AbstractInterval::from_i128(b_limbs[i] as i128);
    }
    row[COL_BLT_CMP_RESULT] = AbstractInterval::from_i128(cmp_result as i128);
    row[COL_BLT_BLT_FLAG]   = AbstractInterval::from_i128((op == BranchLessThanOpcode::BLT)  as i128);
    row[COL_BLT_BLTU_FLAG]  = AbstractInterval::from_i128((op == BranchLessThanOpcode::BLTU) as i128);
    row[COL_BLT_BGE_FLAG]   = AbstractInterval::from_i128((op == BranchLessThanOpcode::BGE)  as i128);
    row[COL_BLT_BGEU_FLAG]  = AbstractInterval::from_i128((op == BranchLessThanOpcode::BGEU) as i128);
    row[COL_BLT_A_MSB_F]    = AbstractInterval::from_i128(a_msb_f);
    row[COL_BLT_B_MSB_F]    = AbstractInterval::from_i128(b_msb_f);
    row[COL_BLT_CMP_LT]     = AbstractInterval::from_i128(cmp_lt as i128);
    for i in 0..4 {
        row[COL_BLT_DIFF_MARKER_START + i] = AbstractInterval::from_i128((i == diff_idx) as i128);
    }
    row[COL_BLT_DIFF_VAL] = AbstractInterval::from_i128(diff_val);

    row
}

// ── JalLui constraints (JAL / LUI) ───────────────────────────────────────────

/// Build ZEBRA constraints for the OpenVM `Rv32JalLuiCoreAir` (JAL / LUI).
pub fn extract_jal_constraints<F: PrimeField32>(
    prime: u32,
) -> (ConstraintInfo, GeneralLookupInfo) {
    let u8_cols: Vec<usize> = (COL_JAL_RD_START..COL_JAL_RD_START + 4).collect();
    let u16_cols: Vec<usize> = Vec::new();
    let lookup_constraints: Vec<LVSExpr> = Vec::new();
    let multiplicities: HashSet<usize> = [COL_JAL_IS_JAL, COL_JAL_IS_LUI].into_iter().collect();
    let received_vars: HashSet<usize> = HashSet::new();

    let bitwise_bus = BitwiseOperationLookupBus::new(BITWISE_OP_LOOKUP_BUS);
    let bitwise_chip = SharedBitwiseOperationLookupChip::<RV32_CELL_BITS>::new(bitwise_bus);
    let core_air = Rv32JalLuiCoreChip::new(bitwise_chip).air;
    let air = VmAirWrapper {
        adapter: make_test_adapter_air(),
        core: core_air,
    };
    let symbolic = get_symbolic_constraints_openvm::<F, _>(&air);
    let mut air_constraints: Vec<LVSExpr> = symbolic
        .constraints
        .iter()
        .filter(|e| !contains_permutation(e))
        .map(|e| convert_openvm_expr::<F>(e))
        .collect();

    let (refinable_cols, range_types) = prepare_constraints_and_range_type(
        NUM_JAL_COLS,
        &u8_cols,
        &u16_cols,
        &multiplicities,
        &received_vars,
        &mut air_constraints,
        &lookup_constraints,
        prime,
        true,
    );

    let constraint_info = ConstraintInfo {
        constraints: ZEBRAConstraints::new(air_constraints, lookup_constraints),
        num_total_columns: NUM_JAL_COLS,
        num_pv_columns: 0,
        output_columns: vec![],
        refinable_cols,
        range_types,
        prime,
    };

    let general_lookup_info = GeneralLookupInfo {
        op_a: (COL_JAL_RD_START..COL_JAL_RD_START + 4).collect(),
        op_b: vec![],
        op_c: vec![],
        is_real: vec![col(COL_JAL_IS_JAL), col(COL_JAL_IS_LUI)],
    };

    (constraint_info, general_lookup_info)
}

/// Build a single abstract trace row for JAL.
///
/// `from_pc` is the current program counter (stored at trace column 0).
/// `imm` is the signed jump offset.
pub fn make_jal_row(from_pc: u32, imm: i32) -> Vec<AbstractInterval> {
    // rd_data = limbs of (from_pc + 4)
    let rd_data = to_limbs(from_pc + 4);
    let mut row = vec![AbstractInterval::zero(); NUM_JAL_COLS];

    // Column 0 (TestAdapterCols.from_pc) must match from_pc so that the
    // polynomial constraint `is_jal * (rd_composition - from_pc - 4) == 0` passes.
    row[0] = AbstractInterval::from_i128(from_pc as i128);

    // JAL imm stored as canonical field element; negative imm wraps via BABY_BEAR_PRIME.
    let imm_field: i128 = if imm >= 0 {
        imm as i128
    } else {
        BABY_BEAR_PRIME as i128 + imm as i128
    };
    row[COL_JAL_IMM] = AbstractInterval::from_i128(imm_field);
    for i in 0..4 {
        row[COL_JAL_RD_START + i] = AbstractInterval::from_i128(rd_data[i] as i128);
    }
    row[COL_JAL_IS_JAL] = AbstractInterval::from_i128(1);

    row
}

/// Build a single abstract trace row for LUI.
///
/// `imm` is the 20-bit upper immediate (bits 31..12 of the encoded value).
pub fn make_lui_row(imm: u32) -> Vec<AbstractInterval> {
    // rd = imm << 12
    let rd_word = imm << 12;
    let rd_data = to_limbs(rd_word);
    let mut row = vec![AbstractInterval::zero(); NUM_JAL_COLS];

    row[COL_JAL_IMM] = AbstractInterval::from_i128(imm as i128);
    for i in 0..4 {
        row[COL_JAL_RD_START + i] = AbstractInterval::from_i128(rd_data[i] as i128);
    }
    row[COL_JAL_IS_LUI] = AbstractInterval::from_i128(1);

    row
}

// ── Jalr constraints (JALR) ───────────────────────────────────────────────────

/// Build ZEBRA constraints for the OpenVM `Rv32JalrCoreAir` (JALR).
///
/// Because `Rv32JalrCoreAir` requires `SignedImmInstruction` (not compatible with
/// `TestAdapterAir` / `DynArray`), we write the polynomial constraints manually rather
/// than using `get_symbolic_constraints_openvm`.
pub fn extract_jalr_constraints(prime: u32) -> (ConstraintInfo, GeneralLookupInfo) {
    let u8_cols: Vec<usize> = (COL_JALR_RS1_START..COL_JALR_RS1_START + 4).collect();
    let u16_cols: Vec<usize> = Vec::new();

    let multiplicities: HashSet<usize> = [COL_JALR_IS_VALID].into_iter().collect();
    let received_vars: HashSet<usize> = HashSet::new();

    let inv_65536 = baby_bear_field_inverse(65536);
    let c_inv = || LVSExpr::Constant(AbstractInterval::from_i128(inv_65536));
    let one = || LVSExpr::Constant(AbstractInterval::one());
    let const_i = |v: i128| LVSExpr::Constant(AbstractInterval::from_i128(v));

    // Helpers to build expressions concisely.
    let bool_constraint = |c: usize| {
        LVSExpr::Mul(
            Box::new(col(c)),
            Box::new(LVSExpr::Sub(Box::new(one()), Box::new(col(c)))),
        )
    };

    let rs1_0 = col(COL_JALR_RS1_START);
    let rs1_1 = col(COL_JALR_RS1_START + 1);
    let rs1_2 = col(COL_JALR_RS1_START + 2);
    let rs1_3 = col(COL_JALR_RS1_START + 3);
    let imm = col(COL_JALR_IMM);
    let imm_sign = col(COL_JALR_IMM_SIGN);
    let to_pc_lsb = col(COL_JALR_TO_PC_LSB);
    let to_pc_l0 = col(COL_JALR_TO_PC_LIMBS_START);
    let to_pc_l1 = col(COL_JALR_TO_PC_LIMBS_START + 1);
    let is_valid = col(COL_JALR_IS_VALID);

    // rs1_01 = rs1[0] + rs1[1] * 256
    let rs1_01 = LVSExpr::Add(
        Box::new(rs1_0),
        Box::new(LVSExpr::Mul(Box::new(rs1_1), Box::new(const_i(256)))),
    );
    // rs1_23 = rs1[2] + rs1[3] * 256
    let rs1_23 = LVSExpr::Add(
        Box::new(rs1_2),
        Box::new(LVSExpr::Mul(Box::new(rs1_3), Box::new(const_i(256)))),
    );

    // val1 = rs1_01 + imm - to_pc_l0 * 2 - to_pc_lsb
    let val1 = LVSExpr::Sub(
        Box::new(LVSExpr::Sub(
            Box::new(LVSExpr::Add(Box::new(rs1_01.clone()), Box::new(imm.clone()))),
            Box::new(LVSExpr::Mul(Box::new(const_i(2)), Box::new(to_pc_l0.clone()))),
        )),
        Box::new(to_pc_lsb.clone()),
    );

    // carry1 = val1 * inv_65536
    let carry1 = LVSExpr::Mul(Box::new(val1), Box::new(c_inv()));

    // imm_extend = imm_sign * 65535
    let imm_extend = LVSExpr::Mul(Box::new(imm_sign.clone()), Box::new(const_i(65535)));

    // val2 = rs1_23 + imm_extend + carry1 - to_pc_l1
    let val2 = LVSExpr::Sub(
        Box::new(LVSExpr::Add(
            Box::new(LVSExpr::Add(Box::new(rs1_23), Box::new(imm_extend))),
            Box::new(carry1.clone()),
        )),
        Box::new(to_pc_l1),
    );

    // carry2 = val2 * inv_65536
    let carry2 = LVSExpr::Mul(Box::new(val2), Box::new(c_inv()));

    // is_valid * carry * (1 - carry) == 0
    let carry_bool = |carry: LVSExpr| {
        LVSExpr::Mul(
            Box::new(is_valid.clone()),
            Box::new(LVSExpr::Mul(
                Box::new(carry.clone()),
                Box::new(LVSExpr::Sub(Box::new(one()), Box::new(carry))),
            )),
        )
    };

    let mut air_constraints: Vec<LVSExpr> = vec![
        bool_constraint(COL_JALR_IS_VALID),
        bool_constraint(COL_JALR_IMM_SIGN),
        bool_constraint(COL_JALR_TO_PC_LSB),
        carry_bool(carry1.clone()),
        carry_bool(carry2),
    ];

    let lookup_constraints: Vec<LVSExpr> = Vec::new();

    let (refinable_cols, range_types) = prepare_constraints_and_range_type(
        NUM_JALR_COLS,
        &u8_cols,
        &u16_cols,
        &multiplicities,
        &received_vars,
        &mut air_constraints,
        &lookup_constraints,
        prime,
        true,
    );

    let constraint_info = ConstraintInfo {
        constraints: ZEBRAConstraints::new(air_constraints, lookup_constraints),
        num_total_columns: NUM_JALR_COLS,
        num_pv_columns: 0,
        output_columns: vec![],
        refinable_cols,
        range_types,
        prime,
    };

    let general_lookup_info = GeneralLookupInfo {
        op_a: (COL_JALR_RD_START..COL_JALR_RD_START + 3).collect(),
        op_b: (COL_JALR_RS1_START..COL_JALR_RS1_START + 4).collect(),
        op_c: vec![],
        is_real: vec![col(COL_JALR_IS_VALID)],
    };

    (constraint_info, general_lookup_info)
}

/// Build a single abstract trace row for JALR.
///
/// `rs1_limbs` — register rs1 as 4 LE 8-bit limbs.
/// `imm` — 16-bit (unsigned) immediate field element as stored in the trace.
/// `imm_sign` — 0 or 1; 1 means the original 32-bit immediate was negative.
///
/// `from_pc` is set to 0 (the TestAdapterAir convention for single-row traces).
pub fn make_jalr_row(rs1_limbs: [u32; 4], imm: u32, imm_sign: u32) -> Vec<AbstractInterval> {
    let from_pc: u32 = 0;
    let imm_extended = imm.wrapping_add(imm_sign.wrapping_mul(0xffff0000));
    let rs1_val = rs1_limbs[0]
        | (rs1_limbs[1] << 8)
        | (rs1_limbs[2] << 16)
        | (rs1_limbs[3] << 24);

    let to_pc_raw = rs1_val.wrapping_add(imm_extended);
    let to_pc_lsb = to_pc_raw & 1;
    let to_pc = to_pc_raw - to_pc_lsb;

    let to_pc_limbs = [
        (to_pc >> 1) & 0x7FFF,
        (to_pc >> 16) & 0x7FFF,
    ];

    // rd_data: limbs 1..3 of (from_pc + 4) = limbs 1..3 of 4 = [0, 0, 0]
    let pc4 = to_limbs(from_pc + 4);
    let rd_data_3msb = [pc4[1], pc4[2], pc4[3]];

    let mut row = vec![AbstractInterval::zero(); NUM_JALR_COLS];
    row[COL_JALR_IMM] = AbstractInterval::from_i128(imm as i128);
    for i in 0..4 {
        row[COL_JALR_RS1_START + i] = AbstractInterval::from_i128(rs1_limbs[i] as i128);
    }
    for i in 0..3 {
        row[COL_JALR_RD_START + i] = AbstractInterval::from_i128(rd_data_3msb[i] as i128);
    }
    row[COL_JALR_IS_VALID] = AbstractInterval::from_i128(1);
    row[COL_JALR_TO_PC_LSB] = AbstractInterval::from_i128(to_pc_lsb as i128);
    row[COL_JALR_TO_PC_LIMBS_START] = AbstractInterval::from_i128(to_pc_limbs[0] as i128);
    row[COL_JALR_TO_PC_LIMBS_START + 1] = AbstractInterval::from_i128(to_pc_limbs[1] as i128);
    row[COL_JALR_IMM_SIGN] = AbstractInterval::from_i128(imm_sign as i128);

    row
}
