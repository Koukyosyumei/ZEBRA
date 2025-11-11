use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub struct UiState {
    pub status: String,
    pub logs: String,
    pub program: String,
    pub recovered: String,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            status: String::new(),
            logs: String::new(),
            program: String::new(),
            recovered: String::new(),
        }
    }

    pub fn render<B: ratatui::backend::Backend>(&self, f: &mut Frame) {
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(f.size());

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
            .wrap(wrap);

        let top_right = Paragraph::new(self.program.clone())
            .block(Block::default().borders(Borders::ALL).title("Program"))
            .style(Style::default().fg(Color::Yellow))
            .wrap(wrap);

        let bottom_left = Paragraph::new(self.logs.clone())
            .block(Block::default().borders(Borders::ALL).title("Logs"))
            .style(Style::default().fg(Color::White))
            .wrap(wrap);

        let bottom_right = Paragraph::new(self.logs.clone())
            .block(Block::default().borders(Borders::ALL).title("Debug"))
            .style(Style::default().fg(Color::Blue))
            .wrap(wrap);

        // それぞれ描画
        f.render_widget(top_left, top_chunks[0]);
        f.render_widget(top_right, top_chunks[1]);
        f.render_widget(bottom_left, bottom_chunks[0]);
        f.render_widget(bottom_right, bottom_chunks[1]);
    }
}
