//! Shared constants and navigation enums.

use std::path::PathBuf;

pub use crate::game_mode::{
    BOT_DIFFICULTY_COUNT, BOT_DIFFICULTY_NAMES, TIME_CONTROL_CUSTOM_INDEX, TIME_CONTROL_OPTIONS,
};
use ratatui::style::Color;
use throbber_widgets_tui::Set;

pub const CHESS_SET: Set = Set {
    full: "♚",
    empty: " ",
    symbols: &["♚", "♛", "♜", "♝", "♞", "♟", "♔", "♕", "♖", "♗", "♘", "♙"],
};

/// Sentinel value meaning "no board position set" (used before a square is chosen).
pub const UNDEFINED_POSITION: u8 = u8::MAX;
/// Light square / piece color used throughout the board renderer.
pub const WHITE: Color = Color::Rgb(160, 160, 160);
/// Dark square / piece color used throughout the board renderer.
pub const BLACK: Color = Color::Rgb(128, 95, 69);

/// Wait after sending a resign to Lichess before continuing.
pub const SLEEP_DURATION_RESIGN_MS: u64 = 500;
/// Pause after showing puzzle solution before proceeding.
pub const SLEEP_DURATION_PUZZLE_MS: u64 = 1500;

/// Seconds in one day, used for converting timestamps to day units.
pub const SECONDS_PER_DAY: f64 = 86400.0;
/// Number of days of rating history shown in the chart.
pub const RATING_HISTORY_DAYS: i64 = 90;

/// Name of the built-in default skin entry.
pub const SKIN_NAME_DEFAULT: &str = "Default";
/// Name of the built-in ASCII skin entry.
pub const SKIN_NAME_ASCII: &str = "ASCII";

pub use chess_tui_lichess::{LICHESS_TIME_CONTROLS, LichessTimeControl};

/// Default selected time control index (Rapid).
pub const DEFAULT_TIME_CONTROL_SELECTED: u8 = 3;
/// Default duration in minutes when "Custom" time control is selected.
pub const DEFAULT_CUSTOM_TIME_VALUE: u16 = 10;

/// ASCII-art banner rendered on the home screen.
pub const TITLE: &str = r"
 ██████╗██╗  ██╗███████╗███████╗███████╗   ████████╗██╗   ██╗██╗
██╔════╝██║  ██║██╔════╝██╔════╝██╔════╝   ╚══██╔══╝██║   ██║██║
██║     ███████║█████╗  ███████╗███████╗█████╗██║   ██║   ██║██║
██║     ██╔══██║██╔══╝  ╚════██║╚════██║╚════╝██║   ██║   ██║██║
╚██████╗██║  ██║███████╗███████║███████║      ██║   ╚██████╔╝██║
 ╚═════╝╚═╝  ╚═╝╚══════╝╚══════╝╚══════╝      ╚═╝    ╚═════╝ ╚═╝
";

/// Returns the user's config directory path.
///
/// # Errors
///
/// Returns an error if the config directory cannot be determined.
pub fn config_dir() -> Result<PathBuf, &'static str> {
    match dirs::config_dir() {
        Some(dir) => Ok(dir),
        None => Err("Could not get config directory"),
    }
}

/// Base URL for all Lichess REST API requests.
pub const LICHESS_API_URL: &str = "https://lichess.org/api";
/// Base URL for the chess-tui documentation.
pub const DOCS_URL: &str = "https://thomas-mauran.github.io/chess-tui/docs";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lichess_time_controls_use_standard_notation() {
        assert_eq!(LICHESS_TIME_CONTROLS[0].notation(), "10+0");
        assert_eq!(LICHESS_TIME_CONTROLS[0].label(), "10+0 Rapid");
        assert_eq!(LICHESS_TIME_CONTROLS[1].notation(), "30+0");
        assert_eq!(LICHESS_TIME_CONTROLS[1].label(), "30+0 Classical");
    }
}
