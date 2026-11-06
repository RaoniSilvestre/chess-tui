//! High-level state of a chess game.

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum GameState {
    Checkmate,
    Draw,
    Playing,
    Promotion,
}
