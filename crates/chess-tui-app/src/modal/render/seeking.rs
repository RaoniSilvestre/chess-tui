//! Lichess seek-in-progress overlay.

use chess_tui_ui::components::centered_rect::centered_rect;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

pub fn render_seeking_popup(frame: &mut Frame<'_>, area: Rect) {
    let popup_area = centered_rect(60, 20, area);
    let block = Block::default()
        .title("Lichess")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray));
    let paragraph = Paragraph::new("Seeking a game on Lichess...\n(Press 'Esc' to cancel)")
        .block(block)
        .alignment(Alignment::Center);
    frame.render_widget(paragraph, popup_area);
}
