//! Tracks the active page, current popup overlay, menu cursor position, and transient UI flags.

use crate::constants::{Pages, Popups};

/// Navigation and overlay state held by [`crate::app::App`].
///
/// Tracks which page and popup are currently rendered, the shared `menu_cursor`
/// used by all list-based pages, and transient flags like `end_screen_dismissed`
/// that coordinate popup visibility across ticks.
pub struct UIState {
    /// Current page to render
    pub current_page: Pages,
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
    /// From the game-mode form: start a game on this page.
    Start(Pages),
    /// From a game in progress: restart it.
    Restart,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            current_page: Pages::Home,
            current_popup: None,
            popup_message: None,
            end_screen_dismissed: false,
            menu_cursor: 0,
            pending_new_game: None,
        }
    }
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
    /// Toggles between the credits page and the home page.
    pub fn toggle_credit_popup(&mut self) {
        if self.current_page == Pages::Home {
            self.current_page = Pages::Credit;
        } else {
            self.navigate_to_homepage();
        }
    }

    /// Switches to the home page and resets the menu cursor to 0.
    pub fn navigate_to_homepage(&mut self) {
        self.current_page = Pages::Home;
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
