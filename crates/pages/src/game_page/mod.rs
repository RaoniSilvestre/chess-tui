//! Concrete game pages and the shared board controller.

mod board;
mod bot;
mod lichess;
mod local;
mod multiplayer;
mod popup_end;
mod popup_promotion;
mod render;

pub use bot::BotGamePage;
pub use lichess::LichessGamePage;
pub use local::LocalGamePage;
pub use multiplayer::MultiplayerGamePage;
pub use render::render_game_ui;
