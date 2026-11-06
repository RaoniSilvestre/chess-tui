//! Bot runtime state layered over the UCI engine crate.

use chess_tui_engine::Engine;
use shakmaty::uci::UciMove;

/// Engine plus the per-game state used by the application loop.
#[derive(Clone)]
pub struct Bot {
    pub engine: Engine,
    /// Used to indicate if a bot move is following
    pub bot_will_move: bool,
    // if the bot is starting, meaning the player is black
    pub is_bot_starting: bool,
}

impl Bot {
    pub fn new(engine_path: &str, is_bot_starting: bool, depth: u8, difficulty: Option<u8>) -> Bot {
        Self {
            engine: Engine::new(engine_path, depth, difficulty),
            bot_will_move: false,
            is_bot_starting,
        }
    }

    /// Get the best move from the chess engine.
    pub fn get_move(&self, fen: &str) -> Result<UciMove, String> {
        self.engine.get_move(fen)
    }

    pub fn depth(&self) -> u8 {
        self.engine.depth
    }

    pub fn difficulty(&self) -> Option<u8> {
        self.engine.difficulty
    }
}
