//! Credits page.

use app::{App, Nav, Page};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap},
};
use ui::components::centered_rect::centered_rect;

pub struct CreditsPage;

impl CreditsPage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CreditsPage {
    fn default() -> Self {
        Self::new()
    }
}

impl Page for CreditsPage {
    fn handle_key(&mut self, key: KeyEvent, app: &mut App) -> Nav {
        match key.code {
            KeyCode::Char(' ') | KeyCode::Esc | KeyCode::Enter => app.go_home(),
            KeyCode::Char('q') => app.quit(),
            _ => {}
        }
        Nav::None
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, _app: &mut App) {
        render_credits(frame, area);
    }
}

fn render_credits(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(40, 40, area);
    let block = Block::default()
        .title("Credits")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .padding(Padding::horizontal(1))
        .border_style(Style::default().fg(Color::Rgb(160, 160, 160)));

    let credits_text = vec![
        Line::from(""),
        Line::from("Thank you for playing chess-tui!"),
        Line::from(""),
        Line::from(
            "chess-tui is an open source chess game for the terminal, written in Rust and built by its community of contributors.",
        ),
        Line::from(""),
        Line::from("Source code: https://github.com/thomas-mauran/chess-tui"),
        Line::from(""),
        Line::from(
            "Contributions are welcome: open an issue, submit a pull request, or star the repository to support the project.",
        ),
        Line::from(""),
        Line::from("Thanks to everyone who has contributed, reported bugs, or shared feedback."),
        Line::from(""),
        Line::from(""),
        Line::from("Press `Esc` to close the popup.").alignment(Alignment::Center),
    ];

    let paragraph = Paragraph::new(credits_text)
        .block(block)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph, popup_area);
}
