//! UCI engine wrapper and bot difficulty config.

use crate::constants::{BOT_DIFFICULTY_DEPTH, BOT_DIFFICULTY_ELO, BOT_DIFFICULTY_MOVETIME_MS};
use shakmaty::fen::Fen;
use shakmaty::uci::UciMove;
use shakmaty::{CastlingMode, Chess, Move};
use std::process::{Command, Stdio};
use uci::{Engine, Go};

/// UCI engine wrapper and bot difficulty config.
#[derive(Clone)]
pub struct Bot {
    pub engine_path: String,
    /// Used to indicate if a bot move is following
    pub bot_will_move: bool,
    // if the bot is starting, meaning the player is black
    pub is_bot_starting: bool,
    /// Base depth when difficulty is Off (full strength)
    pub depth: u8,
    /// Difficulty preset index: None = Off (full strength), Some(0..=3) = Easy/Medium/Hard/Magnus
    pub difficulty: Option<u8>,
}

impl Bot {
    pub fn new(engine_path: &str, is_bot_starting: bool, depth: u8, difficulty: Option<u8>) -> Bot {
        Self {
            engine_path: engine_path.to_string(),
            bot_will_move: false,
            is_bot_starting,
            depth,
            difficulty,
        }
    }

    /// Advances the engine for `fen` and returns the chosen legal move.
    ///
    /// This is what the bot thread runs; it owns all FEN/UCI conversion so the
    /// caller only has to send the resulting `Result` through its channel.
    pub fn get_move(&self, fen: &str) -> Result<Move, String> {
        let position = Fen::from_ascii(fen.as_bytes())
            .map_err(|e| format!("Failed to parse FEN '{fen}': {e}"))?
            .into_position::<Chess>(CastlingMode::Standard)
            .map_err(|e| format!("Failed to set up position from FEN '{fen}': {e}"))?;

        let uci_move = self.query_engine(fen)?;

        uci_move
            .to_move(&position)
            .map_err(|e| format!("Engine returned illegal UCI move: {e}"))
    }

    /// Spawns the engine, configures it and returns its raw UCI move.
    fn query_engine(&self, fen: &str) -> Result<UciMove, String> {
        let mut engine = Engine::spawn(self.engine_command())
            .map_err(|e| format!("Failed to spawn engine process: {e}"))?;

        // UCI handshake (required before setoption/position). Discard options.
        engine
            .handshake()
            .map_err(|e| format!("Failed UCI handshake: {e}"))?;

        // Optional ELO limit via UCI options (UCI_LimitStrength + UCI_Elo) when difficulty is set
        if let Some(elo) = self.elo() {
            engine
                .set_option("UCI_LimitStrength", Some("true"))
                .map_err(|e| format!("Failed to set UCI_LimitStrength: {e}"))?;
            let elo = elo.to_string();
            engine
                .set_option("UCI_Elo", Some(&elo))
                .map_err(|e| format!("Failed to set UCI_Elo: {e}"))?;
        }

        engine
            .position_fen(fen)
            .map_err(|e| format!("Failed to send position to engine: {e}"))?;

        let best_move = engine
            .go(Go {
                depth: Some(u32::from(self.effective_depth())),
                move_time: self.movetime_ms(),
            })
            .map_err(|e| format!("Engine failed to compute move: {e}"))?
            .ok_or_else(|| "Engine returned no move".to_string())?;

        best_move.movement.parse::<UciMove>().map_err(|e| {
            format!(
                "Engine returned invalid UCI move '{}': {e}",
                best_move.movement
            )
        })
    }

    /// Builds the engine command, splitting any arguments off `engine_path`.
    fn engine_command(&self) -> Command {
        let mut parts = self.engine_path.split_whitespace();
        let mut command = Command::new(parts.next().unwrap_or_default());
        command.args(parts);

        // Keep engine diagnostics from corrupting the TUI.
        command.stderr(Stdio::null());
        command
    }

    /// Effective depth: preset depth when difficulty is set, else base depth.
    fn effective_depth(&self) -> u8 {
        self.difficulty
            .and_then(|i| {
                let idx = i as usize;
                if idx < BOT_DIFFICULTY_DEPTH.len() {
                    Some(BOT_DIFFICULTY_DEPTH[idx])
                } else {
                    None
                }
            })
            .unwrap_or(self.depth)
    }

    /// Movetime in ms when difficulty is set; None = no limit (full strength).
    fn movetime_ms(&self) -> Option<u64> {
        self.difficulty.and_then(|i| {
            let idx = i as usize;
            if idx < BOT_DIFFICULTY_MOVETIME_MS.len() {
                Some(BOT_DIFFICULTY_MOVETIME_MS[idx])
            } else {
                None
            }
        })
    }

    /// ELO for UCI_LimitStrength when difficulty is set.
    fn elo(&self) -> Option<u16> {
        self.difficulty.and_then(|i| {
            let idx = i as usize;
            if idx < BOT_DIFFICULTY_ELO.len() {
                Some(BOT_DIFFICULTY_ELO[idx])
            } else {
                None
            }
        })
    }
}
