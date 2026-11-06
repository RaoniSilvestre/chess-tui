//! Tracks the active page, current popup overlay, menu cursor position, and transient UI flags.

use crate::GamePageKind;

/// Navigation and overlay state held by [`crate::App`].
///
/// Tracks which page and popup are currently rendered, the shared `menu_cursor`
/// used by all list-based pages, and transient flags like `end_screen_dismissed`
/// that coordinate popup visibility across ticks.
#[derive(Default)]
pub struct UIState {
    /// Whether the game-over screen is currently visible.
    pub end_screen_visible: bool,
    /// Track if the end screen was dismissed by the user (to prevent re-showing)
    pub end_screen_dismissed: bool,
    /// menu current cursor
    pub menu_cursor: u8,
    /// What the new-game confirmation modal starts once the user says yes
    pub pending_new_game: Option<NewGame>,
}

/// A new game that would discard a resume save, held until the user confirms it.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum NewGame {
    /// From the game-mode form: start a game page.
    Start(GamePageKind),
    /// From a game in progress: restart it.
    Restart,
}

impl UIState {
    /// Resets the shared menu cursor when returning home.
    pub fn navigate_to_homepage(&mut self) {
        self.menu_cursor = 0;
    }
}
