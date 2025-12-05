use latticevm::solver::make_init_val;
use latticevm::solver::RangeType;
use std::collections::HashMap;

use p3_field::{AbstractField, PrimeField32};
//use p3_uni_stark::symbolic_builder::get_symbolic_constraints;

use latticevm::interval::AbstractInterval;

pub fn derive_add_table(
    cpu_main_trace: &Vec<Vec<AbstractInterval>>,
    range_types: &HashMap<usize, RangeType>,
    prime: u32,
) -> Vec<Vec<AbstractInterval>> {
    let mut out = vec![];
    for row in cpu_main_trace {
        if row[58].as_canonical_u32(prime) != 0 && row[3].as_canonical_u32(prime) == 100 {
            let mut r: Vec<_> = (0..16).map(|_| AbstractInterval::top(prime)).collect();
            for (k, v) in range_types {
                r[*k] = make_init_val(*k, &range_types, prime);
            }

            let cpu_columns = vec![32, 33, 34, 35, 38, 39, 40, 41, 44, 45, 46, 47];
            let add_columns = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
            for i in 0..(cpu_columns.len()) {
                r[add_columns[i]] = row[cpu_columns[i]].clone();
            }
            r[15] = AbstractInterval::one();
            out.push(r);
        }
    }

    out
}

pub fn derive_sub_table(
    cpu_main_trace: &Vec<Vec<AbstractInterval>>,
    range_types: &HashMap<usize, RangeType>,
    prime: u32,
) -> Vec<Vec<AbstractInterval>> {
    let mut out = vec![];
    for row in cpu_main_trace {
        if row[58].as_canonical_u32(prime) != 0 && row[3].as_canonical_u32(prime) == 101 {
            let mut r: Vec<_> = (0..17).map(|_| AbstractInterval::top(prime)).collect();
            for (k, v) in range_types {
                r[*k] = make_init_val(*k, &range_types, prime);
            }

            let cpu_columns = vec![32, 33, 34, 35, 38, 39, 40, 41, 44, 45, 46, 47];
            let sub_columns = vec![0, 1, 2, 3, 4, 5, 6, 7, 12, 13, 14, 15];
            for i in 0..(cpu_columns.len()) {
                r[sub_columns[i]] = row[cpu_columns[i]].clone();
            }
            r[16] = AbstractInterval::one();
            out.push(r);
        }
    }

    out
}

pub fn derive_com_table(
    cpu_main_trace: &Vec<Vec<AbstractInterval>>,
    range_types: &HashMap<usize, RangeType>,
    prime: u32,
) -> Vec<Vec<AbstractInterval>> {
    let mut out = vec![];
    for row in cpu_main_trace {
        if row[58].as_canonical_u32(prime) != 0
            && (row[3].as_canonical_u32(prime) == 116 || row[3].as_canonical_u32(prime) == 111)
        {
            let mut r: Vec<_> = (0..14).map(|_| AbstractInterval::i4()).collect();
            for (k, v) in range_types {
                r[*k] = make_init_val(*k, &range_types, prime);
            }

            let cpu_columns = vec![32, 33, 34, 35, 38, 39, 40, 41, 44];
            let com_columns = vec![0, 1, 2, 3, 4, 5, 6, 7, 11];
            for i in 0..(cpu_columns.len()) {
                r[com_columns[i]] = row[cpu_columns[i]].clone();
            }
            if row[3].as_canonical_u32(prime) == 116 {
                r[13] = AbstractInterval::one();
                r[12] = AbstractInterval::zero();
            } else {
                r[13] = AbstractInterval::zero();
                r[12] = AbstractInterval::one();
            }
            out.push(r);
        }
    }

    out
}
