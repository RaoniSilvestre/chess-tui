//! Application pages.

pub mod credits;
pub mod factory;
pub mod game_mode;
pub mod game_page;
pub mod home;
pub mod lichess_pages;
pub mod pgn;
pub mod settings;

pub use credits::CreditsPage;
pub use factory::DefaultPageFactory;
pub use game_mode::GameModeMenuPage;
pub use game_page::{
    BotGamePage, LichessGamePage, LocalGamePage, MultiplayerGamePage,
};
pub use home::HomePage;
pub use lichess_pages::{LichessMenuPage, OngoingGamesPage};
pub use pgn::PgnViewerPage;
pub use settings::SettingsPage;
