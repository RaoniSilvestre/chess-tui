//! Pure chess domain types shared by the application.

mod board;
mod clock;
mod coord;
mod game_state;
mod pgn;
pub mod square_utils;

pub use board::{Board, HistoryError};
pub use clock::{Clock, ClockState, TimeControl};
pub use coord::{Coord, MoveDirection};
pub use game_state::{DrawReason, GameOutcome, GameState, PendingPromotion};
pub use pgn::{DEFAULT_SPEED_IDX, PgnViewer, SPEEDS};
