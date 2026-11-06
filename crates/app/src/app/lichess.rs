//! Lichess authentication, game streaming, and puzzle management.

use crate::App;
use crate::constants::{DOCS_URL, LICHESS_TIME_CONTROLS, SLEEP_DURATION_RESIGN_MS};
use crate::puzzle::PuzzleGame;
use crate::state::lichess_state::LichessUpdate;
use ::game::LichessMode;
use ::lichess::LichessClient;
use model::Clock;
use shakmaty::Color;
use std::sync::mpsc::channel;

impl App {
    const LICHESS_CLIENT_NOT_INITIALIZED_MSG: &'static str =
        "Lichess client not properly initialized, did you configure a lichess token ?";

    /// Validates a Lichess API token by fetching the user profile, then saves it to config and
    /// navigates to the Lichess menu on success, or shows an error popup on failure.
    pub fn save_and_validate_lichess_token(&mut self, token: String) {
        // First, try to validate the token by fetching the user profile

        match LichessClient::new(token.clone()).get_user_profile() {
            Ok(profile) => {
                // Token is valid, save it
                self.services.lichess_state.token = Some(token.clone());
                self.services.lichess_state.client = Some(LichessClient::new(token));
                let username = profile.username.clone();
                self.services.lichess_state.user_profile = Some(profile);

                // Save to config file
                self.update_config_from_app();

                // Navigate to Lichess menu if we're on Home page, otherwise stay on current page
                if self.is_home() {
                    self.show_lichess_menu();
                }

                // Close the popup and show success message
                self.close_modal();
                let msg = format!(
                    "Lichess token saved successfully!\n\n Logged in as: {}\n\n You can now use all Lichess features.",
                    username
                );
                self.show_success(msg);
            }
            Err(e) => {
                // Token is invalid, show error
                let msg = format!(
                    "Invalid Lichess token.\n\nError: {}\n\n Please check your token and try again.\n\n Follow the documentation: {}/Lichess/setup",
                    DOCS_URL, e
                );
                self.show_error(String::new());
                self.show_error(msg);
            }
        }
    }

    /// Clears the Lichess token and user profile, persists the change, and returns to the home page.
    pub fn disconnect_lichess(&mut self) {
        // Clear the token
        self.services.lichess_state.token = None;

        // Clear user profile
        self.services.lichess_state.user_profile = None;

        // Clear ongoing games
        self.services.lichess_state.ongoing_games.clear();

        // Save to config file
        self.update_config_from_app();

        // Navigate back to home menu
        self.show_home();

        // Show success message
        let msg = "Disconnected from Lichess successfully!\n\n Your token has been removed.\n\n You can reconnect anytime from the Lichess menu.".to_string();
        self.show_success(msg);
    }

    /// Starts a background thread to seek a Lichess correspondence game and opens the seeking popup.
    /// The result is polled each tick via `check_lichess_seek`.
    pub fn create_lichess_opponent(&mut self, cursor: u8) {
        let Ok(client) = self.services.lichess_state.require_client() else {
            self.show_error(Self::LICHESS_CLIENT_NOT_INITIALIZED_MSG.to_string());
            return;
        };
        let client = client.clone();

        let (tx, rx) = channel();
        self.services.lichess_state.receiver = Some(rx);

        let cancellation_token = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        self.services.lichess_state.cancellation_token = Some(cancellation_token.clone());

        // Lichess board seek only supports the two presets in
        // `LICHESS_TIME_CONTROLS` (cursor 3 or 4 in the shared time control
        // list).
        let idx = usize::from(cursor >= 4);
        let time_control = LICHESS_TIME_CONTROLS[idx];
        let (time, increment) = (time_control.minutes, time_control.increment);

        self.show_seeking_lichess_game();

        std::thread::spawn(
            move || match client.seek_game(time, increment, cancellation_token) {
                Ok((game_id, color)) => {
                    let _ = tx.send(LichessUpdate::SeekResult(Ok((game_id, color))));
                }
                Err(e) => {
                    let _ = tx.send(LichessUpdate::SeekResult(Err(e.to_string())));
                }
            },
        );
    }

    /// Joins an existing Lichess game by ID or full URL. Fetches the player color from the server
    /// and sends the result through the seek channel so `check_lichess_seek` can finalize setup.
    pub fn join_lichess_game_by_code(&mut self, game_code: String) {
        let Ok(client) = self.services.lichess_state.require_client() else {
            self.show_error(Self::LICHESS_CLIENT_NOT_INITIALIZED_MSG.to_string());
            return;
        };

        let client = client.clone();

        let (tx, rx) = channel();
        self.services.lichess_state.receiver = Some(rx);

        self.show_seeking_lichess_game();

        std::thread::spawn(move || {
            // Extract game ID from URL if a full URL was provided
            let game_id = if game_code.contains("lichess.org/") {
                // Extract ID from URL like "https://lichess.org/O8uBDzKS"
                game_code
                    .split('/')
                    .next_back()
                    .unwrap_or(&game_code)
                    .to_string()
            } else {
                game_code
            };

            let my_id = match client.get_user_profile() {
                Ok(profile) => profile.id,
                Err(e) => {
                    let _ = tx.send(LichessUpdate::SeekResult(Err(format!(
                        "Failed to fetch profile: {}",
                        e
                    ))));
                    return;
                }
            };

            // Join the game by code
            match client.join_game(&game_id, my_id) {
                Ok((game_id, color)) => {
                    let _ = tx.send(LichessUpdate::SeekResult(Ok((game_id, color))));
                }
                Err(e) => {
                    let _ = tx.send(LichessUpdate::SeekResult(Err(e.to_string())));
                }
            }
        });
    }

    /// Polls the shared background channel once per tick and dispatches on message type.
    pub fn check_lichess_updates(&mut self) {
        if let Some(rx) = &self.services.lichess_state.receiver
            && let Ok(update) = rx.try_recv()
        {
            self.services.lichess_state.receiver = None;
            self.close_modal();

            match update {
                // We received an seek fetch
                LichessUpdate::SeekResult(result) => {
                    if let Some(cancellation_token) = &self.services.lichess_state.cancellation_token {
                        cancellation_token.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                    self.services.lichess_state.cancellation_token = None;

                    match result {
                        Ok((game_id, color)) => {
                            log::info!("Found Lichess game: {} with color {:?}", game_id, color);
                            self.setup_lichess_game_with_state(game_id, color, None);
                        }
                        Err(e) => {
                            log::error!("Failed to seek Lichess game: {}", e);
                            self.show_error(format!("Failed to seek game: {}", e));
                        }
                    }
                }
                // We received an update from the profile fetch
                LichessUpdate::ProfileResult(result) => match result {
                    Ok(data) => {
                        let (profile, history) = *data;
                        self.services.lichess_state.user_profile = Some(profile);
                        self.services.lichess_state.rating_history = Some(history);
                    }
                    Err(e) => {
                        log::error!("Failed to fetch user profile: {}", e);
                        self.show_error(
                            "An error occured when trying to fetch your lichess profile"
                                .to_string(),
                        );
                    }
                },
                LichessUpdate::OngoingGamesResult(result) => match result {
                    Ok(games) => {
                        self.services.lichess_state.ongoing_games = games;
                        self.close_modal();
                    }
                    Err(e) => {
                        log::error!("Failed to fetch ongoing games: {}", e);
                        self.show_error(format!("Failed to fetch ongoing games: {}", e));
                    }
                },
            }
        }
    }

    /// Shared setup path for joining a Lichess game by any means (seek, code, ongoing list).
    /// Resolves the starting FEN (from cache, the ongoing-games list, or the API), parses it,
    /// and calls [`Self::setup_lichess_game`]. Returns `false` if setup cannot proceed.
    fn setup_lichess_game_with_state(
        &mut self,
        game_id: String,
        color: Color,
        initial_fen: Option<String>,
    ) -> bool {
        // Try to get FEN if not provided
        let mut fen = initial_fen;

        // If not provided, try to get FEN from ongoing games if available
        if fen.is_none() {
            fen = self.services.lichess_state
                .ongoing_games
                .iter()
                .find(|g| g.game_id == game_id)
                .map(|g| g.fen.clone());
        }

        // If still not found, try fetching ongoing games to get FEN
        if fen.is_none() {
            let Ok(client) = self.services.lichess_state.require_client() else {
                self.show_error(Self::LICHESS_CLIENT_NOT_INITIALIZED_MSG.to_string());
                return false;
            };
            if let Ok(ongoing_games) = client.get_ongoing_games() {
                fen = ongoing_games
                    .iter()
                    .find(|g| g.game_id == game_id)
                    .map(|g| g.fen.clone());
            }
        }

        // If we have FEN, set up board from FEN
        // Also try to get turn count to set initial_move_count correctly
        if let Some(ref fen_str) = fen {
            log::info!("Setting up game from FEN: {}", fen_str);

            // Try to get turn count and last move from public API immediately
            // This ensures the last move shows in green immediately when joining
            let mut initial_move_count = 0;
            let mut last_move_to_add: Option<String> = None;

            let Ok(client) = self.services.lichess_state.require_client() else {
                self.show_error(Self::LICHESS_CLIENT_NOT_INITIALIZED_MSG.to_string());
                return false;
            };

            match client.get_game_turn_count_and_last_move(&game_id) {
                Ok((turns, last_move)) => {
                    initial_move_count = turns;
                    log::info!(
                        "Got turn count from API: {} (half-moves), last move: {:?}",
                        initial_move_count,
                        last_move
                    );
                    last_move_to_add = last_move;
                }
                Err(e) => {
                    log::warn!(
                        "Failed to get game info: {} (last move may not show immediately)",
                        e
                    );
                }
            }

            match shakmaty::fen::Fen::from_ascii(fen_str.as_bytes()) {
                Ok(fen_data) => {
                    match fen_data
                        .into_position::<shakmaty::Chess>(shakmaty::CastlingMode::Standard)
                    {
                        Ok(position) => {
                            let load_result = {
                                let mut game = self.services.game_mut();
                                let halfmove_clock = game.runtime.halfmove_clock();
                                game.runtime.load_history(
                                    vec![position],
                                    vec![],
                                    vec![],
                                    halfmove_clock,
                                )
                            };
                            if let Err(e) = load_result {
                                log::error!("Failed to load Lichess position: {e}");
                                self.show_error(format!("Failed to load position: {e}"));
                                return false;
                            }
                            self.services.game_mut().runtime.sync_turn_with_position();
                            // Send last move immediately if we have it, so it shows in green right away
                            self.setup_lichess_game(
                                game_id,
                                color,
                                initial_move_count,
                                last_move_to_add,
                            );
                            return true;
                        }
                        Err(e) => {
                            log::error!("Failed to parse FEN position: {}", e);
                            self.show_error(format!("Failed to parse FEN: {}", e));
                            return false;
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to parse FEN string: {}", e);
                    self.show_error(format!("Failed to parse FEN: {}", e));
                    return false;
                }
            }
        }

        // Final fallback: No FEN available (new game from seek)
        // For new games from seek, initial_move_count is 0
        log::info!("No FEN available, setting up as new game");
        self.setup_lichess_game(game_id, color, 0, None);
        true
    }

    /// Wires up channels, starts the game-event stream thread and the outgoing-move thread,
    /// sets the player color, flips the board for Black, and navigates to the Lichess page.
    fn setup_lichess_game(
        &mut self,
        game_id: String,
        color: Color,
        initial_move_count: usize,
        immediate_last_move: Option<String>,
    ) {
        // Reset game state so a previous ended game doesn't block input in handle_cell_click
        self.services.game_mut().runtime.start_playing();
        self.close_modal();

        // Best-effort initial clock; the first game event resyncs with the server.
        self.services.game_mut()
            .runtime
            .set_clock(Clock::new(self.services.game_mode_state.get_time_control()));

        let Ok(client) = self.services.lichess_state.require_client() else {
            self.show_error(Self::LICHESS_CLIENT_NOT_INITIALIZED_MSG.to_string());
            return;
        };
        let mode = match LichessMode::new(
            client.clone(),
            game_id,
            color,
            initial_move_count,
            immediate_last_move,
        ) {
            Ok(mode) => mode,
            Err(e) => {
                log::error!("Failed to stream Lichess game: {}", e);
                self.show_error(format!("Failed to stream game: {}", e));
                return;
            }
        };
        self.services.game_mut().runtime.set_mode(Box::new(mode));

        self.services.game_mode_state.selected_color = Some(color);
        self.services.game_mode_state.is_random_color = false;

        if color == Color::Black {
            self.services.game_mut().runtime.flip();
        }
        // Ensure skin is preserved
        if let Some(skin) = &self.services.theme_state.loaded_skin {
            self.services.game_mut().ui.skin = skin.clone();
        }

        // Switch to Lichess page to show the game board
        self.show_lichess();
    }

    /// Fetches the list of ongoing Lichess games and stores them in `lichess_state`.
    pub fn fetch_ongoing_games(&mut self) {
        let Ok(client) = self.services.lichess_state.require_client() else {
            self.show_error(Self::LICHESS_CLIENT_NOT_INITIALIZED_MSG.to_string());
            return;
        };

        let client = client.clone();
        let (tx, rx) = channel();
        self.services.lichess_state.receiver = Some(rx);

        std::thread::spawn(move || match client.get_ongoing_games() {
            Ok(games) => {
                let _ = tx.send(LichessUpdate::OngoingGamesResult(Ok(games)));
            }
            Err(e) => {
                let _ = tx.send(LichessUpdate::OngoingGamesResult(Err(e.to_string())));
            }
        });
    }

    /// Resigns the currently selected game in a background thread and shows a success popup.
    pub fn confirm_resign_game(&mut self, index: usize) {
        if let Some(game) = self.services.lichess_state
            .ongoing_games
            .get(index)
        {
            let game_id = game.game_id.clone();
            let opponent_name = game.opponent.username.clone();

            let Ok(client) = self.services.lichess_state.require_client() else {
                self.show_error(Self::LICHESS_CLIENT_NOT_INITIALIZED_MSG.to_string());
                return;
            };
            let client = client.clone();

            // Resign in a separate thread to avoid blocking
            let game_id_clone = game_id.clone();
            std::thread::spawn(move || {
                match client.resign_game(&game_id_clone) {
                    Ok(_) => {
                        log::info!("Successfully resigned game: {}", game_id_clone);
                    }
                    Err(e) => {
                        log::error!("Failed to resign game {}: {}", game_id_clone, e);
                    }
                }

                // Wait 500ms for the resignation to be processed on Lichess servers
                std::thread::sleep(std::time::Duration::from_millis(SLEEP_DURATION_RESIGN_MS));

                // Fetch updated ongoing games list
                match client.get_ongoing_games() {
                    Ok(games) => {
                        log::info!(
                            "Refreshed ongoing games list after resignation, found {} games",
                            games.len()
                        );
                        // Note: We can't directly update app.ongoing_games from this thread
                        // The UI will need to poll or we need a channel to send the update
                    }
                    Err(e) => {
                        log::error!("Failed to refresh ongoing games after resignation: {}", e);
                    }
                }
            });

            self.close_modal();

            // Show success message
            let msg = format!(
                "Game resigned successfully!\n\nYou have resigned the game vs {}.\n\n The game list will be updated shortly.",
                opponent_name
            );
            self.show_success(msg);

            log::info!(
                "Resignation request sent for game {} vs {}",
                game_id,
                opponent_name
            );
        }
    }

    /// Spawns a background thread to fetch the user profile and rating history.
    /// The result is polled each tick via `check_lichess_profile_loaded`.
    pub fn fetch_lichess_user_profile(&mut self) {
        let Ok(client) = self.services.lichess_state.require_client() else {
            self.show_error(Self::LICHESS_CLIENT_NOT_INITIALIZED_MSG.to_string());
            return;
        };

        let client = client.clone();
        let (tx, rx) = channel();
        self.services.lichess_state.receiver = Some(rx);

        std::thread::spawn(move || {
            const MAX_ATTEMPTS: u32 = 3;
            const RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(2);

            let mut last_err = String::new();
            for attempt in 1..=MAX_ATTEMPTS {
                match client.get_user_profile() {
                    Ok(profile) => {
                        let username = profile.username.clone();
                        let history = client.get_rating_history(&username).unwrap_or_default();
                        let _ = tx.send(LichessUpdate::ProfileResult(Ok(Box::new((
                            profile, history,
                        )))));
                        return;
                    }
                    Err(e) => {
                        last_err = e.to_string();
                        log::warn!(
                            "Profile fetch attempt {}/{} failed: {}",
                            attempt,
                            MAX_ATTEMPTS,
                            last_err
                        );
                        if attempt < MAX_ATTEMPTS {
                            std::thread::sleep(RETRY_DELAY);
                        }
                    }
                }
            }
            let _ = tx.send(LichessUpdate::ProfileResult(Err(last_err)));
        });
    }

    /// Joins the ongoing Lichess game at `index`, restoring board state from FEN.
    pub fn select_ongoing_game(&mut self, index: usize) {
        log::debug!("select_ongoing_game called, index: {}", index);

        if let Some(game) = self.services.lichess_state.ongoing_games.get(index) {
            let game_id = game.game_id.clone();
            let color_str = game.color.clone();
            let fen = game.fen.clone();

            // Convert color string to shakmaty::Color
            let color = if color_str == "white" {
                shakmaty::Color::White
            } else {
                shakmaty::Color::Black
            };

            log::info!(
                "Joining ongoing game: {} as {:?} with FEN: {}",
                game_id,
                color,
                fen
            );

            // Use the helper function to set up the game with state (pass the FEN we already have)
            self.setup_lichess_game_with_state(game_id, color, Some(fen));
        }
    }

    /// Fetches a daily Lichess puzzle, resets game state, and switches to the Solo page in puzzle mode.
    pub fn start_puzzle_mode(&mut self) {
        // Clear any existing popups and error messages when starting a new puzzle
        self.close_modal();
        self.services.lichess_state.puzzle_game = None;

        // Puzzles use a local board with Lichess-provided moves.
        self.services.game_mut().runtime.set_mode(Box::new(::game::LocalMode));
        self.services.game_mode_state.selected_color = None;
        self.services.game_mode_state.is_random_color = false;

        // Reset game state to Playing (in case it was Checkmate/Draw from previous puzzle)
        // This must be done early to prevent check_and_show_game_end from re-showing the popup
        self.services.game_mut().runtime.start_playing();

        // Fetch fresh user profile to get the latest puzzle rating
        // This ensures we use the correct baseline rating after any previous puzzle's Elo change
        self.fetch_lichess_user_profile();

        let Ok(client) = self.services.lichess_state.require_client() else {
            self.show_error(Self::LICHESS_CLIENT_NOT_INITIALIZED_MSG.to_string());
            return;
        };
        let client = client.clone();

        // Get rating before
        let mut rating_before = None;
        if let Some(profile) = &self.services.lichess_state.user_profile
            && let Some(perfs) = &profile.perfs
            && let Some(puzzle_perf) = &perfs.puzzle
        {
            rating_before = Some(puzzle_perf.rating);
        }

        // Initialize PuzzleGame
        let load_result = {
            let mut game = self.services.game_mut();
            PuzzleGame::load(&client, game.runtime.session_mut().board_mut())
        };
        let puzzle = match load_result {
            Ok(p) => p,
            Err(e) => {
                self.show_error(e);
                return;
            }
        };

        // Sync the player turn with the position's turn
        self.services.game_mut().runtime.sync_turn_with_position();

        self.services.lichess_state.puzzle_game = Some(PuzzleGame::new(puzzle, rating_before));

        // Ensure board stays unflipped in puzzle mode
        if self.services.lichess_state.puzzle_game.is_some() {
            self.services.game_mut().runtime.set_flipped(false);
        }
        // Switch to Solo page to show the board
        self.show_solo();
    }

    /// Validates a puzzle move after execution. Returns `true` if correct.
    /// Always auto-plays the opponent's response regardless of whether the move was right or wrong.
    pub fn validate_puzzle_move_after_execution(
        &mut self,
        from: shakmaty::Square,
        to: shakmaty::Square,
    ) -> bool {
        // Check if we're in puzzle mode
        if self.services.lichess_state.puzzle_game.is_none() {
            return true;
        }

        // Convert the move to UCI format (e.g., "e2e4")
        self.validate_puzzle_move_uci(format!("{}{}", from, to))
    }

    /// Validates an already executed puzzle move given in UCI format (e.g. "e2e4", "e7e8q").
    /// Returns `true` if correct or if not in puzzle mode.
    pub fn validate_puzzle_move_uci(&mut self, move_uci: String) -> bool {
        // We need to temporarily take puzzle_game out to avoid borrowing issues
        if let Some(mut puzzle_game) = self.services.lichess_state.puzzle_game.take() {
            let (is_correct, message) = puzzle_game.validate_move(
                move_uci,
                &mut self.services.game_mut(),
                self.services.lichess_state.token.clone(),
            );

            // Put it back
            self.services.lichess_state.puzzle_game = Some(puzzle_game);

            if let Some(msg) = message {
                if is_correct {
                    // Success message (puzzle solved)
                    self.show_puzzle_end(msg);
                } else {
                    self.show_error(msg);
                }
            }

            return is_correct;
        }

        true
    }
}
