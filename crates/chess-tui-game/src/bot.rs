//! Bot runtime state layered over the UCI engine crate.

use chess_tui_engine::Engine;
use shakmaty::uci::UciMove;

/// Engine plus the per-game state used by the bot mode.
#[derive(Clone)]
pub struct Bot {
    engine: Engine,
    starting: bool,
}

impl Bot {
    pub fn new(engine_path: &str, starting: bool, depth: u8, difficulty: Option<u8>) -> Bot {
        Self {
            engine: Engine::new(engine_path, depth, difficulty),
            starting,
        }
    }

    /// Ask the engine for its best move in `fen`.
    pub fn find_move(&self, fen: &str) -> Result<UciMove, String> {
        self.engine.find_move(fen)
    }

    pub fn depth(&self) -> u8 {
        self.engine.depth()
    }

    pub fn difficulty(&self) -> Option<u8> {
        self.engine.difficulty()
    }

    /// Whether the bot starts the game (the human chose Black).
    pub fn is_starting(&self) -> bool {
        self.starting
    }
}
