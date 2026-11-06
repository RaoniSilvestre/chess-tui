//! Bot runtime state layered over the UCI engine crate.

use chess_tui_engine::Engine;
use shakmaty::uci::UciMove;

/// Engine plus the per-game state used by the application loop.
#[derive(Clone)]
pub struct Bot {
    engine: Engine,
    move_requested: bool,
    starting: bool,
}

impl Bot {
    pub fn new(engine_path: &str, starting: bool, depth: u8, difficulty: Option<u8>) -> Bot {
        Self {
            engine: Engine::new(engine_path, depth, difficulty),
            move_requested: false,
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

    pub fn set_starting(&mut self, starting: bool) {
        self.starting = starting;
    }

    /// Whether a bot move should be requested on the next loop iteration.
    pub fn move_requested(&self) -> bool {
        self.move_requested
    }

    pub fn request_move(&mut self) {
        self.move_requested = true;
    }

    pub fn clear_move_request(&mut self) {
        self.move_requested = false;
    }
}
