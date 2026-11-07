//! Key-bindings help overlay, context-sensitive per active page.

use super::pgn_help::render_pgn_help_popup;
use crate::components::centered_rect::centered_rect;
use ratatui::{
    Frame,
    layout::Alignment,
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap},
};

/// Renders the help popup, delegating to the PGN-specific variant when in the PGN viewer.
#[derive(Clone, Copy, Debug, Default)]
pub struct HelpContext {
    pub is_pgn: bool,
    pub is_solo: bool,
    pub is_puzzle: bool,
}

pub fn render_help_popup(frame: &mut Frame, context: HelpContext) {
    let is_solo_mode = context.is_solo;
    let is_puzzle_mode = context.is_puzzle;
    if context.is_pgn {
        render_pgn_help_popup(frame);
        return;
    }

    let block = Block::default()
        .title("Help menu")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .padding(Padding::horizontal(1))
        .border_style(Style::default().fg(crate::color::board_white()));
    let area = centered_rect(40, 65, frame.area());

    let mut text = vec![
        Line::from("Game controls:".underlined().bold()),
        Line::from(""),
    ];

    // In puzzle mode, 'h' is for hint, not left movement
    if is_puzzle_mode {
        text.push(Line::from(vec![
            "← ↑/k ↓/j →/l: Use these keys or the mouse to move the ".into(),
            "blue".blue(),
            " cursor".into(),
        ]));
        text.push(Line::from(""));
        text.push(Line::from(
            "T: Show hint (select the piece to move)".yellow(),
        ));
        text.push(Line::from(""));
    } else {
        text.push(Line::from(vec![
            "←/h ↑/k ↓/j →/l: Use these keys or the mouse to move the ".into(),
            "blue".blue(),
            " cursor".into(),
        ]));
        text.push(Line::from(""));
    }

    text.extend(vec![
        Line::from("`Ctrl` '+' or '-': Zoom in or out to adjust pieces sizes"),
        Line::from("(Might differ in certain terminals)"),
        Line::from(""),
        Line::from("`Space`: Select a piece"),
        Line::from(""),
        Line::from("`Esc`: Deselect a piece / hide popups"),
        Line::from(""),
        Line::from("q: Quit the game"),
        Line::from(""),
        Line::from("b: Go to the home menu / reset the game"),
        Line::from(""),
        Line::from("s: Cycle through available skins"),
        Line::from(""),
        Line::from("m: Type a move in chess notation (e.g. e4, Nf3, O-O)"),
        Line::from(""),
    ]);

    // Only show history navigation controls in solo mode (not against bot or puzzle)
    if is_solo_mode && !is_puzzle_mode {
        text.push(Line::from("P: Navigate to previous position in history"));
        text.push(Line::from(""));
        text.push(Line::from("N: Navigate to next position in history"));
        text.push(Line::from(""));
    }

    text.extend(vec![
        Line::from(""),
        Line::from("Color codes:".underlined().bold()),
        Line::from(""),
        Line::from(vec!["Blue cell".blue(), ": Your cursor ".into()]),
        Line::from(""),
        Line::from(vec!["Green cell".green(), ": Selected Piece ".into()]),
        Line::from(""),
        Line::from(vec![
            "Purple cell".magenta(),
            ": The king is getting checked ".into(),
        ]),
        Line::from(""),
        Line::from("Grey cell: Available cells for the selected piece"),
        Line::from(""),
        Line::from(""),
        Line::from("Press `Esc` to close the popup.").alignment(Alignment::Center),
    ]);

    let text = text;

    let paragraph = Paragraph::new(text)
        .block(block.clone())
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    frame.render_widget(Clear, area); //this clears out the background
    frame.render_widget(block, area);
    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn help_popup_lists_m_shortcut() {
        let backend = TestBackend::new(120, 80);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| {
                render_help_popup(
                    frame,
                    HelpContext {
                        is_solo: true,
                        ..HelpContext::default()
                    },
                );
            })
            .expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let mut rendered = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                rendered.push_str(buffer[(x, y)].symbol());
            }
            rendered.push('\n');
        }

        assert!(
            rendered.contains("Type a move in chess notation"),
            "Help popup should mention the 'm' shortcut.\nRendered:\n{rendered}"
        );
    }
}
