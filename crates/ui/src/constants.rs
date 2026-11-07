//! Shared presentation constants.

use ratatui::style::Color;
use throbber_widgets_tui::Set;

pub const CHESS_SET: Set = Set {
    full: "♚",
    empty: " ",
    symbols: &["♚", "♛", "♜", "♝", "♞", "♟", "♔", "♕", "♖", "♗", "♘", "♙"],
};

pub const WHITE: Color = Color::Rgb(160, 160, 160);
pub const BLACK: Color = Color::Rgb(128, 95, 69);
