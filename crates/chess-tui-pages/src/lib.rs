//! Application pages.

pub mod credits;
pub mod game_mode;
pub mod home;
pub mod lichess;
pub mod pgn;
pub mod settings;

pub use credits::CreditsPage;
pub use game_mode::GameModeMenuPage;
pub use home::HomePage;
pub use lichess::{LichessMenuPage, OngoingGamesPage};
pub use pgn::PgnViewerPage;
pub use settings::SettingsPage;
