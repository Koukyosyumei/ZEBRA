use std::{fs, path::Path};

use crate::symbolic::{AbstractTrace, LatticeVMSymbolicExpr};

#[derive(Debug)]
pub struct BitCombinationsDictOrder {
    len: usize,
    current: usize,
}

impl BitCombinationsDictOrder {
    pub fn new(length: usize) -> Self {
        assert!(
            length <= usize::BITS as usize,
            "Length exceeds available bits"
        );
        BitCombinationsDictOrder {
            len: length,
            current: 0,
        }
    }
}

impl Iterator for BitCombinationsDictOrder {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        let total_combinations = 1_usize.checked_shl(self.len as u32)?;
        if self.current >= total_combinations {
            return None;
        }

        let n = self.current;
        let mut combination = Vec::with_capacity(self.len);
        for i in (0..self.len).rev() {
            let bit = (n >> i) & 1;
            combination.push(bit as u8);
        }

        self.current += 1;
        Some(combination)
    }
}

pub fn create_or_clear_dir(path: &str) -> std::io::Result<()> {
    let p = Path::new(path);

    if p.exists() {
        fs::remove_dir_all(p)?;
    }

    fs::create_dir_all(p)?;
    Ok(())
}

#[derive(Default, Debug, Clone)]
pub struct GeneralLookupInfo {
    pub alu_output: Vec<usize>,
    pub alu_input1: Vec<usize>,
    pub alu_input2: Vec<usize>,
    pub pc_table_is_real: LatticeVMSymbolicExpr,
}

pub const fn indices_arr<const N: usize>() -> [usize; N] {
    let mut indices_arr = [0; N];
    let mut i = 0;
    while i < N {
        indices_arr[i] = i;
        i += 1;
    }
    indices_arr
}

pub fn trace_fmt_with_idxs(trace: &AbstractTrace, i: usize, idxs: &[usize]) -> String {
    idxs.iter()
        .map(|&j| trace.data[i][j].to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
