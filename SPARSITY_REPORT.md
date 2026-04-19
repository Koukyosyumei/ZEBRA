# ZEBRA Constraint Sparsity Report

> Generated: 2026-04-19  
> Branch: `sparcity`

## Overview

Two complementary sparsity views are reported per table:

| Metric | Meaning |
|--------|---------|
| **Column coverage** | Fraction of columns referenced by ≥ 1 constraint |
| **Cols / constraint** | Mean number of columns a single constraint uses |
| **\|N\|** | Neighborhood size — distinct `(entry, row-context, column)` cells across **AIR constraints only** |
| **T\_arith** | Total arithmetic operation nodes in **AIR constraints only** (lookup constraints excluded) |
| **air\_sparsity** | `T_arith / |N|²` — arithmetic density relative to a fully-coupled quadratic AIR (IACR 2018/046, Option B). Lower = sparser. |

Lookup constraints are excluded from `|N|` and `T_arith` because they describe table membership rather than arithmetic structure and would otherwise inflate both quantities independently of the AIR's intrinsic complexity.

## Summary Table

| VM | Table | Cols | Covered | Coverage | \|N\| | T\_arith | air\_sparsity | AIR c | AIR cols/c | Lookup c | Lookup cols/c |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| ziren | AddSub | 19 | 17 | 89.5% | 17 | 76 | 0.263 | 14 | 4.36 | 0 | 0.00 |
| ziren | Bitwise | 18 | 16 | 88.9% | 4 | 16 | 1.000 | 5 | 1.60 | 28 | 6.71 |
| ziren | Mul | 58 | 52 | 89.7% | 52 | 207 | 0.077 | 41 | 4.00 | 2 | 3.00 |
| ziren | DivRem | 106 | 88 | 83.0% | 88 | 1522 | 0.197 | 143 | 5.24 | 17 | 13.41 |
| ziren | Lt | 36 | 30 | 83.3% | 30 | 130 | 0.144 | 32 | 3.59 | 3 | 4.33 |
| ziren | ShiftLeft | 44 | 39 | 88.6% | 39 | 220 | 0.145 | 64 | 2.55 | 0 | 0.00 |
| ziren | ShiftRight | 71 | 66 | 93.0% | 65 | 426 | 0.101 | 83 | 3.75 | 17 | 5.82 |
| ziren | Branch | 62 | 33 | 53.2% | 21 | 200 | 0.454 | 31 | 4.84 | 3 | 14.33 |
| ziren | Jump | 66 | 19 | 28.8% | 19 | 84 | 0.233 | 13 | 4.23 | 1 | 13.00 |
| ziren | MemoryReadWrite | 79 | 62 | 78.5% | 54 | 740 | 0.254 | 102 | 5.90 | 5 | 11.80 |
| ziren | CloClz | 22 | 20 | 90.9% | 20 | 59 | 0.148 | 21 | 2.48 | 2 | 6.00 |
| ziren | MovCond | 32 | 26 | 81.2% | 26 | 152 | 0.225 | 39 | 3.97 | 0 | 0.00 |
| ziren | Cpu | 67 | 66 | 98.5% | 76 | 910 | 0.158 | 84 | 4.88 | 12 | 14.00 |
| sp1 | AddSub | 19 | 18 | 94.7% | 18 | 91 | 0.281 | 18 | 3.72 | 0 | 0.00 |
| sp1 | Bitwise | 17 | 16 | 94.1% | 4 | 16 | 1.000 | 5 | 2.00 | 28 | 6.71 |
| sp1 | Mul | 39 | 38 | 97.4% | 38 | 191 | 0.132 | 31 | 4.84 | 2 | 3.00 |
| sp1 | DivRem | 98 | 85 | 86.7% | 85 | 1158 | 0.160 | 149 | 4.25 | 18 | 13.11 |
| sp1 | Lt | 33 | 28 | 84.8% | 28 | 131 | 0.167 | 29 | 3.90 | 3 | 4.33 |
| sp1 | ShiftLeft | 44 | 40 | 90.9% | 40 | 238 | 0.149 | 65 | 2.78 | 0 | 0.00 |
| sp1 | ShiftRight | 70 | 66 | 94.3% | 65 | 395 | 0.093 | 82 | 3.44 | 17 | 5.82 |
| sp1 | Branch | 34 | 31 | 91.2% | 27 | 1956 | 2.683 | 78 | 8.26 | 23 | 14.91 |
| sp1 | Jump | 26 | 23 | 88.5% | 15 | 71 | 0.316 | 9 | 3.89 | 2 | 13.00 |
| sp1 | MemoryReadWrite | 57 | 54 | 94.7% | 46 | 423 | 0.200 | 76 | 4.53 | 5 | 9.40 |
| sp1 | Cpu | 57 | 56 | 98.2% | 64 | 845 | 0.206 | 71 | 5.21 | 11 | 14.00 |
| sphinx | AddSub | 20 | 18 | 90.0% | 19 | 108 | 0.299 | 19 | 4.21 | 0 | 0.00 |
| sphinx | Bitwise | 18 | 16 | 88.9% | 5 | 16 | 0.640 | 6 | 1.33 | 28 | 5.71 |
| sphinx | Mul | 40 | 38 | 95.0% | 39 | 185 | 0.122 | 32 | 4.44 | 2 | 3.00 |
| sphinx | DivRem | 104 | 85 | 81.7% | 86 | 416 | 0.056 | 116 | 3.09 | 18 | 13.11 |
| sphinx | Lt | 37 | 31 | 83.8% | 32 | 134 | 0.131 | 34 | 3.44 | 3 | 4.33 |
| sphinx | ShiftLeft | 45 | 40 | 88.9% | 41 | 224 | 0.133 | 66 | 2.50 | 0 | 0.00 |
| sphinx | ShiftRight | 71 | 66 | 93.0% | 66 | 393 | 0.090 | 83 | 3.35 | 17 | 5.82 |
| sphinx | Cpu | 151 | 145 | 96.0% | 203 | 1596 | 0.039 | 259 | 5.22 | 39 | 14.54 |
| openvm | BaseAlu | 25 | 17 | 68.0% | 0 | 0 | 0.000 | 0 | 0.00 | 5 | 13.00 |
| openvm | BitwiseAlu | 25 | 17 | 68.0% | 0 | 0 | 0.000 | 0 | 0.00 | 5 | 13.00 |
| openvm | Mul | 21 | 13 | 61.9% | 0 | 0 | 0.000 | 0 | 0.00 | 1 | 13.00 |
| openvm | Lt | 26 | 11 | 42.3% | 0 | 0 | 0.000 | 0 | 0.00 | 2 | 10.00 |
| openvm | Shift | 42 | 13 | 31.0% | 0 | 0 | 0.000 | 0 | 0.00 | 1 | 13.00 |
| openvm | BranchEqual | 24 | 11 | 45.8% | 0 | 0 | 0.000 | 0 | 0.00 | 2 | 10.00 |
| openvm | BranchLessThan | 30 | 13 | 43.3% | 0 | 0 | 0.000 | 0 | 0.00 | 4 | 10.00 |
| openvm | Jal | 15 | 8 | 53.3% | 8 | 20 | 0.313 | 2 | 6.00 | 0 | 0.00 |
| openvm | Lui | 15 | 8 | 53.3% | 8 | 20 | 0.313 | 2 | 6.00 | 0 | 0.00 |
| openvm | Jalr | 21 | 10 | 47.6% | 10 | 22 | 0.220 | 4 | 3.25 | 0 | 0.00 |
| pico | AddSub | 17 | 17 | 100.0% | 17 | 104 | 0.360 | 17 | 4.59 | 0 | 0.00 |
| pico | Bitwise | 15 | 15 | 100.0% | 3 | 12 | 1.333 | 4 | 1.50 | 28 | 5.71 |
| pico | Mul | 37 | 37 | 100.0% | 37 | 181 | 0.132 | 30 | 4.67 | 2 | 2.00 |
| pico | DivRem | 96 | 96 | 100.0% | 96 | 450 | 0.049 | 131 | 2.98 | 20 | 13.35 |
| pico | Lt | 30 | 30 | 100.0% | 30 | 130 | 0.144 | 32 | 3.59 | 3 | 2.33 |
| pico | ShiftLeft | 42 | 39 | 92.9% | 39 | 220 | 0.145 | 64 | 2.55 | 0 | 0.00 |
| pico | ShiftRight | 68 | 64 | 94.1% | 64 | 389 | 0.095 | 81 | 3.41 | 1 | 2.00 |
| pico | MemoryReadWrite | 95 | 75 | 78.9% | 67 | 824 | 0.184 | 92 | 7.85 | 2 | 15.50 |
| pico | Cpu | 117 | 117 | 100.0% | 160 | 1459 | 0.057 | 261 | 4.80 | 43 | 14.51 |
| valida | Add32 | 16 | 15 | 93.8% | 15 | 67 | 0.298 | 10 | 3.20 | 0 | 0.00 |
| valida | Sub32 | 17 | 16 | 94.1% | 16 | 27 | 0.105 | 8 | 2.88 | 0 | 0.00 |
| valida | Bitwise32 | 79 | 79 | 100.0% | 79 | 828 | 0.133 | 88 | 4.07 | 0 | 0.00 |
| valida | Mul32 | 27 | 24 | 88.9% | 24 | 161 | 0.280 | 7 | 7.14 | 2 | 3.00 |
| valida | Div32 | 30 | 30 | 100.0% | 14 | 67 | 0.342 | 20 | 2.95 | 39 | 11.46 |
| valida | Lt32 | 45 | 44 | 97.8% | 44 | 313 | 0.162 | 60 | 3.42 | 0 | 0.00 |
| valida | Com32 | 14 | 14 | 100.0% | 14 | 35 | 0.179 | 8 | 2.88 | 0 | 0.00 |
| valida | Memory | 27 | 27 | 100.0% | 38 | 139 | 0.096 | 42 | 2.67 | 0 | 0.00 |
| valida | Cpu | 59 | 58 | 98.3% | 64 | 399 | 0.097 | 61 | 5.02 | 13 | 12.31 |

---

## ziren (KoalaBear · MIPS/RISC-V)

| Table | Cols | Covered | Coverage | \|N\| | T\_arith | air\_sparsity | AIR cols/c | Lookup cols/c | Top-5 hot columns |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| AddSub | 19 | 17 | 89.5% | 17 | 76 | 0.263 | 4.36 | 0.00 | `[17]=13  [18]=13  [6]=5  [7]=5  [8]=4` |
| Bitwise | 18 | 16 | 88.9% | 4 | 16 | 1.000 | 1.60 | 6.71 | `[15]=31  [16]=30  [17]=30  [14]=29  [6]=7` |
| Mul | 58 | 52 | 89.7% | 52 | 207 | 0.077 | 4.00 | 3.00 | `[55]=12  [39]=9  [10]=8  [14]=8  [11]=7` |
| DivRem | 106 | 88 | 83.0% | 88 | 1522 | 0.197 | 5.24 | 13.41 | `[57]=108  [58]=104  [59]=100  [60]=96  [10]=28` |
| Lt | 36 | 30 | 83.3% | 30 | 130 | 0.144 | 3.59 | 4.33 | `[2]=13  [19]=13  [3]=11  [18]=11  [17]=9` |
| ShiftLeft | 44 | 39 | 88.6% | 39 | 220 | 0.145 | 2.55 | 0.00 | `[30]=12  [14]=10  [15]=10  [16]=10  [39]=7` |
| ShiftRight | 71 | 66 | 93.0% | 65 | 426 | 0.101 | 3.75 | 5.82 | `[59]=26  [60]=26  [61]=26  [70]=20  [68]=19` |
| Branch | 62 | 33 | 53.2% | 21 | 200 | 0.454 | 4.84 | 14.33 | `[59]=21  [53]=17  [54]=17  [55]=17  [56]=17` |
| Jump | 66 | 19 | 28.8% | 19 | 84 | 0.233 | 4.23 | 13.00 | `[49]=10  [50]=10  [51]=8  [1]=3  [2]=3` |
| MemoryReadWrite | 79 | 62 | 78.5% | 54 | 740 | 0.254 | 5.90 | 11.80 | `[38]=33  [18]=31  [36]=31  [37]=31  [16]=30` |
| CloClz | 22 | 20 | 90.9% | 20 | 59 | 0.148 | 2.48 | 6.00 | `[14]=8  [19]=6  [20]=6  [13]=5  [10]=4` |
| MovCond | 32 | 26 | 81.2% | 26 | 152 | 0.225 | 3.97 | 0.00 | `[29]=25  [30]=25  [31]=21  [28]=18  [2]=5` |
| Cpu | 67 | 66 | 98.5% | 76 | 910 | 0.158 | 4.88 | 14.00 | `[65]=61  [8]=39  [6]=31  [7]=29  [47]=28` |

---

## sp1 (BabyBear · RISC-V)

| Table | Cols | Covered | Coverage | \|N\| | T\_arith | air\_sparsity | AIR cols/c | Lookup cols/c | Top-5 hot columns |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| AddSub | 19 | 18 | 94.7% | 18 | 91 | 0.281 | 3.72 | 0.00 | `[16]=15  [5]=6  [6]=6  [7]=4  [1]=3` |
| Bitwise | 17 | 16 | 94.1% | 4 | 16 | 1.000 | 2.00 | 6.71 | `[14]=31  [15]=31  [16]=31  [13]=29  [5]=7` |
| Mul | 39 | 38 | 97.4% | 38 | 191 | 0.132 | 4.84 | 3.00 | `[13]=9  [5]=8  [9]=8  [35]=8  [6]=7` |
| DivRem | 98 | 85 | 86.7% | 85 | 1158 | 0.160 | 4.25 | 13.11 | `[96]=93  [61]=50  [62]=46  [63]=46  [64]=42` |
| Lt | 33 | 28 | 84.8% | 28 | 131 | 0.167 | 3.90 | 4.33 | `[1]=13  [16]=13  [2]=11  [15]=11  [14]=9` |
| ShiftLeft | 44 | 40 | 90.9% | 40 | 238 | 0.149 | 2.78 | 0.00 | `[13]=17  [30]=12  [14]=10  [15]=10  [16]=10` |
| ShiftRight | 70 | 66 | 94.3% | 65 | 395 | 0.093 | 3.44 | 5.82 | `[59]=26  [60]=26  [61]=26  [69]=21  [68]=19` |
| Branch | 34 | 31 | 91.2% | 27 | 1956 | 2.683 | 8.26 | 14.91 | `[23]=88  [24]=88  [25]=88  [26]=88  [27]=88` |
| Jump | 26 | 23 | 88.5% | 15 | 71 | 0.316 | 3.89 | 13.00 | `[23]=8  [24]=8  [0]=3  [1]=3  [2]=3` |
| MemoryReadWrite | 57 | 54 | 94.7% | 46 | 423 | 0.200 | 4.53 | 9.40 | `[18]=28  [16]=27  [19]=24  [20]=24  [17]=23` |
| Cpu | 57 | 56 | 98.2% | 64 | 845 | 0.206 | 5.21 | 14.00 | `[56]=52  [7]=36  [41]=29  [6]=28  [40]=28` |

---

## sphinx (BabyBear · RISC-V fork)

| Table | Cols | Covered | Coverage | \|N\| | T\_arith | air\_sparsity | AIR cols/c | Lookup cols/c | Top-5 hot columns |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| AddSub | 20 | 18 | 90.0% | 19 | 108 | 0.299 | 4.21 | 0.00 | `[18]=16  [19]=16  [7]=6  [8]=6  [9]=4` |
| Bitwise | 18 | 16 | 88.9% | 5 | 16 | 0.640 | 1.33 | 5.71 | `[15]=30  [16]=30  [17]=30  [7]=7  [8]=7` |
| Mul | 40 | 38 | 95.0% | 39 | 185 | 0.122 | 4.44 | 3.00 | `[7]=8  [11]=8  [36]=8  [8]=7  [12]=7` |
| DivRem | 104 | 85 | 81.7% | 86 | 416 | 0.056 | 3.09 | 13.11 | `[102]=61  [62]=25  [63]=21  [64]=21  [93]=21` |
| Lt | 37 | 31 | 83.8% | 32 | 134 | 0.131 | 3.44 | 4.33 | `[3]=13  [20]=13  [4]=11  [19]=11  [18]=9` |
| ShiftLeft | 45 | 40 | 88.9% | 41 | 224 | 0.133 | 2.50 | 0.00 | `[31]=12  [15]=10  [16]=10  [17]=10  [40]=7` |
| ShiftRight | 71 | 66 | 93.0% | 66 | 393 | 0.090 | 3.35 | 5.82 | `[60]=26  [61]=26  [62]=26  [70]=20  [69]=19` |
| Cpu | 151 | 145 | 96.0% | 203 | 1596 | 0.039 | 5.22 | 14.54 | `[140]=63  [41]=56  [42]=55  [44]=55  [43]=52` |

---

## openvm (BabyBear · RV32IM)

| Table | Cols | Covered | Coverage | \|N\| | T\_arith | air\_sparsity | AIR cols/c | Lookup cols/c | Top-5 hot columns |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| BaseAlu | 25 | 17 | 68.0% | 0 | 0 | 0.000 | 0.00 | 13.00 | `[8]=5  [9]=5  [10]=5  [11]=5  [12]=5` |
| BitwiseAlu | 25 | 17 | 68.0% | 0 | 0 | 0.000 | 0.00 | 13.00 | `[8]=5  [9]=5  [10]=5  [11]=5  [12]=5` |
| Mul | 21 | 13 | 61.9% | 0 | 0 | 0.000 | 0.00 | 13.00 | `[8]=1  [9]=1  [10]=1  [11]=1  [12]=1` |
| Lt | 26 | 11 | 42.3% | 0 | 0 | 0.000 | 0.00 | 10.00 | `[8]=2  [9]=2  [10]=2  [11]=2  [12]=2` |
| Shift | 42 | 13 | 31.0% | 0 | 0 | 0.000 | 0.00 | 13.00 | `[8]=1  [9]=1  [10]=1  [11]=1  [12]=1` |
| BranchEqual | 24 | 11 | 45.8% | 0 | 0 | 0.000 | 0.00 | 10.00 | `[8]=2  [9]=2  [10]=2  [11]=2  [12]=2` |
| BranchLessThan | 30 | 13 | 43.3% | 0 | 0 | 0.000 | 0.00 | 10.00 | `[8]=4  [9]=4  [10]=4  [11]=4  [12]=4` |
| Jal | 15 | 8 | 53.3% | 8 | 20 | 0.313 | 6.00 | 0.00 | `[9]=2  [10]=2  [11]=2  [12]=2  [0]=1` |
| Lui | 15 | 8 | 53.3% | 8 | 20 | 0.313 | 6.00 | 0.00 | `[9]=2  [10]=2  [11]=2  [12]=2  [0]=1` |
| Jalr | 21 | 10 | 47.6% | 10 | 22 | 0.220 | 3.25 | 0.00 | `[16]=2  [17]=2  [20]=2  [8]=1  [9]=1` |

---

## pico (KoalaBear · RISC-V)

| Table | Cols | Covered | Coverage | \|N\| | T\_arith | air\_sparsity | AIR cols/c | Lookup cols/c | Top-5 hot columns |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| AddSub | 17 | 17 | 100.0% | 17 | 104 | 0.360 | 4.59 | 0.00 | `[15]=16  [16]=16  [4]=6  [5]=6  [6]=4` |
| Bitwise | 15 | 15 | 100.0% | 3 | 12 | 1.333 | 1.50 | 5.71 | `[12]=30  [13]=30  [14]=30  [4]=7  [5]=7` |
| Mul | 37 | 37 | 100.0% | 37 | 181 | 0.132 | 4.67 | 2.00 | `[4]=8  [8]=8  [33]=8  [5]=7  [9]=7` |
| DivRem | 96 | 96 | 100.0% | 96 | 450 | 0.049 | 2.98 | 13.35 | `[94]=77  [59]=23  [61]=23  [11]=21  [90]=21` |
| Lt | 30 | 30 | 100.0% | 30 | 130 | 0.144 | 3.59 | 2.33 | `[17]=13  [16]=11  [0]=10  [15]=9  [1]=8` |
| ShiftLeft | 42 | 39 | 92.9% | 39 | 220 | 0.145 | 2.55 | 0.00 | `[28]=12  [12]=10  [13]=10  [14]=10  [37]=7` |
| ShiftRight | 68 | 64 | 94.1% | 64 | 389 | 0.095 | 3.41 | 2.00 | `[66]=19  [56]=18  [20]=11  [12]=10  [13]=10` |
| MemoryReadWrite | 95 | 75 | 78.9% | 67 | 824 | 0.184 | 7.85 | 15.50 | `[58]=65  [56]=64  [59]=62  [60]=62  [57]=61` |
| Cpu | 117 | 117 | 100.0% | 160 | 1459 | 0.057 | 4.80 | 14.51 | `[32]=73  [33]=73  [34]=73  [35]=73  [36]=73` |

---

## valida (BabyBear · Valida ISA)

| Table | Cols | Covered | Coverage | \|N\| | T\_arith | air\_sparsity | AIR cols/c | Lookup cols/c | Top-5 hot columns |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| Add32 | 16 | 15 | 93.8% | 15 | 67 | 0.298 | 3.20 | 0.00 | `[12]=4  [13]=4  [14]=3  [0]=2  [1]=2` |
| Sub32 | 17 | 16 | 94.1% | 16 | 27 | 0.105 | 2.88 | 0.00 | `[8]=3  [9]=3  [10]=3  [11]=2  [0]=1` |
| Bitwise32 | 79 | 79 | 100.0% | 79 | 828 | 0.133 | 4.07 | 0.00 | `[76]=6  [77]=6  [78]=6  [0]=5  [1]=5` |
| Mul32 | 27 | 24 | 88.9% | 24 | 161 | 0.280 | 7.14 | 3.00 | `[0]=4  [1]=4  [3]=4  [4]=4  [5]=4` |
| Div32 | 30 | 30 | 100.0% | 14 | 67 | 0.342 | 2.95 | 11.46 | `[29]=51  [28]=42  [7]=35  [4]=34  [5]=34` |
| Lt32 | 45 | 44 | 97.8% | 44 | 313 | 0.162 | 3.42 | 0.00 | `[8]=14  [9]=12  [10]=11  [11]=10  [20]=10` |
| Com32 | 14 | 14 | 100.0% | 14 | 35 | 0.179 | 2.88 | 0.00 | `[10]=4  [8]=3  [12]=3  [13]=3  [0]=1` |
| Memory | 27 | 27 | 100.0% | 38 | 139 | 0.096 | 2.67 | 0.00 | `[19]=14  [14]=13  [15]=11  [24]=11  [16]=9` |
| Cpu | 59 | 58 | 98.3% | 64 | 399 | 0.097 | 5.02 | 12.31 | `[32]=21  [44]=20  [45]=20  [46]=20  [47]=20` |

---

## Key Observations

- **59 tables** measured across 6 zkVM backends (including CPU tables for ziren, sp1, sphinx, pico, valida).
- **air\_sparsity** (AIR-only) clusters tightly in `[0.04, 0.36]` for arithmetic and CPU tables, confirming that real AIR constraint systems are highly sparse relative to a fully-coupled quadratic baseline.
- **CPU tables** are consistently sparse (`0.039–0.206`) despite having the most columns (57–151) and constraints (61–261), reflecting that each constraint touches a small fraction of the full neighborhood.
- **sphinx Cpu** is the sparsest CPU table (0.039) with the largest neighborhood (`|N|=203`), while **sp1 Cpu** is the densest (0.206).
- **Bitwise tables** (ziren/sp1/pico/sphinx) correctly show small AIR neighborhoods (`|N| = 3–5`) and `air_sparsity ≈ 0.64–1.33`, reflecting that their tiny AIR portion is nearly fully dense; the richness lives in the lookup side.
- **valida Div32** dropped from `1.649 → 0.342` after excluding its 39 lookup constraints — the AIR portion alone is sparse.
- **sp1 Branch** remains the densest AIR table (`2.683`), driven by its cross-product selector structure generating ~1956 arithmetic nodes over only 27 AIR-neighborhood cells.
- **Sparsest by air\_sparsity**: sphinx Cpu (0.039), pico DivRem (0.049), sphinx DivRem (0.056), pico Cpu (0.057).
- **openvm pure-lookup tables** correctly report `air_sparsity = 0.000` (zero AIR constraints, zero T\_arith).

### air\_sparsity ranking (AIR-only, excluding pure-lookup tables)

| VM | Table | \|N\| | T\_arith | air\_sparsity |
|---|---|---:|---:|---:|
| sphinx | Cpu | 203 | 1596 | 0.039 |
| pico | DivRem | 96 | 450 | 0.049 |
| sphinx | DivRem | 86 | 416 | 0.056 |
| pico | Cpu | 160 | 1459 | 0.057 |
| ziren | Mul | 52 | 207 | 0.077 |
| sphinx | ShiftRight | 66 | 393 | 0.090 |
| sp1 | ShiftRight | 65 | 395 | 0.093 |
| pico | ShiftRight | 64 | 389 | 0.095 |
| valida | Memory | 38 | 139 | 0.096 |
| ziren | ShiftRight | 65 | 426 | 0.101 |
| valida | Sub32 | 16 | 27 | 0.105 |
| sphinx | Mul | 39 | 185 | 0.122 |
| pico | Mul | 37 | 181 | 0.132 |
| sp1 | Mul | 38 | 191 | 0.132 |
| valida | Bitwise32 | 79 | 828 | 0.133 |
| sphinx | ShiftLeft | 41 | 224 | 0.133 |
| ziren | ShiftLeft | 39 | 220 | 0.145 |
| pico | ShiftLeft | 39 | 220 | 0.145 |
| ziren | Lt | 30 | 130 | 0.144 |
| pico | Lt | 30 | 130 | 0.144 |
| ziren | CloClz | 20 | 59 | 0.148 |
| sp1 | ShiftLeft | 40 | 238 | 0.149 |
| sp1 | DivRem | 85 | 1158 | 0.160 |
| valida | Lt32 | 44 | 313 | 0.162 |
| sp1 | Lt | 28 | 131 | 0.167 |
| valida | Com32 | 14 | 35 | 0.179 |
| pico | MemoryReadWrite | 67 | 824 | 0.184 |
| ziren | DivRem | 88 | 1522 | 0.197 |
| sp1 | MemoryReadWrite | 46 | 423 | 0.200 |
| openvm | Jalr | 10 | 22 | 0.220 |
| ziren | MovCond | 26 | 152 | 0.225 |
| ziren | Jump | 19 | 84 | 0.233 |
| ziren | MemoryReadWrite | 54 | 740 | 0.254 |
| ziren | AddSub | 17 | 76 | 0.263 |
| sp1 | Jump | 15 | 71 | 0.316 |
| valida | Mul32 | 24 | 161 | 0.280 |
| sp1 | AddSub | 18 | 91 | 0.281 |
| valida | Add32 | 15 | 67 | 0.298 |
| sphinx | AddSub | 19 | 108 | 0.299 |
| openvm | Jal | 8 | 20 | 0.313 |
| openvm | Lui | 8 | 20 | 0.313 |
| valida | Div32 | 14 | 67 | 0.342 |
| pico | AddSub | 17 | 104 | 0.360 |
| ziren | Branch | 21 | 200 | 0.454 |
| sphinx | Bitwise | 5 | 16 | 0.640 |
| ziren | Bitwise | 4 | 16 | 1.000 |
| sp1 | Bitwise | 4 | 16 | 1.000 |
| pico | Bitwise | 3 | 12 | 1.333 |
| sp1 | Branch | 27 | 1956 | 2.683 |
