# ZEBRA Constraint Sparsity Report

> Generated: 2026-04-19 (s×|N| metric; |N| from all constraints; T\_arith with CSE)  
> Branch: `sparcity`  
> VMs covered: ziren, sp1, sphinx, pico, valida (openvm excluded — pure-lookup architecture)

## Overview

Two complementary sparsity views are reported per table:

| Metric | Meaning |
|--------|---------|
| **Column coverage** | Fraction of columns referenced by ≥ 1 constraint |
| **Cols / constraint** | Mean number of columns a single constraint uses |
| **\|N\|** | Neighborhood size — distinct `(entry, row-context, column)` cells across **all** constraints (AIR + lookup + PV + blocking), reflecting the true table width |
| **s** | Number of AIR+PV+blocking constraints (the ones contributing to T\_arith) |
| **T\_arith** | Total arithmetic operation nodes across AIR+PV+blocking constraints, counted with **CSE**: subexpressions shared across multiple constraints are counted only once |
| **air\_sparsity** | `T_arith / (s × \|N\|)` — ops per variable slot relative to a fully-dense linear baseline (IACR 2018/046). Lower = sparser. |

`|N|` is gathered from **all** constraints so that lookup-only columns (e.g. the input/output limbs of a Bitwise table) are counted in the neighborhood — otherwise tables with heavy lookup arguments would show an artificially small `|N|`. `T_arith` and `s` remain AIR-only because lookup constraints describe table membership, not arithmetic structure.

**CSE (Common Subexpression Elimination)**: when multiple constraints share an identical subexpression (e.g. `(x+y)` in both `P₁=(x+y)×z` and `P₂=(x+y)×w`), the shared node is counted once rather than per-occurrence. This gives a tighter T\_arith that reflects the real gate count after compiler optimization.

## Summary Table

| VM | Table | Cols | Covered | Coverage | \|N\| | s | T\_arith | air\_sparsity | AIR c | AIR cols/c | Lookup c | Lookup cols/c |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| ziren | AddSub | 19 | 17 | 89.5% | 17 | 14 | 46 | 0.193 | 14 | 4.36 | 0 | 0.00 |
| ziren | Bitwise | 18 | 16 | 88.9% | 16 | 5 | 11 | 0.138 | 5 | 1.60 | 28 | 6.71 |
| ziren | Mul | 58 | 52 | 89.7% | 52 | 41 | 160 | 0.075 | 41 | 4.00 | 2 | 3.00 |
| ziren | DivRem | 106 | 88 | 83.0% | 88 | 143 | 483 | 0.038 | 143 | 5.24 | 17 | 13.41 |
| ziren | Lt | 36 | 30 | 83.3% | 30 | 32 | 96 | 0.100 | 32 | 3.59 | 3 | 4.33 |
| ziren | ShiftLeft | 44 | 39 | 88.6% | 39 | 64 | 169 | 0.068 | 64 | 2.55 | 0 | 0.00 |
| ziren | ShiftRight | 71 | 66 | 93.0% | 66 | 83 | 220 | 0.040 | 83 | 3.75 | 17 | 5.82 |
| ziren | Branch | 62 | 33 | 53.2% | 33 | 31 | 119 | 0.116 | 31 | 4.84 | 3 | 14.33 |
| ziren | Jump | 66 | 19 | 28.8% | 19 | 13 | 70 | 0.283 | 13 | 4.23 | 1 | 13.00 |
| ziren | MemoryReadWrite | 79 | 62 | 78.5% | 62 | 102 | 383 | 0.061 | 102 | 5.90 | 5 | 11.80 |
| ziren | CloClz | 22 | 20 | 90.9% | 20 | 21 | 57 | 0.136 | 21 | 2.48 | 2 | 6.00 |
| ziren | MovCond | 32 | 26 | 81.2% | 26 | 39 | 100 | 0.099 | 39 | 3.97 | 0 | 0.00 |
| ziren | Cpu | 67 | 66 | 98.5% | 76 | 84 | 316 | 0.048 | 84 | 4.88 | 12 | 14.00 |
| sp1 | AddSub | 19 | 18 | 94.7% | 18 | 18 | 91 | 0.281 | 18 | 3.72 | 0 | 0.00 |
| sp1 | Bitwise | 17 | 16 | 94.1% | 16 | 5 | 16 | 0.200 | 5 | 2.00 | 28 | 6.71 |
| sp1 | Mul | 39 | 38 | 97.4% | 38 | 31 | 191 | 0.162 | 31 | 4.84 | 2 | 3.00 |
| sp1 | DivRem | 98 | 85 | 86.7% | 85 | 149 | 1158 | 0.091 | 149 | 4.25 | 18 | 13.11 |
| sp1 | Lt | 33 | 28 | 84.8% | 28 | 29 | 131 | 0.161 | 29 | 3.90 | 3 | 4.33 |
| sp1 | ShiftLeft | 44 | 40 | 90.9% | 40 | 65 | 238 | 0.092 | 65 | 2.78 | 0 | 0.00 |
| sp1 | ShiftRight | 70 | 66 | 94.3% | 66 | 82 | 395 | 0.073 | 82 | 3.44 | 17 | 5.82 |
| sp1 | Branch | 34 | 31 | 91.2% | 31 | 78 | 1956 | 0.809 | 78 | 8.26 | 23 | 14.91 |
| sp1 | Jump | 26 | 23 | 88.5% | 23 | 9 | 71 | 0.343 | 9 | 3.89 | 2 | 13.00 |
| sp1 | MemoryReadWrite | 57 | 54 | 94.7% | 54 | 76 | 423 | 0.103 | 76 | 4.53 | 5 | 9.40 |
| sp1 | Cpu | 57 | 56 | 98.2% | 64 | 71 | 845 | 0.181 | 71 | 5.21 | 11 | 14.00 |
| sphinx | AddSub | 20 | 18 | 90.0% | 19 | 19 | 56 | 0.155 | 19 | 4.21 | 0 | 0.00 |
| sphinx | Bitwise | 18 | 16 | 88.9% | 17 | 6 | 14 | 0.137 | 6 | 1.33 | 28 | 5.71 |
| sphinx | Mul | 40 | 38 | 95.0% | 39 | 32 | 141 | 0.113 | 32 | 4.44 | 2 | 3.00 |
| sphinx | DivRem | 104 | 85 | 81.7% | 86 | 116 | 329 | 0.033 | 116 | 3.09 | 18 | 13.11 |
| sphinx | Lt | 37 | 31 | 83.8% | 32 | 34 | 100 | 0.092 | 34 | 3.44 | 3 | 4.33 |
| sphinx | ShiftLeft | 45 | 40 | 88.9% | 41 | 66 | 173 | 0.064 | 66 | 2.50 | 0 | 0.00 |
| sphinx | ShiftRight | 71 | 66 | 93.0% | 67 | 83 | 204 | 0.037 | 83 | 3.35 | 17 | 5.82 |
| sphinx | Cpu | 151 | 145 | 96.0% | 204 | 259 | 919 | 0.017 | 259 | 5.22 | 39 | 14.54 |
| pico | AddSub | 17 | 17 | 100.0% | 17 | 17 | 52 | 0.180 | 17 | 4.59 | 0 | 0.00 |
| pico | Bitwise | 15 | 15 | 100.0% | 15 | 4 | 10 | 0.167 | 4 | 1.50 | 28 | 5.71 |
| pico | Mul | 37 | 37 | 100.0% | 37 | 30 | 137 | 0.123 | 30 | 4.67 | 2 | 2.00 |
| pico | DivRem | 96 | 96 | 100.0% | 96 | 131 | 357 | 0.028 | 131 | 2.98 | 20 | 13.35 |
| pico | Lt | 30 | 30 | 100.0% | 30 | 32 | 96 | 0.100 | 32 | 3.59 | 3 | 2.33 |
| pico | ShiftLeft | 42 | 39 | 92.9% | 39 | 64 | 169 | 0.068 | 64 | 2.55 | 0 | 0.00 |
| pico | ShiftRight | 68 | 64 | 94.1% | 64 | 81 | 200 | 0.039 | 81 | 3.41 | 1 | 2.00 |
| pico | MemoryReadWrite | 95 | 75 | 78.9% | 75 | 92 | 332 | 0.048 | 92 | 7.85 | 2 | 15.50 |
| pico | Cpu | 117 | 117 | 100.0% | 161 | 261 | 783 | 0.019 | 261 | 4.80 | 43 | 14.51 |
| valida | Add32 | 16 | 15 | 93.8% | 15 | 10 | 34 | 0.227 | 10 | 3.20 | 0 | 0.00 |
| valida | Sub32 | 17 | 16 | 94.1% | 16 | 8 | 27 | 0.211 | 8 | 2.88 | 0 | 0.00 |
| valida | Bitwise32 | 79 | 79 | 100.0% | 79 | 88 | 398 | 0.057 | 88 | 4.07 | 0 | 0.00 |
| valida | Mul32 | 27 | 24 | 88.9% | 25 | 7 | 109 | 0.623 | 7 | 7.14 | 2 | 3.00 |
| valida | Div32 | 30 | 30 | 100.0% | 30 | 20 | 51 | 0.085 | 20 | 2.95 | 39 | 11.46 |
| valida | Lt32 | 45 | 44 | 97.8% | 44 | 60 | 196 | 0.074 | 60 | 3.42 | 0 | 0.00 |
| valida | Com32 | 14 | 14 | 100.0% | 14 | 8 | 29 | 0.259 | 8 | 2.88 | 0 | 0.00 |
| valida | Memory | 27 | 27 | 100.0% | 38 | 42 | 109 | 0.068 | 42 | 2.67 | 0 | 0.00 |
| valida | Cpu | 59 | 58 | 98.3% | 65 | 61 | 300 | 0.076 | 61 | 5.02 | 13 | 12.31 |
| **all** | **average** | | | | | | | **0.140** | | | | |

---

## ziren (KoalaBear · MIPS/RISC-V)

| Table | Cols | Covered | Coverage | \|N\| | s | T\_arith | air\_sparsity | AIR cols/c | Lookup cols/c | Top-5 hot columns |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| AddSub | 19 | 17 | 89.5% | 17 | 14 | 46 | 0.193 | 4.36 | 0.00 | `[17]=13  [18]=13  [6]=5  [7]=5  [8]=4` |
| Bitwise | 18 | 16 | 88.9% | 16 | 5 | 11 | 0.138 | 1.60 | 6.71 | `[15]=31  [16]=30  [17]=30  [14]=29  [6]=7` |
| Mul | 58 | 52 | 89.7% | 52 | 41 | 160 | 0.075 | 4.00 | 3.00 | `[55]=12  [39]=9  [10]=8  [14]=8  [11]=7` |
| DivRem | 106 | 88 | 83.0% | 88 | 143 | 483 | 0.038 | 5.24 | 13.41 | `[57]=108  [58]=104  [59]=100  [60]=96  [10]=28` |
| Lt | 36 | 30 | 83.3% | 30 | 32 | 96 | 0.100 | 3.59 | 4.33 | `[2]=13  [19]=13  [3]=11  [18]=11  [17]=9` |
| ShiftLeft | 44 | 39 | 88.6% | 39 | 64 | 169 | 0.068 | 2.55 | 0.00 | `[30]=12  [14]=10  [15]=10  [16]=10  [39]=7` |
| ShiftRight | 71 | 66 | 93.0% | 66 | 83 | 220 | 0.040 | 3.75 | 5.82 | `[59]=26  [60]=26  [61]=26  [70]=20  [68]=19` |
| Branch | 62 | 33 | 53.2% | 33 | 31 | 119 | 0.116 | 4.84 | 14.33 | `[59]=21  [53]=17  [54]=17  [55]=17  [56]=17` |
| Jump | 66 | 19 | 28.8% | 19 | 13 | 70 | 0.283 | 4.23 | 13.00 | `[49]=10  [50]=10  [51]=8  [1]=3  [2]=3` |
| MemoryReadWrite | 79 | 62 | 78.5% | 62 | 102 | 383 | 0.061 | 5.90 | 11.80 | `[38]=33  [18]=31  [36]=31  [37]=31  [16]=30` |
| CloClz | 22 | 20 | 90.9% | 20 | 21 | 57 | 0.136 | 2.48 | 6.00 | `[14]=8  [19]=6  [20]=6  [13]=5  [10]=4` |
| MovCond | 32 | 26 | 81.2% | 26 | 39 | 100 | 0.099 | 3.97 | 0.00 | `[29]=25  [30]=25  [31]=21  [28]=18  [2]=5` |
| Cpu | 67 | 66 | 98.5% | 76 | 84 | 316 | 0.048 | 4.88 | 14.00 | `[65]=61  [8]=39  [6]=31  [7]=29  [47]=28` |

---

## sp1 (BabyBear · RISC-V)

| Table | Cols | Covered | Coverage | \|N\| | s | T\_arith | air\_sparsity | AIR cols/c | Lookup cols/c | Top-5 hot columns |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| AddSub | 19 | 18 | 94.7% | 18 | 18 | 91 | 0.281 | 3.72 | 0.00 | `[16]=15  [5]=6  [6]=6  [7]=4  [1]=3` |
| Bitwise | 17 | 16 | 94.1% | 16 | 5 | 16 | 0.200 | 2.00 | 6.71 | `[14]=31  [15]=31  [16]=31  [13]=29  [5]=7` |
| Mul | 39 | 38 | 97.4% | 38 | 31 | 191 | 0.162 | 4.84 | 3.00 | `[13]=9  [5]=8  [9]=8  [35]=8  [6]=7` |
| DivRem | 98 | 85 | 86.7% | 85 | 149 | 1158 | 0.091 | 4.25 | 13.11 | `[96]=93  [61]=50  [62]=46  [63]=46  [64]=42` |
| Lt | 33 | 28 | 84.8% | 28 | 29 | 131 | 0.161 | 3.90 | 4.33 | `[1]=13  [16]=13  [2]=11  [15]=11  [14]=9` |
| ShiftLeft | 44 | 40 | 90.9% | 40 | 65 | 238 | 0.092 | 2.78 | 0.00 | `[13]=17  [30]=12  [14]=10  [15]=10  [16]=10` |
| ShiftRight | 70 | 66 | 94.3% | 66 | 82 | 395 | 0.073 | 3.44 | 5.82 | `[59]=26  [60]=26  [61]=26  [69]=21  [68]=19` |
| Branch | 34 | 31 | 91.2% | 31 | 78 | 1956 | 0.809 | 8.26 | 14.91 | `[23]=88  [24]=88  [25]=88  [26]=88  [27]=88` |
| Jump | 26 | 23 | 88.5% | 23 | 9 | 71 | 0.343 | 3.89 | 13.00 | `[23]=8  [24]=8  [0]=3  [1]=3  [2]=3` |
| MemoryReadWrite | 57 | 54 | 94.7% | 54 | 76 | 423 | 0.103 | 4.53 | 9.40 | `[18]=28  [16]=27  [19]=24  [20]=24  [17]=23` |
| Cpu | 57 | 56 | 98.2% | 64 | 71 | 845 | 0.181 | 5.21 | 14.00 | `[56]=52  [7]=36  [41]=29  [6]=28  [40]=28` |

---

## sphinx (BabyBear · RISC-V fork)

| Table | Cols | Covered | Coverage | \|N\| | s | T\_arith | air\_sparsity | AIR cols/c | Lookup cols/c | Top-5 hot columns |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| AddSub | 20 | 18 | 90.0% | 19 | 19 | 56 | 0.155 | 4.21 | 0.00 | `[18]=16  [19]=16  [7]=6  [8]=6  [9]=4` |
| Bitwise | 18 | 16 | 88.9% | 17 | 6 | 14 | 0.137 | 1.33 | 5.71 | `[15]=30  [16]=30  [17]=30  [7]=7  [8]=7` |
| Mul | 40 | 38 | 95.0% | 39 | 32 | 141 | 0.113 | 4.44 | 3.00 | `[7]=8  [11]=8  [36]=8  [8]=7  [12]=7` |
| DivRem | 104 | 85 | 81.7% | 86 | 116 | 329 | 0.033 | 3.09 | 13.11 | `[102]=61  [62]=25  [63]=21  [64]=21  [93]=21` |
| Lt | 37 | 31 | 83.8% | 32 | 34 | 100 | 0.092 | 3.44 | 4.33 | `[3]=13  [20]=13  [4]=11  [19]=11  [18]=9` |
| ShiftLeft | 45 | 40 | 88.9% | 41 | 66 | 173 | 0.064 | 2.50 | 0.00 | `[31]=12  [15]=10  [16]=10  [17]=10  [40]=7` |
| ShiftRight | 71 | 66 | 93.0% | 67 | 83 | 204 | 0.037 | 3.35 | 5.82 | `[60]=26  [61]=26  [62]=26  [70]=20  [69]=19` |
| Cpu | 151 | 145 | 96.0% | 204 | 259 | 919 | 0.017 | 5.22 | 14.54 | `[140]=63  [41]=56  [42]=55  [44]=55  [43]=52` |

---

## pico (KoalaBear · RISC-V)

| Table | Cols | Covered | Coverage | \|N\| | s | T\_arith | air\_sparsity | AIR cols/c | Lookup cols/c | Top-5 hot columns |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| AddSub | 17 | 17 | 100.0% | 17 | 17 | 52 | 0.180 | 4.59 | 0.00 | `[15]=16  [16]=16  [4]=6  [5]=6  [6]=4` |
| Bitwise | 15 | 15 | 100.0% | 15 | 4 | 10 | 0.167 | 1.50 | 5.71 | `[12]=30  [13]=30  [14]=30  [4]=7  [5]=7` |
| Mul | 37 | 37 | 100.0% | 37 | 30 | 137 | 0.123 | 4.67 | 2.00 | `[4]=8  [8]=8  [33]=8  [5]=7  [9]=7` |
| DivRem | 96 | 96 | 100.0% | 96 | 131 | 357 | 0.028 | 2.98 | 13.35 | `[94]=77  [59]=23  [61]=23  [11]=21  [90]=21` |
| Lt | 30 | 30 | 100.0% | 30 | 32 | 96 | 0.100 | 3.59 | 2.33 | `[17]=13  [16]=11  [0]=10  [15]=9  [1]=8` |
| ShiftLeft | 42 | 39 | 92.9% | 39 | 64 | 169 | 0.068 | 2.55 | 0.00 | `[28]=12  [12]=10  [13]=10  [14]=10  [37]=7` |
| ShiftRight | 68 | 64 | 94.1% | 64 | 81 | 200 | 0.039 | 3.41 | 2.00 | `[66]=19  [56]=18  [20]=11  [12]=10  [13]=10` |
| MemoryReadWrite | 95 | 75 | 78.9% | 75 | 92 | 332 | 0.048 | 7.85 | 15.50 | `[58]=65  [56]=64  [59]=62  [60]=62  [57]=61` |
| Cpu | 117 | 117 | 100.0% | 161 | 261 | 783 | 0.019 | 4.80 | 14.51 | `[32]=73  [33]=73  [34]=73  [35]=73  [36]=73` |

---

## valida (BabyBear · Valida ISA)

| Table | Cols | Covered | Coverage | \|N\| | s | T\_arith | air\_sparsity | AIR cols/c | Lookup cols/c | Top-5 hot columns |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| Add32 | 16 | 15 | 93.8% | 15 | 10 | 34 | 0.227 | 3.20 | 0.00 | `[12]=4  [13]=4  [14]=3  [0]=2  [1]=2` |
| Sub32 | 17 | 16 | 94.1% | 16 | 8 | 27 | 0.211 | 2.88 | 0.00 | `[8]=3  [9]=3  [10]=3  [11]=2  [0]=1` |
| Bitwise32 | 79 | 79 | 100.0% | 79 | 88 | 398 | 0.057 | 4.07 | 0.00 | `[76]=6  [77]=6  [78]=6  [0]=5  [1]=5` |
| Mul32 | 27 | 24 | 88.9% | 25 | 7 | 109 | 0.623 | 7.14 | 3.00 | `[0]=4  [1]=4  [3]=4  [4]=4  [5]=4` |
| Div32 | 30 | 30 | 100.0% | 30 | 20 | 51 | 0.085 | 2.95 | 11.46 | `[29]=51  [28]=42  [7]=35  [4]=34  [5]=34` |
| Lt32 | 45 | 44 | 97.8% | 44 | 60 | 196 | 0.074 | 3.42 | 0.00 | `[8]=14  [9]=12  [10]=11  [11]=10  [20]=10` |
| Com32 | 14 | 14 | 100.0% | 14 | 8 | 29 | 0.259 | 2.88 | 0.00 | `[10]=4  [8]=3  [12]=3  [13]=3  [0]=1` |
| Memory | 27 | 27 | 100.0% | 38 | 42 | 109 | 0.068 | 2.67 | 0.00 | `[19]=14  [14]=13  [15]=11  [24]=11  [16]=9` |
| Cpu | 59 | 58 | 98.3% | 65 | 61 | 300 | 0.076 | 5.02 | 12.31 | `[32]=21  [44]=20  [45]=20  [46]=20  [47]=20` |

---

## Key Observations

- **50 tables** measured across 5 zkVM backends (ziren, sp1, sphinx, pico, valida).
- **Global average air\_sparsity: 0.140** (mean over all 50 tables).
- **T\_arith uses CSE**: common subexpressions shared across constraints are counted once. This reduces T\_arith substantially for complex tables (e.g. ziren DivRem: 1522→483, ziren Cpu: 910→316, sphinx Cpu: 1596→919). SP1 tables are unaffected, indicating their constraint expressions share no common subexpressions.
- **air\_sparsity** (`T_arith / (s × |N|)`) now uses CSE-aware T\_arith. Most tables fall in `[0.02, 0.35]`; only sp1 Branch (0.809) and valida Mul32 (0.623) exceed 0.5.
- **CPU tables** are consistently sparse (`0.017–0.181`), with sphinx Cpu the sparsest overall (`|N|=204`, `air_sparsity=0.017`).
- **Bitwise tables** show `|N| ≈ 15–17` (full table width) and `air_sparsity ≈ 0.138–0.200`, reflecting that their tiny AIR portion is moderately dense while the lookup side carries the structural richness.
- **sp1 Branch** is the densest table (0.809), driven by 78 selector-gated constraints referencing 31 neighborhood cells, with no CSE benefit.
- **valida Mul32** (0.623) is similarly dense: only 7 constraints but 109 arithmetic ops over 25 cells after CSE.
- **Sparsest by air\_sparsity**: sphinx Cpu (0.017), pico Cpu (0.019), pico DivRem (0.028), sphinx DivRem (0.033).

### air\_sparsity ranking (all 50 tables, ascending)

| VM | Table | \|N\| | s | T\_arith | air\_sparsity |
|---|---|---:|---:|---:|---:|
| sphinx | Cpu | 204 | 259 | 919 | 0.017 |
| pico | Cpu | 161 | 261 | 783 | 0.019 |
| pico | DivRem | 96 | 131 | 357 | 0.028 |
| sphinx | DivRem | 86 | 116 | 329 | 0.033 |
| sphinx | ShiftRight | 67 | 83 | 204 | 0.037 |
| ziren | DivRem | 88 | 143 | 483 | 0.038 |
| pico | ShiftRight | 64 | 81 | 200 | 0.039 |
| ziren | ShiftRight | 66 | 83 | 220 | 0.040 |
| ziren | Cpu | 76 | 84 | 316 | 0.048 |
| pico | MemoryReadWrite | 75 | 92 | 332 | 0.048 |
| ziren | MemoryReadWrite | 62 | 102 | 383 | 0.061 |
| sphinx | ShiftLeft | 41 | 66 | 173 | 0.064 |
| ziren | ShiftLeft | 39 | 64 | 169 | 0.068 |
| pico | ShiftLeft | 39 | 64 | 169 | 0.068 |
| valida | Memory | 38 | 42 | 109 | 0.068 |
| sp1 | ShiftRight | 66 | 82 | 395 | 0.073 |
| valida | Lt32 | 44 | 60 | 196 | 0.074 |
| ziren | Mul | 52 | 41 | 160 | 0.075 |
| valida | Cpu | 65 | 61 | 300 | 0.076 |
| valida | Div32 | 30 | 20 | 51 | 0.085 |
| sp1 | DivRem | 85 | 149 | 1158 | 0.091 |
| sp1 | ShiftLeft | 40 | 65 | 238 | 0.092 |
| sphinx | Lt | 32 | 34 | 100 | 0.092 |
| ziren | MovCond | 26 | 39 | 100 | 0.099 |
| ziren | Lt | 30 | 32 | 96 | 0.100 |
| pico | Lt | 30 | 32 | 96 | 0.100 |
| sp1 | MemoryReadWrite | 54 | 76 | 423 | 0.103 |
| sphinx | Mul | 39 | 32 | 141 | 0.113 |
| ziren | Branch | 33 | 31 | 119 | 0.116 |
| pico | Mul | 37 | 30 | 137 | 0.123 |
| ziren | CloClz | 20 | 21 | 57 | 0.136 |
| sphinx | Bitwise | 17 | 6 | 14 | 0.137 |
| ziren | Bitwise | 16 | 5 | 11 | 0.138 |
| sphinx | AddSub | 19 | 19 | 56 | 0.155 |
| sp1 | Lt | 28 | 29 | 131 | 0.161 |
| sp1 | Mul | 38 | 31 | 191 | 0.162 |
| pico | Bitwise | 15 | 4 | 10 | 0.167 |
| pico | AddSub | 17 | 17 | 52 | 0.180 |
| sp1 | Cpu | 64 | 71 | 845 | 0.181 |
| ziren | AddSub | 17 | 14 | 46 | 0.193 |
| sp1 | Bitwise | 16 | 5 | 16 | 0.200 |
| valida | Sub32 | 16 | 8 | 27 | 0.211 |
| valida | Add32 | 15 | 10 | 34 | 0.227 |
| valida | Com32 | 14 | 8 | 29 | 0.259 |
| sp1 | AddSub | 18 | 18 | 91 | 0.281 |
| ziren | Jump | 19 | 13 | 70 | 0.283 |
| sp1 | Jump | 23 | 9 | 71 | 0.343 |
| valida | Mul32 | 25 | 7 | 109 | 0.623 |
| sp1 | Branch | 31 | 78 | 1956 | 0.809 |
| **—** | **global avg** | | | | **0.140** |
