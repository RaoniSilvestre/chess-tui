//! Lichess pages: menu, ongoing games, live game, and puzzles.

pub mod live;
pub mod menu;
pub mod ongoing;
pub mod puzzle;
pub mod rating_chart;

use lichess::{LichessClient, OngoingGame, RatingHistoryEntry, UserProfile};
use shakmaty::Color;
use std::sync::mpsc::Receiver;

pub use live::LichessGamePage;
pub use menu::LichessMenuPage;
pub use ongoing::OngoingGamesPage;
pub use puzzle::PuzzleGamePage;

/// Everything a live-game page needs to join a game. The page fetches the
/// remaining state (FEN, turn count, last move) itself.
#[derive(Clone, Debug)]
pub struct LichessGameSpec {
    pub game_id: String,
    pub color: Color,
    pub fen: Option<String>,
}

pub enum LichessUpdate {
    ProfileResult(Result<Box<(UserProfile, Vec<RatingHistoryEntry>)>, String>),
    SeekResult(Result<(String, Color), String>),
    OngoingGamesResult(Result<Vec<OngoingGame>, String>),
}

pub type LichessReceiver = Receiver<LichessUpdate>;

/// Creates a client from the persisted token, if one is configured.
pub fn client_from_globals(globals: &crate::globals::Globals) -> Option<LichessClient> {
    globals.lichess_token().map(LichessClient::new)
}
