//! UI color helpers.

use ratatui::style::Color;

/// Converts a shakmaty piece color to the corresponding ratatui [`Color`].
/// Returns `Color::Red` when the square is empty (`None`).
#[must_use]
pub fn color_to_ratatui_enum(piece_color: Option<shakmaty::Color>) -> Color {
    match piece_color {
        Some(shakmaty::Color::Black) => Color::Black,
        Some(shakmaty::Color::White) => Color::White,
        None => Color::Red,
    }
}
