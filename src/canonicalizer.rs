use std::{collections::HashSet, fs};

use crate::interval::{AbstractInterval, MayBeFlag};
use crate::symbolic::GeneralLookupInfo;
use crate::trace::{trace_fmt_with_idxs, AbstractTrace};
use crate::ui::UiState;
use crate::utils::PrettySet;

pub fn save_repr_if_unique(
    record_reprs: &PrettySet<String>,
    known_reprt: &mut HashSet<String>,
    ui: &mut UiState,
) {
    if record_reprs.0.is_empty() {
        return;
    }

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

fn is_maybe_real_wo_pubval(
    row: &Vec<AbstractInterval>,
    info: &GeneralLookupInfo,
    p: u32,
    n: usize,
    i: usize,
) -> bool {
    let mut is_may_real = false;
    for gri in &info.is_real {
        is_may_real = gri
            .eval(&row, None, None, i == 0, i < n - 1, i == n - 1, p)
            .is_zero(p)
            != MayBeFlag::True;
        if is_may_real {
            break;
        }
    }
    is_may_real
}

pub fn generate_alu_final_checker(
    general_lookup_info: GeneralLookupInfo,
) -> impl Fn(&AbstractTrace, usize, u32, &mut HashSet<String>, &mut UiState, &mut i128) + Clone {
    move |trace: &AbstractTrace,
          _num_trial: usize,
          prime: u32,
          known_reprt: &mut HashSet<String>,
          ui: &mut UiState,
          area: &mut i128| {
        let mut record_reprs = HashSet::new();
        let n = trace.data.len();
        let mut this_area = 1;
        for (i, curr_row) in trace.data.iter().enumerate() {
            if is_maybe_real_wo_pubval(curr_row, &general_lookup_info, prime, n, i) {
                record_reprs.insert(format!(
                    "input0: [{}], input1: [{}], output: [{}]",
                    trace_fmt_with_idxs(trace, 0, &general_lookup_info.op_b),
                    trace_fmt_with_idxs(trace, 0, &general_lookup_info.op_c),
                    trace_fmt_with_idxs(trace, 0, &general_lookup_info.op_a),
                ));
                this_area *= trace.area(&general_lookup_info.op_b);
                this_area *= trace.area(&general_lookup_info.op_c);
                this_area *= trace.area(&general_lookup_info.op_a);
            }
        }
        let string_representation = format!("{}", PrettySet(record_reprs.clone()));
        if !known_reprt.contains(&string_representation) {
            *area += this_area;
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
    general_lookup_info: GeneralLookupInfo,
) -> impl Fn(&AbstractTrace, usize, u32, &mut HashSet<String>, &mut UiState, &mut i128) + Clone {
    move |trace: &AbstractTrace,
          _num_trial: usize,
          prime: u32,
          known_reprt: &mut HashSet<String>,
          ui: &mut UiState,
          _area: &mut i128| {
        let mut record_reprs = HashSet::new();
        let n = trace.data.len();
        for i in 0..trace.data.len() {
            if is_maybe_real_wo_pubval(&trace.data[i], &general_lookup_info, prime, n, i) {
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
