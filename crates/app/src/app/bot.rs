//! Bot game setup and event translation.

use crate::App;
use ::game::{Bot, BotMode};
use model::Clock;
use shakmaty::Color;

impl App {
    /// Initializes the bot for a new game. Creates the bot mode, sets up the clock,
    /// and flips the board if the human plays Black. `BotMode::tick` starts the
    /// engine when it is the bot's turn.
    pub fn bot_setup(&mut self) {
        let is_bot_starting =
            self.services.game_mode_state.selected_color.unwrap_or(Color::White) == Color::Black;
        let path = self.services.bot_state.chess_engine_path.as_deref().unwrap_or("");
        let bot = Bot::new(
            path,
            is_bot_starting,
            self.services.bot_state.bot_depth,
            self.services.bot_state.bot_difficulty,
        );
        self.services.game_mut().runtime.set_mode(Box::new(BotMode::new(bot)));

        // Initialize clock for bot games.
        self.services.game_mut()
            .runtime
            .set_clock(Clock::new(self.services.game_mode_state.get_time_control()));

        if self.services.game_mode_state.selected_color == Some(Color::Black) {
            // Flip the board once so Black player sees from their perspective.
            self.services.game_mut().runtime.flip();
        }

        // Ensure skin is preserved when setting up bot
        if let Some(skin) = &self.services.theme_state.loaded_skin {
            self.services.game_mut().ui.skin = skin.clone();
        }

        self.update_config_from_app();
    }
}
