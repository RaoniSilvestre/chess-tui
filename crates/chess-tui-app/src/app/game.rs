//! Game end detection and restart.

use crate::App;
use crate::constants::{Pages, Popups};
use crate::game::Game;
use chess_tui_core::Clock;
use chess_tui_game::{Bot, BotMode, GameKind};
use shakmaty::Color;

impl App {
    /// Shows the end screen popup. Uses the puzzle variant if a puzzle is active.
    pub fn show_end_screen(&mut self) {
        self.ui_state.current_popup = Some(if self.lichess_state.puzzle_game.is_some() {
            Popups::PuzzleEndScreen
        } else {
            Popups::EndScreen
        })
    }

    /// Updates game state and shows the end screen if the state just transitioned to checkmate or draw.
    /// Skipped on the PGN viewer page to avoid interfering with replay navigation.
    pub fn check_game_end_status(&mut self) {
        // PGN viewer drives its own game state via sync_pgn_to_board; running
        // update_game_state here would flip it to Checkmate/Draw on the final
        // position and open the EndScreen popup, which blocks viewer navigation.
        if self.ui_state.current_page == Pages::PgnViewer {
            return;
        }

        let previous_state = self.game.runtime.state();
        self.game.runtime.update_game_state();
        let new_state = self.game.runtime.state();

        if previous_state != new_state && new_state.is_finished() {
            self.show_end_screen();
            // Player-induced terminal states reach this branch (bot-induced
            // ones go through `check_and_show_game_end`). Either way the
            // game is over, so the resume save must go.
            self.clear_resume_state_for_current_mode();
        }
    }

    /// `restart`, but asks first when that would discard the game's resume save, which exists
    /// once a move has been played. A finished game has none, so it restarts at once.
    pub fn request_restart(&mut self) {
        if self
            .current_resume_mode()
            .is_some_and(|mode| self.has_resume_save(mode))
        {
            self.ui_state.pending_new_game = Some(crate::state::ui_state::NewGame::Restart);
            self.ui_state.current_popup = Some(Popups::NewGameConfirmation);
        } else {
            self.restart();
        }
    }

    /// Restarts the current game while keeping the same opponent and bot.
    /// Re-randomizes color if random color mode is active, re-initializes the clock, and
    /// triggers the bot's first move if it starts as White.
    pub fn restart(&mut self) {
        // Starting a fresh game wipes the previous one from disk — otherwise
        // an old save would shadow the new game until the first move's
        // autosave overwrote it.
        self.clear_resume_state_for_current_mode();

        // Clear puzzle state when restarting (for normal games)
        self.lichess_state.puzzle_game = None;
        self.ui_state.end_screen_dismissed = false;
        let mode_kind = self.game.runtime.kind();
        // Preserve skin and display mode
        let current_skin = self.game.ui.skin.clone();
        let display_mode = self.game.ui.display_mode;
        // Check if we're in a local game or bot game to preserve clock
        let is_local_game =
            self.ui_state.current_page == Pages::Solo && mode_kind == GameKind::Local;
        let is_bot_game = mode_kind == GameKind::Bot;

        if is_bot_game && self.game_mode_state.is_random_color {
            self.game_mode_state.selected_color = Some(if rand::random::<bool>() {
                Color::White
            } else {
                Color::Black
            });
        }

        self.game = Game::default();

        if is_bot_game {
            let bot_starting =
                self.game_mode_state.selected_color.unwrap_or(Color::White) == Color::Black;
            let path = self.bot_state.chess_engine_path.as_deref().unwrap_or("");
            let bot = Bot::new(
                path,
                bot_starting,
                self.bot_state.bot_depth,
                self.bot_state.bot_difficulty,
            );
            self.game.runtime.set_mode(Box::new(BotMode::new(bot)));
            if bot_starting {
                // Flip the board once so Black player sees from their perspective.
                self.game.runtime.flip();
            }
        }

        // Restore skin, display mode and piece styles
        self.game.ui.skin = current_skin;
        self.game.ui.display_mode = display_mode;
        self.game.ui.available_piece_styles = self.theme_state.available_piece_styles.clone();
        self.ui_state.close_popup();

        // Re-initialize clock for local games and bot games
        if is_local_game || is_bot_game {
            self.game
                .runtime
                .set_clock(Clock::new(self.game_mode_state.get_time_control()));
        }
    }

    /// Checks for game end conditions after a move and shows end screen if needed.
    /// This consolidates the repeated game end checking logic.
    pub fn check_and_show_game_end(&mut self) {
        // Update game state first (this will stop the clock if game ended)
        self.game.runtime.update_game_state();

        if self.game.runtime.state().is_finished() {
            // Game ended - only show end screen if it's not already shown and not dismissed
            if self.ui_state.current_popup != Some(Popups::EndScreen)
                && self.ui_state.current_popup != Some(Popups::PuzzleEndScreen)
                && !self.ui_state.end_screen_dismissed
            {
                self.show_end_screen();
            }
            // Drop any resume save: the game is over, there is nothing left to continue.
            self.clear_resume_state_for_current_mode();
        } else {
            // Game is no longer ended, reset the dismissed flag
            self.ui_state.end_screen_dismissed = false;
        }
    }
}

impl App {
    /// Prepare the bot page and report whether the board should be rendered.
    pub fn prepare_bot_page(&mut self) -> bool {
        if self
            .bot_state
            .chess_engine_path
            .as_ref()
            .is_none_or(|p| p.is_empty())
        {
            self.ui_state.popup_message = Some(
                "Chess engine path not configured.\n\n".to_string()
                    + &format!(
                        "To configure the chess engine follow the documentation: {}/Bot/bot-engines\n\n",
                        crate::constants::DOCS_URL
                    )
                    + "Example:\n"
                    + "chess-tui -e /opt/homebrew/opt/stockfish\n"
                    + " or execute the script: ./scripts/install-stockfish.sh\n"
                    + "to install stockfish automatically and set it as the chess engine path \n",
            );
            self.ui_state.current_popup = Some(crate::constants::Popups::Error);
            return false;
        }

        if let Some(engine_path) = &self.bot_state.chess_engine_path {
            let command_path = engine_path.split_whitespace().next().unwrap_or(engine_path);
            if !std::path::Path::new(command_path).exists()
                || !std::path::Path::new(command_path).is_file()
            {
                self.ui_state.popup_message = Some(format!(
                    "Chess engine path is invalid.\n\n\
                    The configured path does not exist or is not a file:\n\
                    {}\n\n\
                    Please check the path and update it in your configuration.\n\n\
                    To configure the chess engine follow the documentation: {}/Bot/bot-engines\n\n\
                    Example:\n\
                    chess-tui -e /opt/homebrew/bin/stockfish\n\
                    or execute the script: ./scripts/install-stockfish.sh\n\
                    to install stockfish automatically and set it as the chess engine path",
                    command_path,
                    crate::constants::DOCS_URL
                ));
                self.ui_state.current_popup = Some(crate::constants::Popups::Error);
                return false;
            }
        }

        self.game_mode_state.resolve_selected_color();
        if self.game.runtime.kind() != chess_tui_game::GameKind::Bot {
            self.bot_setup();
            false
        } else {
            true
        }
    }

    /// Prepare the multiplayer page and report whether the board should be rendered.
    pub fn prepare_multiplayer_page(&mut self) -> bool {
        self.game_mode_state.resolve_selected_color();
        if self.ui_state.current_popup == Some(crate::constants::Popups::Error) {
            return false;
        }
        if self.game.runtime.kind() != chess_tui_game::GameKind::Tcp {
            if self.multiplayer_state.host_ip.is_none() {
                if self.multiplayer_state.hosting == Some(true) {
                    if let Some(color) = self.game_mode_state.selected_color {
                        chess_tui_net::setup_game_server(color);
                        self.multiplayer_state.host_ip = Some("127.0.0.1".to_string());
                    }
                } else {
                    self.ui_state.current_popup = Some(crate::constants::Popups::EnterHostIP);
                }
            } else {
                self.create_opponent();
            }
            false
        } else {
            self.game.runtime.is_ready()
        }
    }
}
