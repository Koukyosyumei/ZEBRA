use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct UiState {
    pub status: String,
    pub logs: String,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            status: String::new(),
            logs: String::new(),
        }
    }

    pub fn render<B: ratatui::backend::Backend>(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
            .split(f.size());

        let top = Paragraph::new(self.status.clone())
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .style(Style::default().fg(Color::Green));

        let bottom = Paragraph::new(self.logs.clone())
            .block(Block::default().borders(Borders::ALL).title("Logs"))
            .style(Style::default().fg(Color::White));

        f.render_widget(top, chunks[0]);
        f.render_widget(bottom, chunks[1]);
    }
}
