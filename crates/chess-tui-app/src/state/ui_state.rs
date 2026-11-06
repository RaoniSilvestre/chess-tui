//! Tracks the active page, current popup overlay, menu cursor position, and transient UI flags.

use crate::GamePageKind;
use crate::constants::Popups;

/// Navigation and overlay state held by [`crate::App`].
///
/// Tracks which page and popup are currently rendered, the shared `menu_cursor`
/// used by all list-based pages, and transient flags like `end_screen_dismissed`
/// that coordinate popup visibility across ticks.
#[derive(Default)]
pub struct UIState {
    /// Current popup to render
    pub current_popup: Option<Popups>,
    /// Error message for Error popup
    pub popup_message: Option<String>,
    /// Track if the end screen was dismissed by the user (to prevent re-showing)
    pub end_screen_dismissed: bool,
    /// menu current cursor
    pub menu_cursor: u8,
    /// What `Popups::NewGameConfirmation` starts once the user says yes
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
    /// Opens the help popup if it is not showing, closes it if it is.
    pub fn toggle_help_popup(&mut self) {
        if self.current_popup == Some(Popups::Help) {
            self.close_popup();
        } else {
            self.current_popup = Some(Popups::Help);
        }
    }

    /// Clears the active popup and its message.
    pub fn close_popup(&mut self) {
        self.current_popup = None;
        self.popup_message = None;
    }
    /// Resets the shared menu cursor when returning home.
    pub fn navigate_to_homepage(&mut self) {
        self.menu_cursor = 0;
    }

    /// Opens a popup without attaching a message.
    pub fn show_popup(&mut self, popup_type: Popups) {
        self.current_popup = Some(popup_type);
    }

    /// Opens a popup and stores `message` so the UI can display it.
    pub fn show_message_popup(&mut self, message: String, popup_type: Popups) {
        self.popup_message = Some(message);
        self.current_popup = Some(popup_type);
    }
}
