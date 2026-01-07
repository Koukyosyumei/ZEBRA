use std::collections::HashSet;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::{symbolic::AbstractTrace, utils::GeneralLookupInfo};

pub struct UiState {
    pub status: String,
    pub logs: String,
    pub program: String,
    pub recovered: String,

    pub scroll_status: u16,
    pub scroll_logs: u16,
    pub scroll_program: u16,
    pub scroll_recovered: u16,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            status: String::new(),
            logs: String::new(),
            program: String::new(),
            recovered: String::new(),

            scroll_status: 0,
            scroll_logs: 0,
            scroll_program: 0,
            scroll_recovered: 0,
        }
    }

    pub fn render<B: ratatui::backend::Backend>(&self, f: &mut Frame) {
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(f.area());

        let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
            .split(vertical_chunks[0]);

        let bottom_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(vertical_chunks[1]);

        let wrap = Wrap { trim: true };

        let top_left = Paragraph::new(self.status.clone())
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .style(Style::default().fg(Color::Green))
            .wrap(wrap)
            .scroll((self.scroll_status, 0));

        let top_right = Paragraph::new(self.program.clone())
            .block(Block::default().borders(Borders::ALL).title("Program"))
            .style(Style::default().fg(Color::Yellow))
            .wrap(wrap)
            .scroll((self.scroll_program, 0));

        let bottom_left = Paragraph::new(self.logs.clone())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("SAT Assignment"),
            )
            .style(Style::default().fg(Color::Cyan))
            .wrap(wrap)
            .scroll((self.scroll_logs, 0));

        let bottom_right = Paragraph::new(self.recovered.clone())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Canonical Representation"),
            )
            .style(Style::default().fg(Color::Magenta))
            .wrap(wrap)
            .scroll((self.scroll_recovered, 0));

        f.render_widget(top_left, top_chunks[0]);
        f.render_widget(top_right, top_chunks[1]);
        f.render_widget(bottom_left, bottom_chunks[0]);
        f.render_widget(bottom_right, bottom_chunks[1]);
    }
}

pub fn generate_alu_final_checker(
    general_lookup_info: GeneralLookupInfo,
) -> impl Fn(&AbstractTrace, usize, u32, &mut HashSet<String>, &mut UiState) + Clone {
    move |trace: &AbstractTrace,
          _num_trial: usize,
          _prime: u32,
          known_reprt: &mut HashSet<String>,
          ui: &mut UiState| {
        let i1 = &general_lookup_info.alu_input1;
        let i2 = &general_lookup_info.alu_input2;
        let o = &general_lookup_info.alu_output;

        debug_assert!(i1.len() == 4);
        debug_assert!(i2.len() == 4);
        debug_assert!(o.len() == 4);

        let row = &trace.data[0];

        let string_representation = format!(
            "input0: [{}, {}, {}, {}], input1: [{}, {}, {}, {}], output: [{}, {}, {}, {}]",
            row[i1[0]],
            row[i1[1]],
            row[i1[2]],
            row[i1[3]],
            row[i2[0]],
            row[i2[1]],
            row[i2[2]],
            row[i2[3]],
            row[o[0]],
            row[o[1]],
            row[o[2]],
            row[o[3]],
        );

        if known_reprt.insert(string_representation.clone()) {
            ui.recovered = string_representation;

            std::fs::write(
                format!("voutput/{}_states.txt", known_reprt.len()),
                ui.recovered.clone(),
            )
            .unwrap();

            std::fs::write(
                format!("voutput/{}_assignments.txt", known_reprt.len()),
                ui.logs.clone(),
            )
            .unwrap();
        }
    }
}
