//! Application state, page framework, and shared services.

pub mod animations;
pub mod app;
pub mod config;
pub mod constants;
pub mod game_mode;
pub mod game_mode_menu;
pub mod modal;
pub mod puzzle;
pub mod state;

pub use app::{App, AppResult};
pub use game_mode::GameModeState;

pub use page::{
    AsAny, AvailableGameMode, GameAction, GameModeAction, GameModeView, GamePageKind, GameView,
    LichessAction, LichessView, OngoingAction, Page, PageAction, PageContext, PageFactory,
    PgnAction, PgnViewerAccess, Router, SettingsAction, SettingsView,
};

use ratatui::Frame;
use ratatui::crossterm::event::KeyEvent;

/// Result of a modal key event.
pub enum ModalResult {
    None,
    Close,
    Replace(Box<dyn Modal>),
}

/// A modal overlay. Modals own their local state and receive the application.
pub trait Modal: AsAny {
    fn handle_key(&mut self, key: KeyEvent, app: &mut App) -> ModalResult;
    fn render(&mut self, frame: &mut Frame<'_>, area: ratatui::layout::Rect, app: &App);
}
