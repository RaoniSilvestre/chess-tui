//! Application pages.

pub mod credits;
pub mod factory;
pub mod game;
pub mod game_mode;
pub mod home;
pub mod lichess;
pub mod pgn;
pub mod popups;
pub mod settings;

pub use credits::CreditsPage;
pub use game::GamePage;
pub use game_mode::GameModeMenuPage;
pub use home::HomePage;
pub use lichess::{LichessMenuPage, OngoingGamesPage};
pub use pgn::PgnViewerPage;
pub use settings::SettingsPage;
pub use factory::DefaultPageFactory;
