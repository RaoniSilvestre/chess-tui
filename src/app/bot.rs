//! Bot game setup and event translation.

use crate::app::App;
use chess_tui_core::Clock;
use chess_tui_game::{Bot, BotMode};
use shakmaty::Color;

impl App {
    /// Initializes the bot for a new game. Creates the bot mode, sets up the clock,
    /// and flips the board if the human plays Black. `BotMode::tick` starts the
    /// engine when it is the bot's turn.
    pub fn bot_setup(&mut self) {
        let is_bot_starting =
            self.game_mode_state.selected_color.unwrap_or(Color::White) == Color::Black;
        let path = self.bot_state.chess_engine_path.as_deref().unwrap_or("");
        let bot = Bot::new(
            path,
            is_bot_starting,
            self.bot_state.bot_depth,
            self.bot_state.bot_difficulty,
        );
        self.game.runtime.set_mode(Box::new(BotMode::new(bot)));

        // Initialize clock for bot games.
        self.game
            .runtime
            .set_clock(Clock::new(self.game_mode_state.get_time_control()));

        if self.game_mode_state.selected_color == Some(Color::Black) {
            // Flip the board once so Black player sees from their perspective.
            self.game.runtime.flip();
        }

        // Ensure skin is preserved when setting up bot
        if let Some(skin) = &self.theme_state.loaded_skin {
            self.game.ui.skin = skin.clone();
        }

        self.update_config_from_app();
    }
}
