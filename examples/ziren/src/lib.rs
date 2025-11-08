pub mod executor;
pub mod p3_to_tv;
pub mod pv_constraints;
pub mod state;
pub mod table_generator;

/*
CpuCols { shard: 1, clk_16bit_limb: 0, clk_8bit_limb: 0, shard_to_send: 0, clk_to_send: 0, pc: 4, next_pc: 8, next_next_pc: 12, instruction: InstructionCols { opcode: 0, op_a: 1, op_b: Word([5, 0, 0, 0]), op_c: Word([3, 0, 0, 0]), op_a_0: 0, imm_b: 0, imm_c: 1 }, num_extra_cycles: 0, is_memory: 0, is_rw_a: 0, is_write_hi: 0, is_halt: 0, is_sequential: 1, op_a_value: Word([3, 0, 0, 0]), hi_or_prev_a: Word([0, 0, 0, 0]), op_a_access: MemoryReadWriteCols { prev_value: Word([0, 0, 0, 0]), access: MemoryAccessCols { value: Word([3, 0, 0, 0]), prev_shard: 0, prev_clk: 0, compare_clk: 0, diff_16bit_limb: 0, diff_8bit_limb: 0 } }, op_b_access: MemoryReadCols { access: MemoryAccessCols { value: Word([0, 0, 0, 0]), prev_shard: 0, prev_clk: 0, compare_clk: 0, diff_16bit_limb: 0, diff_8bit_limb: 0 } }, op_c_access: MemoryReadCols { access: MemoryAccessCols { value: Word([3, 0, 0, 0]), prev_shard: 0, prev_clk: 0, compare_clk: 0, diff_16bit_limb: 0, diff_8bit_limb: 0 } }, is_real: 1, op_a_immutable: 0 }
  row[1]: CpuCols { shard: 0, clk_16bit_limb: 0, clk_8bit_limb: 0, shard_to_send: 0, clk_to_send: 0, pc: 0, next_pc: 0, next_next_pc: 0, instruction: InstructionCols { opcode: 0, op_a: 0, op_b: Word([0, 0, 0, 0]), op_c: Word([0, 0, 0, 0]), op_a_0: 0, imm_b: 1, imm_c: 1 }, num_extra_cycles: 0, is_memory: 0, is_rw_a: 1, is_write_hi: 0, is_halt: 0, is_sequential: 0, op_a_value: Word([0, 0, 0, 0]), hi_or_prev_a: Word([0, 0, 0, 0]), op_a_access: MemoryReadWriteCols { prev_value: Word([0, 0, 0, 0]), access: MemoryAccessCols { value: Word([0, 0, 0, 0]), prev_shard: 0, prev_clk: 0, compare_clk: 0, diff_16bit_limb: 0, diff_8bit_limb: 0 } }, op_b_access: MemoryReadCols { access: MemoryAccessCols { value: Word([0, 0, 0, 0]), prev_shard: 0, prev_clk: 0, compare_clk: 0, diff_16bit_limb: 0, diff_8bit_limb: 0 } }, op_c_access: MemoryReadCols { access: MemoryAccessCols { value: Word([0, 0, 0, 0]), prev_shard: 0, prev_clk: 0, compare_clk: 0, diff_16bit_limb: 0, diff_8bit_limb: 0 } }, is_real: 0, op_a_immutable: 0 }

CpuCols { shard: 0, clk_16bit_limb: 1, clk_8bit_limb: 2, shard_to_send: 3,
clk_to_send: 4, pc: 5, next_pc: 6, next_next_pc: 7,
instruction: InstructionCols { opcode: 8, op_a: 9, op_b: Word([10, 11, 12, 13]),
op_c: Word([14, 15, 16, 17]), op_a_0: 18, imm_b: 19, imm_c: 20 },
num_extra_cycles: 21, is_memory: 22, is_rw_a: 23, is_write_hi: 24, is_halt: 25,
is_sequential: 26,
op_a_value: Word([27, 28, 29, 30]),
hi_or_prev_a: Word([31, 32, 33, 34]),
op_a_access: MemoryReadWriteCols { prev_value: Word([35, 36, 37, 38]), access: MemoryAccessCols { value: Word([39, 40, 41, 42]), prev_shard: 43, prev_clk: 44, compare_clk: 45, diff_16bit_limb: 46, diff_8bit_limb: 47 } },
op_b_access: MemoryReadCols { access: MemoryAccessCols { value: Word([48, 49, 50, 51]), prev_shard: 52, prev_clk: 53, compare_clk: 54, diff_16bit_limb: 55, diff_8bit_limb: 56 } },
op_c_access: MemoryReadCols { access: MemoryAccessCols { value: Word([57, 58, 59, 60]), prev_shard: 61, prev_clk: 62, compare_clk: 63, diff_16bit_limb: 64, diff_8bit_limb: 65 } },
is_real: 66, op_a_immutable: 67 }

(curr[19] * (curr[48] - curr[10])) = 0
(curr[19] * (curr[49] - curr[11])) = 0
(curr[19] * (curr[50] - curr[12])) = 0
(curr[19] * (curr[51] - curr[13])) = 0
(curr[20] * (curr[57] - curr[14])) = 0
(curr[20] * (curr[58] - curr[15])) = 0
(curr[20] * (curr[59] - curr[16])) = 0
(curr[20] * (curr[60] - curr[17])) = 0
((1 - curr[19]) * ((1 - curr[19]) - 1)) = 0
((1 - curr[19]) * (curr[54] * (curr[54] - 1))) = 0
((1 - curr[19]) * (curr[54] * (curr[0] - curr[52]))) = 0
((1 - curr[19]) * (((((curr[54] * (((65536 * curr[2]) + curr[1]) + 2)) + ((1 - curr[54]) * curr[0])) - ((curr[54] * curr[53]) + ((1 - curr[54]) * curr[52]))) - 1) - (curr[55] + (curr[56] * 65536)))) = 0
((1 - curr[20]) * ((1 - curr[20]) - 1)) = 0
((1 - curr[20]) * (curr[63] * (curr[63] - 1))) = 0
((1 - curr[20]) * (curr[63] * (curr[0] - curr[61]))) = 0
((1 - curr[20]) * (((((curr[63] * (((65536 * curr[2]) + curr[1]) + 1)) + ((1 - curr[63]) * curr[0])) - ((curr[63] * curr[62]) + ((1 - curr[63]) * curr[61]))) - 1) - (curr[64] + (curr[65] * 65536)))) = 0
(curr[18] * curr[39]) = 0
(curr[18] * curr[40]) = 0
(curr[18] * curr[41]) = 0
(curr[18] * curr[42]) = 0
((curr[18] - 1) * (curr[27] - curr[39])) = 0
((curr[18] - 1) * (curr[28] - curr[40])) = 0
((curr[18] - 1) * (curr[29] - curr[41])) = 0
((curr[18] - 1) * (curr[30] - curr[42])) = 0
(curr[23] * (curr[31] - curr[35])) = 0
(curr[23] * (curr[32] - curr[36])) = 0
(curr[23] * (curr[33] - curr[37])) = 0
(curr[23] * (curr[34] - curr[38])) = 0
(curr[66] * (curr[66] - 1)) = 0
(curr[66] * (curr[45] * (curr[45] - 1))) = 0
(curr[66] * (curr[45] * (curr[0] - curr[43]))) = 0
(curr[66] * (((((curr[45] * (((65536 * curr[2]) + curr[1]) + 3)) + ((1 - curr[45]) * curr[0])) - ((curr[45] * curr[44]) + ((1 - curr[45]) * curr[43]))) - 1) - (curr[46] + (curr[47] * 65536)))) = 0
(curr[67] * (curr[39] - curr[35])) = 0
(curr[67] * (curr[40] - curr[36])) = 0
(curr[67] * (curr[41] - curr[37])) = 0
(curr[67] * (curr[42] - curr[38])) = 0
(curr[66] * (curr[3] - ((((curr[22] + curr[23]) + curr[24]) * curr[0]) + ((1 - ((curr[22] + curr[23]) + curr[24])) * 0)))) = 0
(curr[66] * (curr[4] - ((((curr[22] + curr[23]) + curr[24]) * ((65536 * curr[2]) + curr[1])) + ((1 - ((curr[22] + curr[23]) + curr[24])) * 0)))) = 0
(IsTransition * (next[66] * (curr[0] - next[0]))) = 0
(IsFirstRow * ((65536 * curr[2]) + curr[1])) = 0
(IsTransition * (next[66] * (((((65536 * curr[2]) + curr[1]) + 5) + curr[21]) - ((65536 * next[2]) + next[1])))) = 0
(curr[66] * (((65536 * curr[2]) + curr[1]) - (curr[1] + (curr[2] * 65536)))) = 0
(curr[66] * (public[44] - curr[0])) = 0
(IsFirstRow * (public[40] - curr[5])) = 0
(IsFirstRow * ((curr[25] - 1) * ((curr[5] + 4) - curr[6]))) = 0
(IsTransition * (next[66] * (curr[6] - next[5]))) = 0
(IsTransition * (next[66] * ((next[25] - 1) * (curr[7] - next[6])))) = 0
(IsTransition * (curr[66] * (curr[26] * (curr[7] - (curr[6] + 4))))) = 0
(IsTransition * ((curr[66] - next[66]) * (public[41] - curr[6]))) = 0
(IsLastRow * (curr[66] * (public[41] - curr[6]))) = 0
(curr[66] * (curr[66] - 1)) = 0
(IsFirstRow * (curr[66] - 1)) = 0
(IsTransition * ((curr[66] - 1) * next[66])) = 0
(IsTransition * (curr[25] * next[66])) = 0
((1 - curr[66]) * (1 - curr[19])) = 0
((1 - curr[66]) * (1 - curr[20])) = 0
((1 - curr[66]) * (1 - curr[23])) = 0
*/
