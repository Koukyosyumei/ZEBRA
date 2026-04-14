# ZEBRA Constraint Sparsity Report

> Generated: 2026-04-14  
> Branch: `sparcity`

## Overview

**Sparsity** measures how little of the available column space each constraint
actually touches. Two complementary views are reported per table:

| Metric | Meaning |
|--------|---------|
| **Column coverage** | Fraction of columns referenced by ≥ 1 constraint |
| **Cols / constraint** | Mean number of columns a single constraint uses |

A table with 100 columns where each constraint touches only 4 columns has a
per-constraint density of **4 %** — highly sparse even if column coverage is high.

## Summary Table

| VM | Table | Opcode | Cols | Covered | Coverage | AIR constraints | AIR cols/c | Lookup constraints | Lookup cols/c |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|
| ziren | AddSub | `ADD` | 19 | 17 | 89.5% | 14 | 4.36 | 0 | 0.00 |
| ziren | Bitwise | `AND` | 18 | 16 | 88.9% | 5 | 1.60 | 28 | 6.71 |
| ziren | Mul | `MUL` | 58 | 52 | 89.7% | 41 | 4.00 | 2 | 3.00 |
| ziren | DivRem | `DIVU` | 106 | 88 | 83.0% | 143 | 5.24 | 17 | 13.41 |
| ziren | Lt | `SLT` | 36 | 30 | 83.3% | 32 | 3.59 | 3 | 4.33 |
| ziren | ShiftLeft | `SLL` | 44 | 39 | 88.6% | 64 | 2.55 | 0 | 0.00 |
| ziren | ShiftRight | `SRL` | 71 | 66 | 93.0% | 83 | 3.75 | 17 | 5.82 |
| ziren | Branch | `BEQ` | 62 | 33 | 53.2% | 31 | 4.84 | 3 | 14.33 |
| ziren | Jump | `Jump` | 66 | 19 | 28.8% | 13 | 4.23 | 1 | 13.00 |
| ziren | MemoryReadWrite | `LW` | 79 | 62 | 78.5% | 102 | 5.90 | 5 | 11.80 |
| ziren | CloClz | `CLO` | 22 | 20 | 90.9% | 21 | 2.48 | 2 | 6.00 |
| ziren | MovCond | `MEQ` | 32 | 26 | 81.2% | 39 | 3.97 | 0 | 0.00 |
| sp1 | AddSub | `ADD` | 19 | 18 | 94.7% | 18 | 3.72 | 0 | 0.00 |
| sp1 | Bitwise | `AND` | 17 | 16 | 94.1% | 5 | 2.00 | 28 | 6.71 |
| sp1 | Mul | `MUL` | 39 | 38 | 97.4% | 31 | 4.84 | 2 | 3.00 |
| sp1 | DivRem | `DIVU` | 98 | 85 | 86.7% | 149 | 4.25 | 18 | 13.11 |
| sp1 | Lt | `SLTU` | 33 | 28 | 84.8% | 29 | 3.90 | 3 | 4.33 |
| sp1 | ShiftLeft | `SLL` | 44 | 40 | 90.9% | 65 | 2.78 | 0 | 0.00 |
| sp1 | ShiftRight | `SRL` | 70 | 66 | 94.3% | 82 | 3.44 | 17 | 5.82 |
| sp1 | Branch | `BEQ` | 34 | 31 | 91.2% | 78 | 8.26 | 23 | 14.91 |
| sp1 | Jump | `JAL` | 26 | 23 | 88.5% | 9 | 3.89 | 2 | 13.00 |
| sp1 | MemoryReadWrite | `LW` | 57 | 54 | 94.7% | 76 | 4.53 | 5 | 9.40 |
| sphinx | AddSub | `ADD` | 20 | 18 | 90.0% | 19 | 4.21 | 0 | 0.00 |
| sphinx | Bitwise | `AND` | 18 | 16 | 88.9% | 6 | 1.33 | 28 | 5.71 |
| sphinx | Mul | `MUL` | 40 | 38 | 95.0% | 32 | 4.44 | 2 | 3.00 |
| sphinx | DivRem | `DIVU` | 104 | 85 | 81.7% | 116 | 3.09 | 18 | 13.11 |
| sphinx | Lt | `SLTU` | 37 | 31 | 83.8% | 34 | 3.44 | 3 | 4.33 |
| sphinx | ShiftLeft | `SLL` | 45 | 40 | 88.9% | 66 | 2.50 | 0 | 0.00 |
| sphinx | ShiftRight | `SRL` | 71 | 66 | 93.0% | 83 | 3.35 | 17 | 5.82 |
| openvm | BaseAlu | `ADD` | 25 | 17 | 68.0% | 0 | 0.00 | 5 | 13.00 |
| openvm | BitwiseAlu | `AND` | 25 | 17 | 68.0% | 0 | 0.00 | 5 | 13.00 |
| openvm | Mul | `MUL` | 21 | 13 | 61.9% | 0 | 0.00 | 1 | 13.00 |
| openvm | Lt | `SLT` | 26 | 11 | 42.3% | 0 | 0.00 | 2 | 10.00 |
| openvm | Shift | `SLL` | 42 | 13 | 31.0% | 0 | 0.00 | 1 | 13.00 |
| openvm | BranchEqual | `BEQ` | 24 | 11 | 45.8% | 0 | 0.00 | 2 | 10.00 |
| openvm | Jal | `JAL` | 15 | 8 | 53.3% | 2 | 6.00 | 0 | 0.00 |
| pico | AddSub | `ADD` | 17 | 17 | 100.0% | 17 | 4.59 | 0 | 0.00 |
| pico | Bitwise | `AND` | 15 | 15 | 100.0% | 4 | 1.50 | 28 | 5.71 |
| pico | Mul | `MUL` | 37 | 37 | 100.0% | 30 | 4.67 | 2 | 2.00 |
| pico | DivRem | `DIVU` | 96 | 96 | 100.0% | 131 | 2.98 | 20 | 13.35 |
| pico | Lt | `SLT` | 30 | 30 | 100.0% | 32 | 3.59 | 3 | 2.33 |
| pico | ShiftLeft | `SLL` | 42 | 39 | 92.9% | 64 | 2.55 | 0 | 0.00 |
| pico | ShiftRight | `SRL` | 68 | 64 | 94.1% | 81 | 3.41 | 1 | 2.00 |
| pico | MemoryReadWrite | `LW` | 95 | 75 | 78.9% | 92 | 7.85 | 2 | 15.50 |
| valida | Add32 | `ADD` | 16 | 15 | 93.8% | 10 | 3.20 | 0 | 0.00 |
| valida | Sub32 | `SUB` | 17 | 16 | 94.1% | 8 | 2.88 | 0 | 0.00 |
| valida | Bitwise32 | `AND` | 79 | 79 | 100.0% | 88 | 4.07 | 0 | 0.00 |
| valida | Mul32 | `MUL32` | 27 | 24 | 88.9% | 7 | 7.14 | 2 | 3.00 |
| valida | Div32 | `DIV` | 30 | 30 | 100.0% | 20 | 2.95 | 39 | 11.46 |
| valida | Lt32 | `LT` | 45 | 44 | 97.8% | 60 | 3.42 | 0 | 0.00 |
| valida | Com32 | `EQ` | 14 | 14 | 100.0% | 8 | 2.88 | 0 | 0.00 |
| valida | Memory | `LOAD` | 27 | 27 | 100.0% | 42 | 2.67 | 0 | 0.00 |

---

## ziren (KoalaBear · MIPS/RISC-V)

| Table | Opcode | Total cols | Covered cols | Total coverage | AIR cols/c | Lookup cols/c | Top-5 hot columns |
|---|---|---:|---:|---:|---:|---:|---|
| AddSub | `ADD` | 19 | 17 | 89.5% | 4.36 | 0.00 | `hit_count):  [17]=13  [18]=13  [6]=5  [7]=5  [8]=4` |
| Bitwise | `AND` | 18 | 16 | 88.9% | 1.60 | 6.71 | `hit_count):  [15]=31  [16]=30  [17]=30  [14]=29  [6]=7` |
| Mul | `MUL` | 58 | 52 | 89.7% | 4.00 | 3.00 | `hit_count):  [55]=12  [39]=9  [10]=8  [14]=8  [11]=7` |
| DivRem | `DIVU` | 106 | 88 | 83.0% | 5.24 | 13.41 | `hit_count):  [57]=108  [58]=104  [59]=100  [60]=96  [10]=28` |
| Lt | `SLT` | 36 | 30 | 83.3% | 3.59 | 4.33 | `hit_count):  [2]=13  [19]=13  [3]=11  [18]=11  [17]=9` |
| ShiftLeft | `SLL` | 44 | 39 | 88.6% | 2.55 | 0.00 | `hit_count):  [30]=12  [14]=10  [15]=10  [16]=10  [39]=7` |
| ShiftRight | `SRL` | 71 | 66 | 93.0% | 3.75 | 5.82 | `hit_count):  [59]=26  [60]=26  [61]=26  [70]=20  [68]=19` |
| Branch | `BEQ` | 62 | 33 | 53.2% | 4.84 | 14.33 | `hit_count):  [59]=21  [53]=17  [54]=17  [55]=17  [56]=17` |
| Jump | `Jump` | 66 | 19 | 28.8% | 4.23 | 13.00 | `hit_count):  [49]=10  [50]=10  [51]=8  [1]=3  [2]=3` |
| MemoryReadWrite | `LW` | 79 | 62 | 78.5% | 5.90 | 11.80 | `hit_count):  [38]=33  [18]=31  [36]=31  [37]=31  [16]=30` |
| CloClz | `CLO` | 22 | 20 | 90.9% | 2.48 | 6.00 | `hit_count):  [14]=8  [19]=6  [20]=6  [13]=5  [10]=4` |
| MovCond | `MEQ` | 32 | 26 | 81.2% | 3.97 | 0.00 | `hit_count):  [29]=25  [30]=25  [31]=21  [28]=18  [2]=5` |

### AddSub (`ADD`)
- **Columns**: 19  
- **Total coverage**: 17/19 = 89.5%  
- **AIR**: 14 constraints, 17/19 cols covered (89.5%), avg **4.36 cols/constraint** (22.9% density)  
- **Lookup**: 0 constraints, 0/19 cols covered (0.0%), avg **0.00 cols/constraint** (0.0% density)  
- **Top-5 hottest**: `hit_count):  [17]=13  [18]=13  [6]=5  [7]=5  [8]=4`

### Bitwise (`AND`)
- **Columns**: 18  
- **Total coverage**: 16/18 = 88.9%  
- **AIR**: 5 constraints, 4/18 cols covered (22.2%), avg **1.60 cols/constraint** (8.9% density)  
- **Lookup**: 28 constraints, 16/18 cols covered (88.9%), avg **6.71 cols/constraint** (37.3% density)  
- **Top-5 hottest**: `hit_count):  [15]=31  [16]=30  [17]=30  [14]=29  [6]=7`

### Mul (`MUL`)
- **Columns**: 58  
- **Total coverage**: 52/58 = 89.7%  
- **AIR**: 41 constraints, 52/58 cols covered (89.7%), avg **4.00 cols/constraint** (6.9% density)  
- **Lookup**: 2 constraints, 5/58 cols covered (8.6%), avg **3.00 cols/constraint** (5.2% density)  
- **Top-5 hottest**: `hit_count):  [55]=12  [39]=9  [10]=8  [14]=8  [11]=7`

### DivRem (`DIVU`)
- **Columns**: 106  
- **Total coverage**: 88/106 = 83.0%  
- **AIR**: 143 constraints, 88/106 cols covered (83.0%), avg **5.24 cols/constraint** (4.9% density)  
- **Lookup**: 17 constraints, 38/106 cols covered (35.8%), avg **13.41 cols/constraint** (12.7% density)  
- **Top-5 hottest**: `hit_count):  [57]=108  [58]=104  [59]=100  [60]=96  [10]=28`

### Lt (`SLT`)
- **Columns**: 36  
- **Total coverage**: 30/36 = 83.3%  
- **AIR**: 32 constraints, 30/36 cols covered (83.3%), avg **3.59 cols/constraint** (10.0% density)  
- **Lookup**: 3 constraints, 9/36 cols covered (25.0%), avg **4.33 cols/constraint** (12.0% density)  
- **Top-5 hottest**: `hit_count):  [2]=13  [19]=13  [3]=11  [18]=11  [17]=9`

### ShiftLeft (`SLL`)
- **Columns**: 44  
- **Total coverage**: 39/44 = 88.6%  
- **AIR**: 64 constraints, 39/44 cols covered (88.6%), avg **2.55 cols/constraint** (5.8% density)  
- **Lookup**: 0 constraints, 0/44 cols covered (0.0%), avg **0.00 cols/constraint** (0.0% density)  
- **Top-5 hottest**: `hit_count):  [30]=12  [14]=10  [15]=10  [16]=10  [39]=7`

### ShiftRight (`SRL`)
- **Columns**: 71  
- **Total coverage**: 66/71 = 93.0%  
- **AIR**: 83 constraints, 65/71 cols covered (91.5%), avg **3.75 cols/constraint** (5.3% density)  
- **Lookup**: 17 constraints, 30/71 cols covered (42.3%), avg **5.82 cols/constraint** (8.2% density)  
- **Top-5 hottest**: `hit_count):  [59]=26  [60]=26  [61]=26  [70]=20  [68]=19`

### Branch (`BEQ`)
- **Columns**: 62  
- **Total coverage**: 33/62 = 53.2%  
- **AIR**: 31 constraints, 21/62 cols covered (33.9%), avg **4.84 cols/constraint** (7.8% density)  
- **Lookup**: 3 constraints, 29/62 cols covered (46.8%), avg **14.33 cols/constraint** (23.1% density)  
- **Top-5 hottest**: `hit_count):  [59]=21  [53]=17  [54]=17  [55]=17  [56]=17`

### Jump (`Jump`)
- **Columns**: 66  
- **Total coverage**: 19/66 = 28.8%  
- **AIR**: 13 constraints, 19/66 cols covered (28.8%), avg **4.23 cols/constraint** (6.4% density)  
- **Lookup**: 1 constraints, 13/66 cols covered (19.7%), avg **13.00 cols/constraint** (19.7% density)  
- **Top-5 hottest**: `hit_count):  [49]=10  [50]=10  [51]=8  [1]=3  [2]=3`

### MemoryReadWrite (`LW`)
- **Columns**: 79  
- **Total coverage**: 62/79 = 78.5%  
- **AIR**: 102 constraints, 54/79 cols covered (68.4%), avg **5.90 cols/constraint** (7.5% density)  
- **Lookup**: 5 constraints, 39/79 cols covered (49.4%), avg **11.80 cols/constraint** (14.9% density)  
- **Top-5 hottest**: `hit_count):  [38]=33  [18]=31  [36]=31  [37]=31  [16]=30`

### CloClz (`CLO`)
- **Columns**: 22  
- **Total coverage**: 20/22 = 90.9%  
- **AIR**: 21 constraints, 20/22 cols covered (90.9%), avg **2.48 cols/constraint** (11.3% density)  
- **Lookup**: 2 constraints, 11/22 cols covered (50.0%), avg **6.00 cols/constraint** (27.3% density)  
- **Top-5 hottest**: `hit_count):  [14]=8  [19]=6  [20]=6  [13]=5  [10]=4`

### MovCond (`MEQ`)
- **Columns**: 32  
- **Total coverage**: 26/32 = 81.2%  
- **AIR**: 39 constraints, 26/32 cols covered (81.2%), avg **3.97 cols/constraint** (12.4% density)  
- **Lookup**: 0 constraints, 0/32 cols covered (0.0%), avg **0.00 cols/constraint** (0.0% density)  
- **Top-5 hottest**: `hit_count):  [29]=25  [30]=25  [31]=21  [28]=18  [2]=5`


---

## sp1 (BabyBear · RISC-V)

| Table | Opcode | Total cols | Covered cols | Total coverage | AIR cols/c | Lookup cols/c | Top-5 hot columns |
|---|---|---:|---:|---:|---:|---:|---|
| AddSub | `ADD` | 19 | 18 | 94.7% | 3.72 | 0.00 | `hit_count):  [16]=15  [5]=6  [6]=6  [7]=4  [1]=3` |
| Bitwise | `AND` | 17 | 16 | 94.1% | 2.00 | 6.71 | `hit_count):  [14]=31  [15]=31  [16]=31  [13]=29  [5]=7` |
| Mul | `MUL` | 39 | 38 | 97.4% | 4.84 | 3.00 | `hit_count):  [13]=9  [5]=8  [9]=8  [35]=8  [6]=7` |
| DivRem | `DIVU` | 98 | 85 | 86.7% | 4.25 | 13.11 | `hit_count):  [96]=93  [61]=50  [62]=46  [63]=46  [64]=42` |
| Lt | `SLTU` | 33 | 28 | 84.8% | 3.90 | 4.33 | `hit_count):  [1]=13  [16]=13  [2]=11  [15]=11  [14]=9` |
| ShiftLeft | `SLL` | 44 | 40 | 90.9% | 2.78 | 0.00 | `hit_count):  [13]=17  [30]=12  [14]=10  [15]=10  [16]=10` |
| ShiftRight | `SRL` | 70 | 66 | 94.3% | 3.44 | 5.82 | `hit_count):  [59]=26  [60]=26  [61]=26  [69]=21  [68]=19` |
| Branch | `BEQ` | 34 | 31 | 91.2% | 8.26 | 14.91 | `hit_count):  [23]=88  [24]=88  [25]=88  [26]=88  [27]=88` |
| Jump | `JAL` | 26 | 23 | 88.5% | 3.89 | 13.00 | `hit_count):  [23]=8  [24]=8  [0]=3  [1]=3  [2]=3` |
| MemoryReadWrite | `LW` | 57 | 54 | 94.7% | 4.53 | 9.40 | `hit_count):  [18]=28  [16]=27  [19]=24  [20]=24  [17]=23` |

### AddSub (`ADD`)
- **Columns**: 19  
- **Total coverage**: 18/19 = 94.7%  
- **AIR**: 18 constraints, 18/19 cols covered (94.7%), avg **3.72 cols/constraint** (19.6% density)  
- **Lookup**: 0 constraints, 0/19 cols covered (0.0%), avg **0.00 cols/constraint** (0.0% density)  
- **Top-5 hottest**: `hit_count):  [16]=15  [5]=6  [6]=6  [7]=4  [1]=3`

### Bitwise (`AND`)
- **Columns**: 17  
- **Total coverage**: 16/17 = 94.1%  
- **AIR**: 5 constraints, 4/17 cols covered (23.5%), avg **2.00 cols/constraint** (11.8% density)  
- **Lookup**: 28 constraints, 16/17 cols covered (94.1%), avg **6.71 cols/constraint** (39.5% density)  
- **Top-5 hottest**: `hit_count):  [14]=31  [15]=31  [16]=31  [13]=29  [5]=7`

### Mul (`MUL`)
- **Columns**: 39  
- **Total coverage**: 38/39 = 97.4%  
- **AIR**: 31 constraints, 38/39 cols covered (97.4%), avg **4.84 cols/constraint** (12.4% density)  
- **Lookup**: 2 constraints, 5/39 cols covered (12.8%), avg **3.00 cols/constraint** (7.7% density)  
- **Top-5 hottest**: `hit_count):  [13]=9  [5]=8  [9]=8  [35]=8  [6]=7`

### DivRem (`DIVU`)
- **Columns**: 98  
- **Total coverage**: 85/98 = 86.7%  
- **AIR**: 149 constraints, 85/98 cols covered (86.7%), avg **4.25 cols/constraint** (4.3% density)  
- **Lookup**: 18 constraints, 44/98 cols covered (44.9%), avg **13.11 cols/constraint** (13.4% density)  
- **Top-5 hottest**: `hit_count):  [96]=93  [61]=50  [62]=46  [63]=46  [64]=42`

### Lt (`SLTU`)
- **Columns**: 33  
- **Total coverage**: 28/33 = 84.8%  
- **AIR**: 29 constraints, 28/33 cols covered (84.8%), avg **3.90 cols/constraint** (11.8% density)  
- **Lookup**: 3 constraints, 9/33 cols covered (27.3%), avg **4.33 cols/constraint** (13.1% density)  
- **Top-5 hottest**: `hit_count):  [1]=13  [16]=13  [2]=11  [15]=11  [14]=9`

### ShiftLeft (`SLL`)
- **Columns**: 44  
- **Total coverage**: 40/44 = 90.9%  
- **AIR**: 65 constraints, 40/44 cols covered (90.9%), avg **2.78 cols/constraint** (6.3% density)  
- **Lookup**: 0 constraints, 0/44 cols covered (0.0%), avg **0.00 cols/constraint** (0.0% density)  
- **Top-5 hottest**: `hit_count):  [13]=17  [30]=12  [14]=10  [15]=10  [16]=10`

### ShiftRight (`SRL`)
- **Columns**: 70  
- **Total coverage**: 66/70 = 94.3%  
- **AIR**: 82 constraints, 65/70 cols covered (92.9%), avg **3.44 cols/constraint** (4.9% density)  
- **Lookup**: 17 constraints, 30/70 cols covered (42.9%), avg **5.82 cols/constraint** (8.3% density)  
- **Top-5 hottest**: `hit_count):  [59]=26  [60]=26  [61]=26  [69]=21  [68]=19`

### Branch (`BEQ`)
- **Columns**: 34  
- **Total coverage**: 31/34 = 91.2%  
- **AIR**: 78 constraints, 27/34 cols covered (79.4%), avg **8.26 cols/constraint** (24.3% density)  
- **Lookup**: 23 constraints, 29/34 cols covered (85.3%), avg **14.91 cols/constraint** (43.9% density)  
- **Top-5 hottest**: `hit_count):  [23]=88  [24]=88  [25]=88  [26]=88  [27]=88`

### Jump (`JAL`)
- **Columns**: 26  
- **Total coverage**: 23/26 = 88.5%  
- **AIR**: 9 constraints, 15/26 cols covered (57.7%), avg **3.89 cols/constraint** (15.0% density)  
- **Lookup**: 2 constraints, 18/26 cols covered (69.2%), avg **13.00 cols/constraint** (50.0% density)  
- **Top-5 hottest**: `hit_count):  [23]=8  [24]=8  [0]=3  [1]=3  [2]=3`

### MemoryReadWrite (`LW`)
- **Columns**: 57  
- **Total coverage**: 54/57 = 94.7%  
- **AIR**: 76 constraints, 46/57 cols covered (80.7%), avg **4.53 cols/constraint** (7.9% density)  
- **Lookup**: 5 constraints, 33/57 cols covered (57.9%), avg **9.40 cols/constraint** (16.5% density)  
- **Top-5 hottest**: `hit_count):  [18]=28  [16]=27  [19]=24  [20]=24  [17]=23`


---

## sphinx (BabyBear · RISC-V fork)

| Table | Opcode | Total cols | Covered cols | Total coverage | AIR cols/c | Lookup cols/c | Top-5 hot columns |
|---|---|---:|---:|---:|---:|---:|---|
| AddSub | `ADD` | 20 | 18 | 90.0% | 4.21 | 0.00 | `hit_count):  [18]=16  [19]=16  [7]=6  [8]=6  [9]=4` |
| Bitwise | `AND` | 18 | 16 | 88.9% | 1.33 | 5.71 | `hit_count):  [15]=30  [16]=30  [17]=30  [7]=7  [8]=7` |
| Mul | `MUL` | 40 | 38 | 95.0% | 4.44 | 3.00 | `hit_count):  [7]=8  [11]=8  [36]=8  [8]=7  [12]=7` |
| DivRem | `DIVU` | 104 | 85 | 81.7% | 3.09 | 13.11 | `hit_count):  [102]=61  [62]=25  [63]=21  [64]=21  [93]=21` |
| Lt | `SLTU` | 37 | 31 | 83.8% | 3.44 | 4.33 | `hit_count):  [3]=13  [20]=13  [4]=11  [19]=11  [18]=9` |
| ShiftLeft | `SLL` | 45 | 40 | 88.9% | 2.50 | 0.00 | `hit_count):  [31]=12  [15]=10  [16]=10  [17]=10  [40]=7` |
| ShiftRight | `SRL` | 71 | 66 | 93.0% | 3.35 | 5.82 | `hit_count):  [60]=26  [61]=26  [62]=26  [70]=20  [69]=19` |

### AddSub (`ADD`)
- **Columns**: 20  
- **Total coverage**: 18/20 = 90.0%  
- **AIR**: 19 constraints, 18/20 cols covered (90.0%), avg **4.21 cols/constraint** (21.1% density)  
- **Lookup**: 0 constraints, 0/20 cols covered (0.0%), avg **0.00 cols/constraint** (0.0% density)  
- **Top-5 hottest**: `hit_count):  [18]=16  [19]=16  [7]=6  [8]=6  [9]=4`

### Bitwise (`AND`)
- **Columns**: 18  
- **Total coverage**: 16/18 = 88.9%  
- **AIR**: 6 constraints, 4/18 cols covered (22.2%), avg **1.33 cols/constraint** (7.4% density)  
- **Lookup**: 28 constraints, 15/18 cols covered (83.3%), avg **5.71 cols/constraint** (31.7% density)  
- **Top-5 hottest**: `hit_count):  [15]=30  [16]=30  [17]=30  [7]=7  [8]=7`

### Mul (`MUL`)
- **Columns**: 40  
- **Total coverage**: 38/40 = 95.0%  
- **AIR**: 32 constraints, 38/40 cols covered (95.0%), avg **4.44 cols/constraint** (11.1% density)  
- **Lookup**: 2 constraints, 5/40 cols covered (12.5%), avg **3.00 cols/constraint** (7.5% density)  
- **Top-5 hottest**: `hit_count):  [7]=8  [11]=8  [36]=8  [8]=7  [12]=7`

### DivRem (`DIVU`)
- **Columns**: 104  
- **Total coverage**: 85/104 = 81.7%  
- **AIR**: 116 constraints, 85/104 cols covered (81.7%), avg **3.09 cols/constraint** (3.0% density)  
- **Lookup**: 18 constraints, 44/104 cols covered (42.3%), avg **13.11 cols/constraint** (12.6% density)  
- **Top-5 hottest**: `hit_count):  [102]=61  [62]=25  [63]=21  [64]=21  [93]=21`

### Lt (`SLTU`)
- **Columns**: 37  
- **Total coverage**: 31/37 = 83.8%  
- **AIR**: 34 constraints, 31/37 cols covered (83.8%), avg **3.44 cols/constraint** (9.3% density)  
- **Lookup**: 3 constraints, 9/37 cols covered (24.3%), avg **4.33 cols/constraint** (11.7% density)  
- **Top-5 hottest**: `hit_count):  [3]=13  [20]=13  [4]=11  [19]=11  [18]=9`

### ShiftLeft (`SLL`)
- **Columns**: 45  
- **Total coverage**: 40/45 = 88.9%  
- **AIR**: 66 constraints, 40/45 cols covered (88.9%), avg **2.50 cols/constraint** (5.6% density)  
- **Lookup**: 0 constraints, 0/45 cols covered (0.0%), avg **0.00 cols/constraint** (0.0% density)  
- **Top-5 hottest**: `hit_count):  [31]=12  [15]=10  [16]=10  [17]=10  [40]=7`

### ShiftRight (`SRL`)
- **Columns**: 71  
- **Total coverage**: 66/71 = 93.0%  
- **AIR**: 83 constraints, 65/71 cols covered (91.5%), avg **3.35 cols/constraint** (4.7% density)  
- **Lookup**: 17 constraints, 30/71 cols covered (42.3%), avg **5.82 cols/constraint** (8.2% density)  
- **Top-5 hottest**: `hit_count):  [60]=26  [61]=26  [62]=26  [70]=20  [69]=19`


---

## openvm (BabyBear · RV32IM)

| Table | Opcode | Total cols | Covered cols | Total coverage | AIR cols/c | Lookup cols/c | Top-5 hot columns |
|---|---|---:|---:|---:|---:|---:|---|
| BaseAlu | `ADD` | 25 | 17 | 68.0% | 0.00 | 13.00 | `hit_count):  [8]=5  [9]=5  [10]=5  [11]=5  [12]=5` |
| BitwiseAlu | `AND` | 25 | 17 | 68.0% | 0.00 | 13.00 | `hit_count):  [8]=5  [9]=5  [10]=5  [11]=5  [12]=5` |
| Mul | `MUL` | 21 | 13 | 61.9% | 0.00 | 13.00 | `hit_count):  [8]=1  [9]=1  [10]=1  [11]=1  [12]=1` |
| Lt | `SLT` | 26 | 11 | 42.3% | 0.00 | 10.00 | `hit_count):  [8]=2  [9]=2  [10]=2  [11]=2  [12]=2` |
| Shift | `SLL` | 42 | 13 | 31.0% | 0.00 | 13.00 | `hit_count):  [8]=1  [9]=1  [10]=1  [11]=1  [12]=1` |
| BranchEqual | `BEQ` | 24 | 11 | 45.8% | 0.00 | 10.00 | `hit_count):  [8]=2  [9]=2  [10]=2  [11]=2  [12]=2` |
| Jal | `JAL` | 15 | 8 | 53.3% | 6.00 | 0.00 | `hit_count):  [9]=2  [10]=2  [11]=2  [12]=2  [0]=1` |

### BaseAlu (`ADD`)
- **Columns**: 25  
- **Total coverage**: 17/25 = 68.0%  
- **AIR**: 0 constraints, 0/25 cols covered (0.0%), avg **0.00 cols/constraint** (0.0% density)  
- **Lookup**: 5 constraints, 17/25 cols covered (68.0%), avg **13.00 cols/constraint** (52.0% density)  
- **Top-5 hottest**: `hit_count):  [8]=5  [9]=5  [10]=5  [11]=5  [12]=5`

### BitwiseAlu (`AND`)
- **Columns**: 25  
- **Total coverage**: 17/25 = 68.0%  
- **AIR**: 0 constraints, 0/25 cols covered (0.0%), avg **0.00 cols/constraint** (0.0% density)  
- **Lookup**: 5 constraints, 17/25 cols covered (68.0%), avg **13.00 cols/constraint** (52.0% density)  
- **Top-5 hottest**: `hit_count):  [8]=5  [9]=5  [10]=5  [11]=5  [12]=5`

### Mul (`MUL`)
- **Columns**: 21  
- **Total coverage**: 13/21 = 61.9%  
- **AIR**: 0 constraints, 0/21 cols covered (0.0%), avg **0.00 cols/constraint** (0.0% density)  
- **Lookup**: 1 constraints, 13/21 cols covered (61.9%), avg **13.00 cols/constraint** (61.9% density)  
- **Top-5 hottest**: `hit_count):  [8]=1  [9]=1  [10]=1  [11]=1  [12]=1`

### Lt (`SLT`)
- **Columns**: 26  
- **Total coverage**: 11/26 = 42.3%  
- **AIR**: 0 constraints, 0/26 cols covered (0.0%), avg **0.00 cols/constraint** (0.0% density)  
- **Lookup**: 2 constraints, 11/26 cols covered (42.3%), avg **10.00 cols/constraint** (38.5% density)  
- **Top-5 hottest**: `hit_count):  [8]=2  [9]=2  [10]=2  [11]=2  [12]=2`

### Shift (`SLL`)
- **Columns**: 42  
- **Total coverage**: 13/42 = 31.0%  
- **AIR**: 0 constraints, 0/42 cols covered (0.0%), avg **0.00 cols/constraint** (0.0% density)  
- **Lookup**: 1 constraints, 13/42 cols covered (31.0%), avg **13.00 cols/constraint** (31.0% density)  
- **Top-5 hottest**: `hit_count):  [8]=1  [9]=1  [10]=1  [11]=1  [12]=1`

### BranchEqual (`BEQ`)
- **Columns**: 24  
- **Total coverage**: 11/24 = 45.8%  
- **AIR**: 0 constraints, 0/24 cols covered (0.0%), avg **0.00 cols/constraint** (0.0% density)  
- **Lookup**: 2 constraints, 11/24 cols covered (45.8%), avg **10.00 cols/constraint** (41.7% density)  
- **Top-5 hottest**: `hit_count):  [8]=2  [9]=2  [10]=2  [11]=2  [12]=2`

### Jal (`JAL`)
- **Columns**: 15  
- **Total coverage**: 8/15 = 53.3%  
- **AIR**: 2 constraints, 8/15 cols covered (53.3%), avg **6.00 cols/constraint** (40.0% density)  
- **Lookup**: 0 constraints, 0/15 cols covered (0.0%), avg **0.00 cols/constraint** (0.0% density)  
- **Top-5 hottest**: `hit_count):  [9]=2  [10]=2  [11]=2  [12]=2  [0]=1`


---

## pico (KoalaBear · RISC-V)

| Table | Opcode | Total cols | Covered cols | Total coverage | AIR cols/c | Lookup cols/c | Top-5 hot columns |
|---|---|---:|---:|---:|---:|---:|---|
| AddSub | `ADD` | 17 | 17 | 100.0% | 4.59 | 0.00 | `hit_count):  [15]=16  [16]=16  [4]=6  [5]=6  [6]=4` |
| Bitwise | `AND` | 15 | 15 | 100.0% | 1.50 | 5.71 | `hit_count):  [12]=30  [13]=30  [14]=30  [4]=7  [5]=7` |
| Mul | `MUL` | 37 | 37 | 100.0% | 4.67 | 2.00 | `hit_count):  [4]=8  [8]=8  [33]=8  [5]=7  [9]=7` |
| DivRem | `DIVU` | 96 | 96 | 100.0% | 2.98 | 13.35 | `hit_count):  [94]=77  [59]=23  [61]=23  [11]=21  [90]=21` |
| Lt | `SLT` | 30 | 30 | 100.0% | 3.59 | 2.33 | `hit_count):  [17]=13  [16]=11  [0]=10  [15]=9  [1]=8` |
| ShiftLeft | `SLL` | 42 | 39 | 92.9% | 2.55 | 0.00 | `hit_count):  [28]=12  [12]=10  [13]=10  [14]=10  [37]=7` |
| ShiftRight | `SRL` | 68 | 64 | 94.1% | 3.41 | 2.00 | `hit_count):  [66]=19  [56]=18  [20]=11  [12]=10  [13]=10` |
| MemoryReadWrite | `LW` | 95 | 75 | 78.9% | 7.85 | 15.50 | `hit_count):  [58]=65  [56]=64  [59]=62  [60]=62  [57]=61` |

### AddSub (`ADD`)
- **Columns**: 17  
- **Total coverage**: 17/17 = 100.0%  
- **AIR**: 17 constraints, 17/17 cols covered (100.0%), avg **4.59 cols/constraint** (27.0% density)  
- **Lookup**: 0 constraints, 0/17 cols covered (0.0%), avg **0.00 cols/constraint** (0.0% density)  
- **Top-5 hottest**: `hit_count):  [15]=16  [16]=16  [4]=6  [5]=6  [6]=4`

### Bitwise (`AND`)
- **Columns**: 15  
- **Total coverage**: 15/15 = 100.0%  
- **AIR**: 4 constraints, 3/15 cols covered (20.0%), avg **1.50 cols/constraint** (10.0% density)  
- **Lookup**: 28 constraints, 15/15 cols covered (100.0%), avg **5.71 cols/constraint** (38.1% density)  
- **Top-5 hottest**: `hit_count):  [12]=30  [13]=30  [14]=30  [4]=7  [5]=7`

### Mul (`MUL`)
- **Columns**: 37  
- **Total coverage**: 37/37 = 100.0%  
- **AIR**: 30 constraints, 37/37 cols covered (100.0%), avg **4.67 cols/constraint** (12.6% density)  
- **Lookup**: 2 constraints, 4/37 cols covered (10.8%), avg **2.00 cols/constraint** (5.4% density)  
- **Top-5 hottest**: `hit_count):  [4]=8  [8]=8  [33]=8  [5]=7  [9]=7`

### DivRem (`DIVU`)
- **Columns**: 96  
- **Total coverage**: 96/96 = 100.0%  
- **AIR**: 131 constraints, 96/96 cols covered (100.0%), avg **2.98 cols/constraint** (3.1% density)  
- **Lookup**: 20 constraints, 44/96 cols covered (45.8%), avg **13.35 cols/constraint** (13.9% density)  
- **Top-5 hottest**: `hit_count):  [94]=77  [59]=23  [61]=23  [11]=21  [90]=21`

### Lt (`SLT`)
- **Columns**: 30  
- **Total coverage**: 30/30 = 100.0%  
- **AIR**: 32 constraints, 30/30 cols covered (100.0%), avg **3.59 cols/constraint** (12.0% density)  
- **Lookup**: 3 constraints, 7/30 cols covered (23.3%), avg **2.33 cols/constraint** (7.8% density)  
- **Top-5 hottest**: `hit_count):  [17]=13  [16]=11  [0]=10  [15]=9  [1]=8`

### ShiftLeft (`SLL`)
- **Columns**: 42  
- **Total coverage**: 39/42 = 92.9%  
- **AIR**: 64 constraints, 39/42 cols covered (92.9%), avg **2.55 cols/constraint** (6.1% density)  
- **Lookup**: 0 constraints, 0/42 cols covered (0.0%), avg **0.00 cols/constraint** (0.0% density)  
- **Top-5 hottest**: `hit_count):  [28]=12  [12]=10  [13]=10  [14]=10  [37]=7`

### ShiftRight (`SRL`)
- **Columns**: 68  
- **Total coverage**: 64/68 = 94.1%  
- **AIR**: 81 constraints, 64/68 cols covered (94.1%), avg **3.41 cols/constraint** (5.0% density)  
- **Lookup**: 1 constraints, 2/68 cols covered (2.9%), avg **2.00 cols/constraint** (2.9% density)  
- **Top-5 hottest**: `hit_count):  [66]=19  [56]=18  [20]=11  [12]=10  [13]=10`

### MemoryReadWrite (`LW`)
- **Columns**: 95  
- **Total coverage**: 75/95 = 78.9%  
- **AIR**: 92 constraints, 67/95 cols covered (70.5%), avg **7.85 cols/constraint** (8.3% density)  
- **Lookup**: 2 constraints, 29/95 cols covered (30.5%), avg **15.50 cols/constraint** (16.3% density)  
- **Top-5 hottest**: `hit_count):  [58]=65  [56]=64  [59]=62  [60]=62  [57]=61`


---

## valida (BabyBear · Valida ISA)

| Table | Opcode | Total cols | Covered cols | Total coverage | AIR cols/c | Lookup cols/c | Top-5 hot columns |
|---|---|---:|---:|---:|---:|---:|---|
| Add32 | `ADD` | 16 | 15 | 93.8% | 3.20 | 0.00 | `hit_count):  [12]=4  [13]=4  [14]=3  [0]=2  [1]=2` |
| Sub32 | `SUB` | 17 | 16 | 94.1% | 2.88 | 0.00 | `hit_count):  [8]=3  [9]=3  [10]=3  [11]=2  [0]=1` |
| Bitwise32 | `AND` | 79 | 79 | 100.0% | 4.07 | 0.00 | `hit_count):  [76]=6  [77]=6  [78]=6  [0]=5  [1]=5` |
| Mul32 | `MUL32` | 27 | 24 | 88.9% | 7.14 | 3.00 | `hit_count):  [0]=4  [1]=4  [3]=4  [4]=4  [5]=4` |
| Div32 | `DIV` | 30 | 30 | 100.0% | 2.95 | 11.46 | `hit_count):  [29]=51  [28]=42  [7]=35  [4]=34  [5]=34` |
| Lt32 | `LT` | 45 | 44 | 97.8% | 3.42 | 0.00 | `hit_count):  [8]=14  [9]=12  [10]=11  [11]=10  [20]=10` |
| Com32 | `EQ` | 14 | 14 | 100.0% | 2.88 | 0.00 | `hit_count):  [10]=4  [8]=3  [12]=3  [13]=3  [0]=1` |
| Memory | `LOAD` | 27 | 27 | 100.0% | 2.67 | 0.00 | `hit_count):  [19]=14  [14]=13  [15]=11  [24]=11  [16]=9` |

### Add32 (`ADD`)
- **Columns**: 16  
- **Total coverage**: 15/16 = 93.8%  
- **AIR**: 10 constraints, 15/16 cols covered (93.8%), avg **3.20 cols/constraint** (20.0% density)  
- **Lookup**: 0 constraints, 0/16 cols covered (0.0%), avg **0.00 cols/constraint** (0.0% density)  
- **Top-5 hottest**: `hit_count):  [12]=4  [13]=4  [14]=3  [0]=2  [1]=2`

### Sub32 (`SUB`)
- **Columns**: 17  
- **Total coverage**: 16/17 = 94.1%  
- **AIR**: 8 constraints, 16/17 cols covered (94.1%), avg **2.88 cols/constraint** (16.9% density)  
- **Lookup**: 0 constraints, 0/17 cols covered (0.0%), avg **0.00 cols/constraint** (0.0% density)  
- **Top-5 hottest**: `hit_count):  [8]=3  [9]=3  [10]=3  [11]=2  [0]=1`

### Bitwise32 (`AND`)
- **Columns**: 79  
- **Total coverage**: 79/79 = 100.0%  
- **AIR**: 88 constraints, 79/79 cols covered (100.0%), avg **4.07 cols/constraint** (5.2% density)  
- **Lookup**: 0 constraints, 0/79 cols covered (0.0%), avg **0.00 cols/constraint** (0.0% density)  
- **Top-5 hottest**: `hit_count):  [76]=6  [77]=6  [78]=6  [0]=5  [1]=5`

### Mul32 (`MUL32`)
- **Columns**: 27  
- **Total coverage**: 24/27 = 88.9%  
- **AIR**: 7 constraints, 23/27 cols covered (85.2%), avg **7.14 cols/constraint** (26.4% density)  
- **Lookup**: 2 constraints, 5/27 cols covered (18.5%), avg **3.00 cols/constraint** (11.1% density)  
- **Top-5 hottest**: `hit_count):  [0]=4  [1]=4  [3]=4  [4]=4  [5]=4`

### Div32 (`DIV`)
- **Columns**: 30  
- **Total coverage**: 30/30 = 100.0%  
- **AIR**: 20 constraints, 14/30 cols covered (46.7%), avg **2.95 cols/constraint** (9.8% density)  
- **Lookup**: 39 constraints, 30/30 cols covered (100.0%), avg **11.46 cols/constraint** (38.2% density)  
- **Top-5 hottest**: `hit_count):  [29]=51  [28]=42  [7]=35  [4]=34  [5]=34`

### Lt32 (`LT`)
- **Columns**: 45  
- **Total coverage**: 44/45 = 97.8%  
- **AIR**: 60 constraints, 44/45 cols covered (97.8%), avg **3.42 cols/constraint** (7.6% density)  
- **Lookup**: 0 constraints, 0/45 cols covered (0.0%), avg **0.00 cols/constraint** (0.0% density)  
- **Top-5 hottest**: `hit_count):  [8]=14  [9]=12  [10]=11  [11]=10  [20]=10`

### Com32 (`EQ`)
- **Columns**: 14  
- **Total coverage**: 14/14 = 100.0%  
- **AIR**: 8 constraints, 14/14 cols covered (100.0%), avg **2.88 cols/constraint** (20.6% density)  
- **Lookup**: 0 constraints, 0/14 cols covered (0.0%), avg **0.00 cols/constraint** (0.0% density)  
- **Top-5 hottest**: `hit_count):  [10]=4  [8]=3  [12]=3  [13]=3  [0]=1`

### Memory (`LOAD`)
- **Columns**: 27  
- **Total coverage**: 27/27 = 100.0%  
- **AIR**: 42 constraints, 27/27 cols covered (100.0%), avg **2.67 cols/constraint** (9.9% density)  
- **Lookup**: 0 constraints, 0/27 cols covered (0.0%), avg **0.00 cols/constraint** (0.0% density)  
- **Top-5 hottest**: `hit_count):  [19]=14  [14]=13  [15]=11  [24]=11  [16]=9`


---

## Key Observations

- **Table sizes** range from 14 to 106 columns across all 6 zkVM backends.
- **Average AIR cols/constraint**: 3.40 (across all tables with AIR constraints)
- **Average Lookup cols/constraint**: 5.92
- **Most constraint-sparse AIR table**: DivRem (104 cols, 3.09 cols/constraint = **3.0% density**)
- **Densest AIR table**: Jal (15 cols, 6.00 cols/constraint = **40.0% density**)

### Per-constraint density by table (AIR only)

Density = avg cols touched per constraint ÷ total columns × 100%.

| VM | Table | Total cols | AIR cols/c | Density |
|---|---|---:|---:|---:|
| sphinx | DivRem | 104 | 3.09 | 3.0% █ |
| pico | DivRem | 96 | 2.98 | 3.1% █ |
| sp1 | DivRem | 98 | 4.25 | 4.3% █ |
| sphinx | ShiftRight | 71 | 3.35 | 4.7% █ |
| sp1 | ShiftRight | 70 | 3.44 | 4.9% █ |
| ziren | DivRem | 106 | 5.24 | 4.9% █ |
| pico | ShiftRight | 68 | 3.41 | 5.0% █ |
| valida | Bitwise32 | 79 | 4.07 | 5.2% █ |
| ziren | ShiftRight | 71 | 3.75 | 5.3% █ |
| sphinx | ShiftLeft | 45 | 2.50 | 5.6% █ |
| ziren | ShiftLeft | 44 | 2.55 | 5.8% █ |
| pico | ShiftLeft | 42 | 2.55 | 6.1% █ |
| sp1 | ShiftLeft | 44 | 2.78 | 6.3% █ |
| ziren | Jump | 66 | 4.23 | 6.4% █ |
| ziren | Mul | 58 | 4.00 | 6.9% █ |
| sphinx | Bitwise | 18 | 1.33 | 7.4% █ |
| ziren | MemoryReadWrite | 79 | 5.90 | 7.5% █ |
| valida | Lt32 | 45 | 3.42 | 7.6% █ |
| ziren | Branch | 62 | 4.84 | 7.8% █ |
| sp1 | MemoryReadWrite | 57 | 4.53 | 7.9% █ |
| pico | MemoryReadWrite | 95 | 7.85 | 8.3% █ |
| ziren | Bitwise | 18 | 1.60 | 8.9% █ |
| sphinx | Lt | 37 | 3.44 | 9.3% █ |
| valida | Div32 | 30 | 2.95 | 9.8% █ |
| valida | Memory | 27 | 2.67 | 9.9% █ |
| ziren | Lt | 36 | 3.59 | 10.0% █ |
| pico | Bitwise | 15 | 1.50 | 10.0% ██ |
| sphinx | Mul | 40 | 4.44 | 11.1% ██ |
| ziren | CloClz | 22 | 2.48 | 11.3% ██ |
| sp1 | Bitwise | 17 | 2.00 | 11.8% ██ |
| sp1 | Lt | 33 | 3.90 | 11.8% ██ |
| pico | Lt | 30 | 3.59 | 12.0% ██ |
| ziren | MovCond | 32 | 3.97 | 12.4% ██ |
| sp1 | Mul | 39 | 4.84 | 12.4% ██ |
| pico | Mul | 37 | 4.67 | 12.6% ██ |
| sp1 | Jump | 26 | 3.89 | 15.0% ██ |
| valida | Sub32 | 17 | 2.88 | 16.9% ███ |
| sp1 | AddSub | 19 | 3.72 | 19.6% ███ |
| valida | Add32 | 16 | 3.20 | 20.0% ████ |
| valida | Com32 | 14 | 2.88 | 20.6% ████ |
| sphinx | AddSub | 20 | 4.21 | 21.1% ████ |
| ziren | AddSub | 19 | 4.36 | 22.9% ████ |
| sp1 | Branch | 34 | 8.26 | 24.3% ████ |
| valida | Mul32 | 27 | 7.14 | 26.4% █████ |
| pico | AddSub | 17 | 4.59 | 27.0% █████ |
| openvm | Jal | 15 | 6.00 | 40.0% ████████ |
