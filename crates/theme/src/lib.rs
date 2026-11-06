//! Skins, piece styles, and display modes.

mod skin;

pub use skin::{PieceStyle, PieceStyleCollection, PieceStyleSize, Skin, SkinCollection};

use core::fmt;

/// Piece rendering style: built-in Unicode, plain ASCII, or a custom skin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplayMode {
    /// Built-in Unicode piece set.
    DEFAULT,
    /// Plain ASCII characters.
    ASCII,
    /// User-defined skin loaded from `skins.json`.
    CUSTOM,
}

impl fmt::Display for DisplayMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DisplayMode::ASCII => write!(f, "ASCII"),
            DisplayMode::DEFAULT => write!(f, "DEFAULT"),
            DisplayMode::CUSTOM => write!(f, "CUSTOM"),
        }
    }
}
