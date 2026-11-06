//! Bot engine configuration.

/// Engine configuration used when constructing a bot mode.
pub struct BotState {
    /// Bot thinking depth for chess engine (used when difficulty is Off)
    pub bot_depth: u8,
    /// Bot difficulty preset: None = Off (full strength), Some(0..=3) = Easy/Medium/Hard/Magnus
    pub bot_difficulty: Option<u8>,
    /// Path of the chess engine.
    pub chess_engine_path: Option<String>,
}

impl Default for BotState {
    fn default() -> Self {
        Self {
            bot_depth: 10,
            bot_difficulty: None,
            chess_engine_path: None,
        }
    }
}
