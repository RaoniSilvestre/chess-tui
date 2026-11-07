//! Concrete game pages and the shared board controller.

pub mod board;
mod bot;
mod local;
mod multiplayer;

pub use bot::{BotGameConfig, BotGamePage};
pub use local::LocalGamePage;
pub use multiplayer::MultiplayerGamePage;
