//! UCI chess engine integration.

use ruci::{Engine as RuciEngine, Go};
use shakmaty::fen::Fen;
use shakmaty::uci::UciMove;
use std::borrow::Cow;
use std::process::Command;
use std::str::FromStr;

/// `UCI_Elo` values passed to the engine for each difficulty preset.
pub const BOT_DIFFICULTY_ELO: [u16; 4] = [400, 900, 1500, 2700];
/// Engine search depth for each difficulty preset.
pub const BOT_DIFFICULTY_DEPTH: [u8; 4] = [1, 4, 8, 20];
/// Engine think time in milliseconds for each difficulty preset.
pub const BOT_DIFFICULTY_MOVETIME_MS: [u64; 4] = [25, 120, 500, 12_000];

/// Configuration and UCI access for a chess engine process.
#[derive(Clone)]
pub struct Engine {
    engine_path: String,
    /// Base depth when difficulty is Off (full strength)
    depth: u8,
    /// Difficulty preset index: None = Off (full strength), Some(0..=3) = Easy/Medium/Hard/Magnus
    difficulty: Option<u8>,
}

impl Engine {
    pub fn new(engine_path: &str, depth: u8, difficulty: Option<u8>) -> Engine {
        Self {
            engine_path: engine_path.to_string(),
            depth,
            difficulty,
        }
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

    pub fn depth(&self) -> u8 {
        self.depth
    }

    pub fn difficulty(&self) -> Option<u8> {
        self.difficulty
    }

    /// Get the best move from the chess engine.
    pub fn find_move(&self, fen: &str) -> Result<UciMove, String> {
        let parts: Vec<&str> = self.engine_path.split_whitespace().collect();
        let (command, args) = if parts.is_empty() {
            (self.engine_path.as_str(), &[] as &[&str])
        } else {
            (parts[0], &parts[1..])
        };

        let mut cmd = Command::new(command);
        if !args.is_empty() {
            cmd.args(args);
        }

        let mut process = cmd
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn engine process: {e}"))?;

        let mut engine = RuciEngine::from_process(&mut process, false)
            .map_err(|e| format!("Failed to initialize engine: {e}"))?;

        // UCI handshake (required before setoption/position). Discard options.
        engine
            .use_uci(|_| {})
            .map_err(|e| format!("Failed UCI handshake: {e}"))?;

        // Optional ELO limit via UCI options (UCI_LimitStrength + UCI_Elo) when difficulty is set
        if let Some(elo) = self.elo() {
            engine
                .send(ruci::gui::SetOption {
                    name: Cow::Borrowed("UCI_LimitStrength"),
                    value: Some(Cow::Borrowed("true")),
                })
                .map_err(|e| format!("Failed to set UCI_LimitStrength: {e}"))?;
            engine
                .send(ruci::gui::SetOption {
                    name: Cow::Borrowed("UCI_Elo"),
                    value: Some(Cow::Owned(elo.to_string())),
                })
                .map_err(|e| format!("Failed to set UCI_Elo: {e}"))?;
        }

        let fen_parsed =
            Fen::from_str(fen).map_err(|e| format!("Failed to parse FEN '{fen}': {e}"))?;

        engine
            .send(ruci::Position::Fen {
                fen: Cow::Owned(fen_parsed),
                moves: Cow::Borrowed(&[]),
            })
            .map_err(|e| format!("Failed to send position to engine: {e}"))?;

        let depth = self.effective_depth();
        let move_time_ms = self.movetime_ms();

        engine
            .go(
                &Go {
                    depth: Some(depth as usize),
                    move_time: move_time_ms.map(|ms| ms as usize),
                    ..Default::default()
                },
                |_| {},
            )
            .map_err(|e| format!("Engine failed to compute move: {e}"))?
            .take_normal()
            .ok_or_else(|| "Engine returned non-normal move".to_string())
            .map(|r| r.r#move)
    }
}
