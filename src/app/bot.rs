//! Bot game setup and move application.

use crate::constants::Popups;
use crate::game_logic::bot::{Bot, BotEvent};
use crate::sound::play_move_sound;
use crate::{app::App, game_logic::clock::Clock};
use shakmaty::{Color, Move};

impl App {
    /// Initializes the bot for a new game. Creates the `Bot` instance, sets up the clock,
    /// starts the engine session, flips the board if the human plays Black, and triggers
    /// the bot's first move if it starts.
    pub fn bot_setup(&mut self) {
        // Bot starts (plays White) when the human chose Black.
        let is_bot_starting =
            self.game_mode_state.selected_color.unwrap_or(Color::White) == shakmaty::Color::Black;

        self.game.logic.bot = Some(Bot::new(
            is_bot_starting,
            self.bot_state.bot_depth,
            self.bot_state.bot_difficulty,
        ));

        // Initialize clock for bot games.
        self.game.logic.clock = Clock::new(self.game_mode_state.get_time_control());

        // A new game gets a fresh engine session, started eagerly.
        let engine_started = self.start_bot_engine();

        if let Some(color) = self.game_mode_state.selected_color
            && color == Color::Black
        {
            // Flip the board once so Black player sees from their perspective
            self.game.logic.game_board.flip_the_board();

            if engine_started && self.game.logic.player_turn != color {
                self.request_bot_move();
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

    /// Starts the engine session for the current bot game. Returns whether it started.
    pub fn start_bot_engine(&mut self) -> bool {
        match self.bot_state.start_engine() {
            Ok(()) => true,
            Err(error) => {
                self.show_engine_error(&error);
                false
            }
        }
    }

    /// Requests the next bot move, showing a popup if the engine cannot be used.
    pub fn request_bot_move(&mut self) {
        let fen = self.game.logic.game_board.fen_position();
        if let Err(error) = self.bot_state.request_move(fen) {
            self.show_engine_error(&error);
        }
    }

    /// Polls the bot move channel and applies the move if one is ready. Returns `true` if a move was applied.
    pub fn check_and_apply_bot_move(&mut self) -> bool {
        match self.bot_state.poll_move() {
            Some(BotEvent::Move(bot_move)) => {
                self.apply_bot_move(bot_move);
                true
            }
            Some(BotEvent::Failed(error)) => {
                self.show_engine_error(&error);
                false
            }
            None => false,
        }
    }

    fn show_engine_error(&mut self, error: &str) {
        log::error!("Bot engine error: {error}");
        self.ui_state
            .show_message_popup(format!("Chess engine error:\n{error}"), Popups::Error);
    }

    /// Applies a bot move. Records captures, updates position history, switches turn, and plays the move sound.
    pub fn apply_bot_move(&mut self, bot_move: Move) {
        use shakmaty::Position;

        let current_position = match self.game.logic.game_board.current_position() {
            Some(pos) => pos.clone(),
            None => {
                log::error!("Cannot apply bot move: position history is empty");
                return;
            }
        };

        // Record captured piece (before applying the move) so material display is correct
        match &bot_move {
            Move::Normal { .. } => {
                if let Some(captured_piece) = current_position.board().piece_at(bot_move.to()) {
                    self.game.logic.game_board.taken_pieces.push(captured_piece);
                }
            }
            Move::EnPassant { .. } => {
                if let (Some(from_square), to_square) = (bot_move.from(), bot_move.to()) {
                    // Captured pawn sits on the destination file but the origin rank (behind the passing pawn).
                    let captured_pawn_square =
                        shakmaty::Square::from_coords(to_square.file(), from_square.rank());
                    if let Some(captured_piece) =
                        current_position.board().piece_at(captured_pawn_square)
                    {
                        self.game.logic.game_board.taken_pieces.push(captured_piece);
                    }
                }
            }
            Move::Castle { .. } | Move::Put { .. } => {}
        }

        // Store in history
        let Some(from_square) = bot_move.from() else {
            log::error!("Bot move has no from square");
            return;
        };
        // Normalize all move types to Normal for history display (castles/en-passant share the same fields).
        self.game.logic.game_board.move_history.push(Move::Normal {
            role: bot_move.role(),
            from: from_square,
            capture: bot_move.capture(),
            to: bot_move.to(),
            promotion: bot_move.promotion(),
        });

        let Ok(new_position) = current_position.play(bot_move) else {
            log::error!("Failed to play bot move");
            return;
        };
        self.game
            .logic
            .game_board
            .position_history
            .push(new_position);
        // Reset history navigation. A new move invalidates any in-progress replay and restores board orientation.
        self.game.logic.game_board.history_position_index = None;
        self.game.logic.game_board.original_flip_state = None;
        self.game.logic.switch_player_turn();

        // Play move sound
        play_move_sound();
    }
}
