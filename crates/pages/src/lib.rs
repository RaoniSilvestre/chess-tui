//! Pages and page-owned application state.
//!
//! The application shell only owns the active page, `Globals`, and the run
//! flag. Everything else lives in the page that uses it.

pub mod constants;
pub mod credits;
pub mod factory;
pub mod game_mode;
pub mod game_page;
pub mod globals;
pub mod home;
pub mod lichess_pages;
pub mod overlay;
pub mod page;
pub mod pgn;
pub mod resume;
pub mod settings;

#[cfg(test)]
mod test_support;

pub use credits::CreditsPage;
pub use factory::create_page;
pub use game_mode::{AvailableGameMode, GameModeMenuPage, MatchConfig};
pub use game_page::{BotGameConfig, BotGamePage, LocalGamePage, MultiplayerGamePage};
pub use globals::{BotSettings, Globals, Theme};
pub use home::HomePage;
pub use lichess_pages::{
    LichessGamePage, LichessGameSpec, LichessMenuPage, OngoingGamesPage, PuzzleGamePage,
};
pub use page::{Page, PageAction, PageKind, PageRequest};
pub use pgn::PgnViewerPage;
pub use settings::SettingsPage;
