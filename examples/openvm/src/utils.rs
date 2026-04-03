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
    BaseAluCoreChip, LessThanCoreChip, MultiplicationCoreChip, ShiftCoreChip,
};
use openvm_rv32im_transpiler::{BaseAluOpcode, LessThanOpcode, MulOpcode, ShiftOpcode};
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
pub fn make_lt_row(
    b_limbs: [u32; 4],
    c_limbs: [u32; 4],
    op: LessThanOpcode,
) -> Vec<AbstractInterval> {
    let cmp = run_lt(op, &b_limbs, &c_limbs);
    let mut row = vec![AbstractInterval::zero(); NUM_LT_COLS];

    for i in 0..4 {
        row[COL_LT_B_START + i] = AbstractInterval::from_i128(b_limbs[i] as i128);
        row[COL_LT_C_START + i] = AbstractInterval::from_i128(c_limbs[i] as i128);
    }
    row[COL_LT_CMP_RESULT] = AbstractInterval::from_i128(cmp as i128);
    row[COL_LT_SLT_FLAG]   = AbstractInterval::from_i128((op == LessThanOpcode::SLT)  as i128);
    row[COL_LT_SLTU_FLAG]  = AbstractInterval::from_i128((op == LessThanOpcode::SLTU) as i128);

    row
}
