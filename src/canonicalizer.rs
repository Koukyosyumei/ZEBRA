use std::{collections::HashSet, fs};

use crate::interval::MayBeFlag;
use crate::symbolic::GeneralLookupInfo;
use crate::trace::{trace_fmt_with_idxs, AbstractTrace};
use crate::ui::UiState;
use crate::utils::PrettySet;

pub fn save_repr_if_unique(
    record_reprs: &PrettySet<String>,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    let string_representation = format!("{}", record_reprs);
    if !known_reprt.contains(&string_representation) {
        known_reprt.insert(string_representation.clone());
        ui.recovered = string_representation.clone();

        fs::write(
            format!("voutput/{}_states.txt", known_reprt.len()),
            ui.recovered.clone(),
        )
        .unwrap();
        fs::write(
            format!("voutput/{}_assignments.txt", known_reprt.len()),
            ui.logs.clone(),
        )
        .unwrap();
    }
}

pub fn generate_alu_final_checker(
    general_lookup_info: GeneralLookupInfo,
) -> impl Fn(&AbstractTrace, usize, u32, &mut HashSet<String>, &mut UiState) + Clone {
    move |trace: &AbstractTrace,
          _num_trial: usize,
          prime: u32,
          known_reprt: &mut HashSet<String>,
          ui: &mut UiState| {
        let mut record_reprs = HashSet::new();
        let n = trace.data.len();
        for (i, curr_row) in trace.data.iter().enumerate() {
            let is_may_real = general_lookup_info
                .pc_table_is_real
                .eval(&curr_row, None, None, i == 0, i < n - 1, i == n - 1, prime)
                .is_zero(prime)
                != MayBeFlag::False;
            if is_may_real {
                record_reprs.insert(format!(
                    "input0: [{}], input1: [{}], output: [{}]",
                    trace_fmt_with_idxs(trace, 0, &general_lookup_info.op_b),
                    trace_fmt_with_idxs(trace, 0, &general_lookup_info.op_c),
                    trace_fmt_with_idxs(trace, 0, &general_lookup_info.op_a),
                ));
            }
        }

        save_repr_if_unique(&PrettySet(record_reprs), known_reprt, ui);
    }
}

pub fn generate_memory_op_final_checker(
    clk_column: usize,
    op_a_columns: Vec<usize>,
    op_b_columns: Vec<usize>,
    op_c_columns: Vec<usize>,
    memory_columns: Vec<usize>,
    is_real: usize,
    prime: u32,
) -> impl Fn(&AbstractTrace, usize, u32, &mut HashSet<String>, &mut UiState) + Clone {
    move |trace: &AbstractTrace,
          _num_trial: usize,
          _prime: u32,
          known_reprt: &mut HashSet<String>,
          ui: &mut UiState| {
        let mut record_reprs = HashSet::new();
        for i in 0..trace.data.len() {
            if trace.data[i][is_real].is_zero(prime) != MayBeFlag::False {
                let row_string_representation = format!(
                "clk: {}\nop_a_access: [{}]\nop_b_access: [{}]\nop_c_access: [{}]\nmem_access: [{}]\n--------------\n",
                trace.data[i][clk_column],
                trace_fmt_with_idxs(trace, i, &op_a_columns),
                trace_fmt_with_idxs(trace, i, &op_b_columns),
                trace_fmt_with_idxs(trace, i, &op_c_columns),
                trace_fmt_with_idxs(trace, i, &memory_columns),
            );
                record_reprs.insert(row_string_representation);
            }
        }
        save_repr_if_unique(&PrettySet(record_reprs), known_reprt, ui);
    }
}
