//! Text input widget.

#[derive(Clone, Default)]
pub struct Prompt {
    /// Current value of the input box.
    pub input: String,
    /// Cursor position measured in characters (not bytes).
    pub character_index: usize,
    /// The last submitted value, set by [`Prompt::submit_message`].
    pub message: String,
    /// Inline validation error to display alongside the input.
    pub error: Option<String>,
}

impl Prompt {
    /// Creates a new empty `Prompt` with the cursor at position 0.
    pub fn new() -> Self {
        Self {
            input: "".to_string(),
            character_index: 0,
            message: String::new(),
            error: None,
        }
    }

    /// Moves the cursor one character to the left, clamped at the start.
    pub fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    /// Moves the cursor one character to the right, clamped at the end of input.
    pub fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    /// Clamps `new_cursor_pos` to `[0, input.len()]` in characters.
    pub fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    /// Resets the cursor to position 0 without clearing the input.
    pub fn reset_cursor(&mut self) {
        self.character_index = 0;
    }

    /// Clears `input`, `message`, and any pending `error`, and resets the cursor.
    pub fn reset(&mut self) {
        self.input.clear();
        self.message.clear();
        self.error = None;
        self.reset_cursor();
    }

    /// Copies `input` into `message`, then clears `input` and resets the cursor.
    pub fn submit_message(&mut self) {
        self.message = self.input.clone();
        self.input.clear();
        self.reset_cursor();
    }

    /// Inserts `new_char` at the cursor position if the input is under 200 bytes.
    pub fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        // Limit accommodates long API tokens (Lichess tokens can be 100+ chars).
        if index < 200 {
            self.input.insert(index, new_char);
            self.move_cursor_right();
            self.error = None;
        }
    }

    /// Returns the byte offset of the current cursor position.
    ///
    /// Each Unicode scalar can occupy multiple bytes, so the character index
    /// cannot be used directly as a byte index into the backing `String`.
    pub fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    /// Deletes the character immediately to the left of the cursor.
    ///
    /// Uses char-boundary-safe reconstruction instead of `String::remove` to
    /// avoid panicking on multi-byte characters.
    pub fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            let after_char_to_delete = self.input.chars().skip(current_index);

            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
            self.error = None;
        }
    }
}

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Position};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap};

use crate::components::right_rect::right_rect;

pub enum PromptResult {
    None,
    Submit,
    Cancel,
}

/// Routes editing keys into a prompt. Enter/Esc are returned to the caller so
/// it can decide what submitting/cancelling means for its page.
pub fn handle_key(prompt: &mut Prompt, key: KeyEvent) -> PromptResult {
    match key.code {
        KeyCode::Enter => PromptResult::Submit,
        KeyCode::Esc => PromptResult::Cancel,
        KeyCode::Char(to_insert) => {
            prompt.enter_char(to_insert);
            PromptResult::None
        }
        KeyCode::Backspace => {
            prompt.delete_char();
            PromptResult::None
        }
        KeyCode::Left => {
            prompt.move_cursor_left();
            PromptResult::None
        }
        KeyCode::Right => {
            prompt.move_cursor_right();
            PromptResult::None
        }
        _ => PromptResult::None,
    }
}

/// Renders a right-anchored text input box for typing a move in SAN notation.
pub fn render_move_input_popup(frame: &mut Frame, prompt: &Prompt) {
    let block = Block::default()
        .title("Input a move")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .padding(Padding::horizontal(1))
        .border_style(Style::default().fg(crate::color::board_white()));
    let area = right_rect(23, 40, frame.area());

    let current_input = prompt.input.as_str();
    let mut text = vec![
        Line::from("Enter a move in chess notation").alignment(Alignment::Center),
        Line::from(""),
        Line::from(current_input),
        Line::from(""),
    ];
    if let Some(err) = prompt.error.as_deref() {
        text.push(Line::from(err).style(Style::default().fg(Color::Red)));
        text.push(Line::from(""));
    }
    text.extend([
        Line::from(
            "Visit this website for more information: https://www.chess.com/terms/chess-notation",
        ),
        Line::from(""),
        Line::from("Press `Esc` to close the popup.").alignment(Alignment::Center),
    ]);

    let paragraph = Paragraph::new(text)
        .block(block.clone())
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    frame.set_cursor_position(Position::new(
        area.x + prompt.character_index as u16 + 2,
        area.y + 3,
    ));

    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_default_is_none() {
        assert!(Prompt::default().error.is_none());
        assert!(Prompt::new().error.is_none());
    }

    #[test]
    fn enter_char_clears_error() {
        let mut p = Prompt::new();
        p.error = Some("boom".into());
        p.enter_char('a');
        assert!(p.error.is_none());
    }

    #[test]
    fn delete_char_clears_error_when_deleting() {
        let mut p = Prompt::new();
        p.enter_char('a');
        p.error = Some("boom".into());
        p.delete_char();
        assert!(p.error.is_none());
    }

    #[test]
    fn delete_char_at_start_does_not_clear_error() {
        let mut p = Prompt::new();
        p.error = Some("boom".into());
        // Cursor at 0 with empty input: delete_char is a no-op, error must remain.
        p.delete_char();
        assert_eq!(p.error.as_deref(), Some("boom"));
    }

    #[test]
    fn reset_clears_error() {
        let mut p = Prompt::new();
        p.error = Some("boom".into());
        p.reset();
        assert!(p.error.is_none());
    }
}
