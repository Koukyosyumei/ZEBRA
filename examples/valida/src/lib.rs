pub mod alu_constraints;
pub mod alu_tables;
pub mod config;
pub mod p3_to_tv;
pub mod state;
pub mod utils;

/*
CpuCols { clk: 0, pc: 1, fp: 2,
instruction: InstructionCols { opcode: 3, operands: Operands([4, 5, 6, 7, 8]) },
opcode_flags: OpcodeFlagCols { is_bus_op: 9, is_pointer_op: 10, is_imm_op: 11, is_left_imm_op: 12, is_load: 13,
is_load_u8: 14, is_load_s8: 15, is_store: 16, is_store_u8: 17, is_beq: 18, is_bne: 19, is_jal: 20, is_jalv: 21,
is_imm32: 22, is_advice: 23, is_stop: 24, is_loadfp: 25, is_write: 26 }, diff: 27, diff_inv: 28, not_equal: 29,
mem_read_channels: [ReadChannelCols { used: 30, addr: 31, value: Word([32, 33, 34, 35]) },
ReadChannelCols { used: 36, addr: 37, value: Word([38, 39, 40, 41]) }],
mem_write_channels: [WriteChannelCols { used: 42, addr: 43, value:
Word([44, 45, 46, 47]), old_value: Word([48, 49, 50, 51]) }],
addr_offset_flags: Word([52, 53, 54, 55]), sign_bit: 56, is_last_segment: 57, is_real: 58 }

(IsFirstRow * (curr[1] - public[0]))
(IsTransition * (next[58] * ((1 - ((((curr[20] + curr[21]) + curr[19]) + curr[18]) + curr[24])) * (next[1] - (curr[1] + 1)))))
(IsTransition * (curr[18] * ((24 * next[1]) - (((1 - curr[29]) * curr[4]) + ((24 * curr[29]) * (curr[1] + 1))))))
(IsTransition * (curr[19] * ((24 * next[1]) - (((24 * (1 - curr[29])) * (curr[1] + 1)) + (curr[29] * curr[4])))))
(IsTransition * (curr[20] * ((24 * next[1]) - curr[5])))
(IsTransition * (curr[21] * ((24 * next[1]) - ((((1 * curr[32]) + (256 * curr[33])) + (65536 * curr[34])) + (16777216 * curr[35])))))
(IsFirstRow * (curr[2] - public[1]))
(IsTransition * (curr[20] * (next[2] - (curr[2] + curr[6]))))
(IsTransition * (curr[21] * (next[2] - ((curr[2] + ((((1 * curr[38]) + (256 * curr[39])) + (65536 * curr[40])) + (16777216 * curr[41]))) - (curr[56] * 268435454)))))
(next[58] * (IsTransition * (((1 - curr[20]) - curr[21]) * (next[2] - curr[2]))))
(IsFirstRow * (curr[57] - public[2]))
(curr[57] * (curr[57] - 1))
(curr[27] - (((((curr[32] - curr[38]) * (curr[32] - curr[38])) + ((curr[33] - curr[39]) * (curr[33] - curr[39]))) + ((curr[34] - curr[40]) * (curr[34] - curr[40]))) + ((curr[35] - curr[41]) * (curr[35] - curr[41]))))
(curr[29] * (curr[29] - 1))
(curr[29] - (curr[27] * curr[28]))
((1 - curr[29]) * curr[27])
(((((curr[21] + curr[18]) + curr[19]) + ((curr[9] + curr[10]) * (1 - curr[12]))) + curr[26]) * (curr[31] - (curr[2] + curr[5])))
(((((curr[13] + curr[16]) + curr[14]) + curr[15]) + curr[17]) * (curr[31] - (curr[2] + curr[6])))
((((((((((curr[13] + curr[16]) + curr[15]) + curr[14]) + curr[17]) + curr[21]) + curr[18]) + curr[19]) + ((1 - curr[12]) * curr[9])) + curr[26]) * (curr[30] - 1))
((((curr[20] + curr[12]) + curr[25]) + curr[22]) * curr[30])
(curr[13] * (curr[37] - ((((1 * curr[32]) + (256 * curr[33])) + (65536 * curr[34])) + (16777216 * curr[35]))))
((curr[15] + curr[14]) * ((curr[37] + ((((curr[52] * 0) + (curr[53] * 1)) + (curr[54] * 2)) + (curr[55] * 3))) - ((((1 * curr[32]) + (256 * curr[33])) + (65536 * curr[34])) + (16777216 * curr[35]))))
((curr[16] + curr[17]) * (curr[37] - (curr[2] + curr[5])))
((curr[21] + ((1 - curr[11]) * curr[9])) * (curr[37] - (curr[2] + curr[6])))
(((((((curr[13] + curr[14]) + curr[15]) + curr[17]) + curr[16]) + curr[21]) + ((1 - curr[11]) * ((curr[18] + curr[19]) + curr[9]))) * (curr[36] - 1))
(((((curr[20] + (curr[11] * ((curr[18] + curr[19]) + curr[9]))) + curr[25]) + curr[22]) + curr[26]) * curr[36])
((((((((curr[13] + curr[14]) + curr[15]) + curr[20]) + curr[21]) + curr[22]) + curr[9]) + curr[25]) * (curr[43] - (curr[2] + curr[4])))
(curr[16] * (curr[43] - ((((1 * curr[38]) + (256 * curr[39])) + (65536 * curr[40])) + (16777216 * curr[41]))))
(curr[17] * ((curr[43] + ((((curr[52] * 0) + (curr[53] * 1)) + (curr[54] * 2)) + (curr[55] * 3))) - ((((1 * curr[38]) + (256 * curr[39])) + (65536 * curr[40])) + (16777216 * curr[41]))))
(curr[16] * (((((curr[32] - curr[44]) * (curr[32] - curr[44])) + ((curr[33] - curr[45]) * (curr[33] - curr[45]))) + ((curr[34] - curr[46]) * (curr[34] - curr[46]))) + ((curr[35] - curr[47]) * (curr[35] - curr[47]))))
(curr[17] * ((1 - curr[52]) * (curr[44] - curr[48])))
(curr[17] * (curr[52] * (curr[32] - curr[44])))
(curr[17] * ((1 - curr[53]) * (curr[45] - curr[49])))
(curr[17] * (curr[53] * (curr[32] - curr[45])))
(curr[17] * ((1 - curr[54]) * (curr[46] - curr[50])))
(curr[17] * (curr[54] * (curr[32] - curr[46])))
(curr[17] * ((1 - curr[55]) * (curr[47] - curr[51])))
(curr[17] * (curr[55] * (curr[32] - curr[47])))
(curr[13] * (((((curr[38] - curr[44]) * (curr[38] - curr[44])) + ((curr[39] - curr[45]) * (curr[39] - curr[45]))) + ((curr[40] - curr[46]) * (curr[40] - curr[46]))) + ((curr[41] - curr[47]) * (curr[41] - curr[47]))))
((curr[14] + curr[15]) * (curr[44] - ((((curr[38] * curr[52]) + (curr[39] * curr[53])) + (curr[40] * curr[54])) + (curr[41] * curr[55]))))
(curr[14] * curr[45])
(curr[15] * (curr[45] - (curr[56] * 255)))
(curr[14] * curr[46])
(curr[15] * (curr[46] - (curr[56] * 255)))
(curr[14] * curr[47])
(curr[15] * (curr[47] - (curr[56] * 255)))
(IsTransition * ((curr[20] + curr[21]) * ((24 * (curr[1] + 1)) - ((((1 * curr[44]) + (256 * curr[45])) + (65536 * curr[46])) + (16777216 * curr[47])))))
(curr[22] * (((((curr[44] - curr[5]) * (curr[44] - curr[5])) + ((curr[45] - curr[6]) * (curr[45] - curr[6]))) + ((curr[46] - curr[7]) * (curr[46] - curr[7]))) + ((curr[47] - curr[8]) * (curr[47] - curr[8]))))
(curr[25] * ((curr[2] + curr[5]) - ((((1 * curr[44]) + (256 * curr[45])) + (65536 * curr[46])) + (16777216 * curr[47]))))
(((((((curr[16] + curr[13]) + curr[20]) + curr[21]) + curr[22]) + curr[25]) + curr[9]) * (curr[42] - 1))
(((curr[18] + curr[19]) + curr[26]) * curr[42])
(curr[58] * (curr[58] - 1))
((curr[58] - 1) * (curr[0] - 0))
(IsTransition * (next[58] * (curr[58] - 1)))
(IsFirstRow * curr[0])
(next[58] * (IsTransition * ((curr[0] + 1) - next[0])))
((curr[11] + curr[12]) * ((curr[11] + curr[12]) - 1))
(curr[11] * (curr[6] - ((((1 * curr[38]) + (256 * curr[39])) + (65536 * curr[40])) + (16777216 * curr[41]))))
(curr[12] * (curr[5] - ((((1 * curr[32]) + (256 * curr[33])) + (65536 * curr[34])) + (16777216 * curr[35]))))
(IsTransition * (curr[24] * (next[1] - 0)))
((curr[57] - 0) * (IsLastRow * (curr[24] - 1)))
*/
