//! Holds the [`Bot`](crate::game_logic::bot::Bot) configuration and the engine session used to
//! compute moves off-thread.

use crate::game_logic::bot::{BotEvent, BotSession, EngineConfig};

/// Bot settings and the engine session for the current game.
///
/// The session is started explicitly at game setup, polled each tick via
/// [`BotState::poll_move`], and shut down when the game ends or restarts.
pub struct BotState {
    /// Bot thinking depth for chess engine (used when difficulty is Off)
    pub bot_depth: u8,
    /// Bot difficulty preset: None = Off (full strength), Some(0..=3) = Easy/Medium/Hard/Magnus
    pub bot_difficulty: Option<u8>,
    /// path of the chess engine
    pub chess_engine_path: Option<String>,
    /// Engine session for the current game.
    session: Option<BotSession>,
}

impl Default for BotState {
    fn default() -> Self {
        Self {
            bot_depth: 10,
            bot_difficulty: None,
            chess_engine_path: None,
            session: None,
        }
    }
}

impl BotState {
    /// Starts a fresh engine session from the configured settings.
    ///
    /// Any previous session is shut down first.
    pub fn start_engine(&mut self) -> Result<(), String> {
        self.shutdown_engine();

        let path = self
            .chess_engine_path
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .ok_or_else(|| {
                "No chess engine configured. Set one from the settings menu.".to_string()
            })?
            .to_string();

        let config = EngineConfig {
            path,
            depth: self.bot_depth,
            difficulty: self.bot_difficulty,
        };

        self.session = Some(BotSession::start(config)?);
        Ok(())
    }

    /// Queues a bot move for `fen` on the current session.
    pub fn request_move(&mut self, fen: String) -> Result<(), String> {
        match &mut self.session {
            Some(session) => session.request_move(fen),
            None => Err("Bot engine is not running".to_string()),
        }
    }

    /// Returns the next engine event, if one is ready.
    pub fn poll_move(&mut self) -> Option<BotEvent> {
        self.session.as_mut().and_then(BotSession::poll)
    }

    /// Whether a bot move has been requested and not yet received.
    pub fn is_bot_thinking(&self) -> bool {
        self.session.as_ref().is_some_and(BotSession::is_thinking)
    }

    /// Stops the engine session.
    pub fn shutdown_engine(&mut self) {
        if let Some(session) = self.session.take() {
            session.shutdown();
        }
    }
}
