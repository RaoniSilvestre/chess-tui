//! Pure chess domain types shared by the application.

mod clock;
mod coord;
mod game_board;
mod game_state;
mod pgn;
pub mod square_utils;

pub use clock::{Clock, ClockState, TimeControl};
pub use coord::{Coord, MoveDirection};
pub use game_board::{GameBoard, HistoryError};
pub use game_state::{GameOutcome, GameState, PendingPromotion};
pub use pgn::{DEFAULT_SPEED_IDX, PgnViewer, SPEEDS};
