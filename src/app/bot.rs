//! Engine thread and move application.

use crate::app::App;
use crate::constants::Popups;
use crate::game_logic::bot::Bot;
use crate::sound::play_move_sound;
use chess_tui_core::Clock;
use shakmaty::{Color, Move};

impl App {
    /// Initializes the bot for a new game. Creates the `Bot` instance, sets up the clock,
    /// flips the board if the human plays Black, and triggers the bot's first move if it starts.
    pub fn bot_setup(&mut self) {
        // Bot starts (plays White) when the human chose Black.
        let is_bot_starting =
            self.game_mode_state.selected_color.unwrap_or(Color::White) == shakmaty::Color::Black;
        let path = self.bot_state.chess_engine_path.as_deref().unwrap_or("");
        self.game.logic.bot = Some(Bot::new(
            path,
            is_bot_starting,
            self.bot_state.bot_depth,
            self.bot_state.bot_difficulty,
        ));

        // Initialize clock for bot games.
        self.game.logic.clock = Clock::new(self.game_mode_state.get_time_control());

        if let Some(color) = self.game_mode_state.selected_color
            && color == Color::Black
        {
            // Flip the board once so Black player sees from their perspective
            self.game.logic.game_board.flip();

            if let Some(bot) = &self.game.logic.bot
                && self.game.logic.player_turn != color
            {
                self.bot_state.start_bot_thinking(
                    self.game.logic.game_board.fen(),
                    bot.depth(),
                    bot.difficulty(),
                );
            }
            // Don't set player_turn to Black here - the bot (White) moves first,
            // so player_turn should remain White until after the bot's first move
        }

        // Ensure skin is preserved when setting up bot
        if let Some(skin) = &self.theme_state.loaded_skin {
            self.game.ui.skin = skin.clone();
        }

        self.update_config_from_app();
    }

    /// Polls the bot move channel and applies the move if one is ready. Returns `true` if a move was applied.
    pub fn check_and_apply_bot_move(&mut self) -> bool {
        if let Some(rx) = &self.bot_state.bot_move_receiver {
            match rx.try_recv() {
                Ok(Ok(bot_move)) => {
                    self.apply_bot_move(bot_move);
                    self.bot_state.bot_move_receiver = None;
                    return true;
                }
                Ok(Err(e)) => {
                    log::error!("Bot engine error: {e}");
                    self.ui_state
                        .show_message_popup(format!("Chess engine error:\n{e}"), Popups::Error);
                    self.bot_state.bot_move_receiver = None;
                }
                Err(_) => {}
            }
        }
        false
    }

    /// Applies a bot move. Records captures, updates position history, switches turn, and plays the move sound.
    pub fn apply_bot_move(&mut self, bot_move: Move) {
        let Some(from_square) = bot_move.from() else {
            log::error!("Bot move has no from square");
            return;
        };
        let to_square = bot_move.to();
        let role = bot_move.role();
        let capture = bot_move.capture();
        let promotion = bot_move.promotion();

        // Normalize all move types to Normal for history display (castles/en-passant share the same fields).
        let played =
            self.game
                .logic
                .game_board
                .play_and_record(from_square, to_square, promotion, |_| Move::Normal {
                    role,
                    from: from_square,
                    capture,
                    to: to_square,
                    promotion,
                });

        if played.is_none() {
            log::error!("Failed to play bot move");
            return;
        }

        self.game.logic.switch_player_turn();

        // Play move sound
        play_move_sound();
    }
}
