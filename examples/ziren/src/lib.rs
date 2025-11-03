pub mod executor;
pub mod p3_to_tv;
pub mod pv_constraints;
pub mod state;

/*
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
