//! Resignation confirmation popup.

use crate::constants::WHITE as board_white;
use chess_tui_ui::components::centered_rect::centered_rect;
use ratatui::{
    Frame,
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap},
};

/// Renders a "Yes / No" confirmation dialog before resigning the current game.
pub fn render_resign_confirmation_popup(frame: &mut Frame, opponent_name: Option<&str>) {
    let block = Block::default()
        .title("Resign Game")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .padding(Padding::horizontal(1))
        .border_style(Style::default().fg(board_white));
    let area = centered_rect(50, 30, frame.area());

    let opponent_name = opponent_name
        .map(|name| format!("vs {name}"))
        .unwrap_or_else(|| "this game".to_string());

    let text = vec![
        Line::from(""),
        Line::from(format!(
            "Are you sure you want to resign {}?",
            opponent_name
        ))
        .alignment(Alignment::Center),
        Line::from(""),
        Line::from("This action cannot be undone.").alignment(Alignment::Center),
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Y",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("es / "),
            Span::styled(
                "N",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw("o"),
        ])
        .alignment(Alignment::Center),
    ];

    let paragraph = Paragraph::new(text)
        .block(block.clone())
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    frame.render_widget(Clear, area); //this clears out the background
    frame.render_widget(block, area);
    frame.render_widget(paragraph, area);
}
