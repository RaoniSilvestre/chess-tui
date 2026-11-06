//! Game state and move execution.

use super::{opponent::Opponent, ui::UI};
use chess_tui_core::{
    ClockState, Coord, GameBoard, GameOutcome, GameState, square_utils::flip_square_if_needed,
};
use chess_tui_game::{GameEvent, GameKind, GameMode, GameRuntime, GameSession, LocalMode};
use shakmaty::{Color, Move, Position, Role, Square};
use std::ops::{Deref, DerefMut};
use std::time::Instant;

/// Application-level game logic: a [`GameRuntime`] plus temporary opponent
/// orchestration that will move into a remote game mode.
pub struct GameLogic {
    runtime: GameRuntime,
    /// The other player when playing in multiplayer
    pub opponent: Option<Opponent>,
}

impl Default for GameLogic {
    fn default() -> Self {
        Self {
            runtime: GameRuntime::new(GameSession::default(), Box::new(LocalMode)),
            opponent: None,
        }
    }
}

impl Deref for GameLogic {
    type Target = GameRuntime;

    fn deref(&self) -> &Self::Target {
        &self.runtime
    }
}

impl DerefMut for GameLogic {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.runtime
    }
}

impl GameLogic {
    pub fn new(board: GameBoard, turn: Color) -> Self {
        Self {
            runtime: GameRuntime::new(GameSession::new(board, turn), Box::new(LocalMode)),
            opponent: None,
        }
    }

    pub fn runtime(&self) -> &GameRuntime {
        &self.runtime
    }

    pub fn runtime_mut(&mut self) -> &mut GameRuntime {
        &mut self.runtime
    }

    pub fn set_mode(&mut self, mode: Box<dyn GameMode>) {
        self.runtime.set_mode(mode);
    }

    pub fn kind(&self) -> GameKind {
        self.runtime.kind()
    }

    pub fn tick(&mut self) {
        self.runtime.tick();
    }

    pub fn take_events(&mut self) -> Vec<GameEvent> {
        self.runtime.take_events()
    }

    pub fn on_player_move(&mut self) {
        self.runtime.on_player_move();
    }

    pub fn on_promotion(&mut self) {
        self.runtime.on_promotion();
    }

    pub fn is_player_move_allowed(&self) -> bool {
        self.runtime.can_player_move()
    }

    pub fn flip_after_move(&self) -> bool {
        self.runtime.flip_after_move()
    }

    pub fn history_is_solo(&self) -> bool {
        self.runtime.history_is_solo()
    }

    pub fn session(&self) -> &GameSession {
        self.runtime.session()
    }

    pub fn session_mut(&mut self) -> &mut GameSession {
        self.runtime.session_mut()
    }
}

#[derive(Default)]
pub struct Game {
    pub logic: GameLogic,
    /// The struct to handle UI related stuff
    pub ui: UI,
}

impl Game {
    pub fn new(game_board: GameBoard, player_turn: Color) -> Self {
        Self {
            logic: GameLogic::new(game_board, player_turn),
            ui: UI::default(),
        }
    }

    /// Switch the player turn
    pub fn switch_player_turn(&mut self) {
        self.logic.switch_player_turn();
    }

    /// Methods to select a cell on the board
    pub fn handle_cell_click(&mut self, player_color: Option<shakmaty::Color>) {
        // In TCP multiplayer mode, check if it's the player's turn
        if let Some(opponent) = &self.logic.opponent
            && opponent.is_tcp_multiplayer()
            && let Some(my_color) = player_color
            && self.logic.turn() != my_color
        {
            // Player can only move when it's their turn
            return;
        }

        // If we are viewing history and making a move, truncate history at this point
        if let Some(history_index) = self.logic.history_position() {
            self.logic.truncate_history_at(history_index);
        }

        match self.logic.state() {
            GameState::Promotion(_) => {
                let should_flip = self.logic.flip_after_move();
                self.handle_promotion(should_flip);
            }
            GameState::Playing => {
                if !self.logic.is_player_move_allowed() {
                    return;
                }
                if self.ui.is_cell_selected() {
                    // Try to execute the move
                    self.already_selected_cell_action();
                } else {
                    // Selection Guard: Check if we SHOULD even call select_cell()
                    let square: shakmaty::Square = self.ui.cursor_coordinates.into();
                    let actual_square = flip_square_if_needed(square, self.logic.is_flipped());

                    let piece_color = self.logic.get_piece_color_at_square(&actual_square);

                    // Do nothing if clicking an empty square or the opponent's piece
                    if piece_color != Some(self.logic.turn()) {
                        return;
                    }

                    let authorized_positions = self
                        .logic
                        .get_authorized_positions(self.logic.turn(), &actual_square);

                    if !authorized_positions.is_empty() {
                        self.select_cell();
                    }
                }
            }
            _ => {}
        };

        self.logic.update_game_state();
    }

    /// Method to take care of the promotion after the selection
    pub fn handle_promotion(&mut self, should_flip: bool) {
        let role = match self.ui.promotion_cursor {
            0 => Role::Queen,
            1 => Role::Rook,
            2 => Role::Bishop,
            3 => Role::Knight,
            _ => {
                log::error!(
                    "Promotion cursor {} is out of valid range (0-3)",
                    self.ui.promotion_cursor
                );
                self.ui.promotion_cursor = 0;
                return;
            }
        };
        self.logic.promote_piece(role, should_flip);
        self.ui.promotion_cursor = 0;

        // Switch player turn (this will start the opponent's clock)
        self.logic.switch_player_turn();
        // Start clock on first move if needed (promotion can happen on first move)
        self.logic.start_clock_if_needed();

        // Update game state after promotion
        self.logic.update_game_state();

        if self.logic.opponent.is_some() {
            self.logic.handle_multiplayer_promotion();
        }

        self.logic.on_promotion();
    }

    /// Handle opponent-specific logic after a move
    fn handle_after_move_opponent_logic(&mut self) {
        if self.logic.opponent.is_none() {
            return;
        }

        self.logic.update_game_state();

        let state = self.logic.state();
        if state.is_promotion() {
            // Wait for the player to choose a promotion piece before sending.
            return;
        }

        let last_move = self.logic.moves().last().cloned();

        // Signal opponent to move and send move if game is still in progress.
        if let Some(opponent) = self.logic.opponent.as_mut() {
            if !state.is_finished() {
                opponent.opponent_will_move = true;
            }

            if let Some(last_move) = last_move {
                opponent.send_move_to_server(&last_move, last_move.promotion());

                // For Lichess games, signal the polling thread that player made a move.
                // This resets the polling skip flag so polling resumes for opponent's turn.
                if let Some(crate::game_logic::opponent::OpponentKind::Lichess {
                    player_move_tx: Some(tx),
                    ..
                }) = &opponent.kind
                {
                    if let Err(e) = tx.send(()) {
                        log::warn!("Failed to signal player move to polling thread: {}", e);
                    } else {
                        log::debug!("Signaled polling thread that player made a move");
                    }
                }
            } else {
                log::error!("Cannot send move to opponent: move history is empty");
            }
        }
    }

    /// Handle board flipping logic after a move (only in single-player mode)
    fn handle_after_move_board_flip(&mut self) {
        if self.logic.flip_after_move()
            && (!self.logic.is_latest_move_promotion()
                || self.logic.is_draw()
                || self.logic.is_checkmate())
        {
            self.logic.flip();
        }
    }

    /// Method to handle what we do after selecting a new cell with one already selected
    pub fn already_selected_cell_action(&mut self) {
        let Some(selected_square) = self.ui.selected_square else {
            return;
        };

        let cursor_square = self.ui.cursor_coordinates.into();

        let actual_selected_coords =
            flip_square_if_needed(selected_square, self.logic.is_flipped());
        let actual_cursor_coords = flip_square_if_needed(cursor_square, self.logic.is_flipped());

        if actual_cursor_coords == actual_selected_coords {
            self.ui.unselect_cell();
            return;
        }

        // Check authorized positions before taking any action.
        let authorized_positions = self
            .logic
            .get_authorized_positions(self.logic.turn(), &actual_selected_coords);

        if !authorized_positions.contains(&actual_cursor_coords) {
            let piece_at_destination = self.logic.get_piece_color_at_square(&actual_cursor_coords);

            if piece_at_destination == Some(self.logic.turn()) {
                self.select_cell();
            } else {
                self.ui.unselect_cell();
            }
            return;
        }

        self.logic
            .execute_move(actual_selected_coords, actual_cursor_coords);
        self.ui.unselect_cell();

        self.logic.update_game_state();

        if !self.logic.state().is_promotion() {
            self.logic.start_clock_if_needed();
            self.logic.switch_player_turn();
        } else {
            self.logic.pause_clock();
        }

        self.handle_after_move_board_flip();
        self.logic.on_player_move();
        self.handle_after_move_opponent_logic();
    }

    /// Method applied after selecting a cell on the board
    pub fn select_cell(&mut self) {
        // Is really needed?
        let square: Square = self.ui.cursor_coordinates.into();
        let square: Square = Coord::from(square).into();

        let actual_square = flip_square_if_needed(square, self.logic.is_flipped());

        // Check if there is a piece on the cell and if it's the right color
        let piece_color = self.logic.get_piece_color_at_square(&actual_square);

        if self.logic.get_role_at_square(&actual_square).is_none()
            || piece_color != Some(self.logic.turn())
        {
            return;
        }

        // Check if the piece on the cell can move before selecting it
        let authorized_positions = self
            .logic
            .get_authorized_positions(self.logic.turn(), &actual_square);

        if authorized_positions.is_empty() {
            return;
        }

        // We already verified the piece color matches player_turn above, so we can proceed
        self.ui.selected_square = Some(square);
        self.ui.old_cursor_position = Some(self.ui.cursor_coordinates);
    }

    pub fn apply_player_move(&mut self, from: Square, to: Square, promotion: Option<Role>) -> bool {
        let role_from = match self.logic.get_role_at_square(&from) {
            Some(role) => role,
            None => return false,
        };
        let role_to = self.logic.get_role_at_square(&to);

        let executed_move = self.logic.play(from, to, promotion);

        let Some(_executed_move) = executed_move else {
            return false;
        };

        crate::sound::play_move_sound();
        self.logic.record_move_for_draw_rule(role_from, role_to);

        self.logic.update_game_state();

        if !self.logic.state().is_promotion() {
            self.logic.start_clock_if_needed();
            self.logic.switch_player_turn();
        } else {
            self.logic.pause_clock();
        }

        self.handle_after_move_board_flip();
        self.logic.on_player_move();
        self.handle_after_move_opponent_logic();

        true
    }

    pub fn clock_state(&self) -> ClockState {
        self.logic.clock().state()
    }
}

impl GameLogic {
    pub fn update_game_state(&mut self) {
        self.session_mut().update_game_state();
    }

    /// Switch the player turn
    pub fn switch_player_turn(&mut self) {
        self.switch_turn();
    }

    /// Start the clock for the current player (used on first move)
    pub fn start_clock_if_needed(&mut self) {
        self.session_mut().start_clock_if_needed();
    }

    /// Sync player_turn with the current position's turn
    /// This is needed when navigating history to ensure moves are allowed for the correct player
    pub fn sync_player_turn_with_position(&mut self) {
        self.sync_turn_with_position();
    }

    /// Method to promote a pawn
    pub fn promote_piece(&mut self, role: Role, should_flip: bool) {
        if let Some(last_move) = self.moves().last().cloned() {
            let new_piece = role;

            // Promotion moves are always pawn moves, so they should have a from square
            let from_square = match last_move.from() {
                Some(sq) => sq,
                None => {
                    log::error!("Promotion move has no from square (unexpected castling?)");
                    return;
                }
            };

            let to_square = last_move.to();
            let capture = last_move.capture();
            self.undo_last_move();

            // Re-execute the move with the correct promotion
            self.play_and_record(from_square, to_square, Some(new_piece), |_| Move::Normal {
                role: Role::Pawn,
                from: from_square,
                capture,
                to: to_square,
                promotion: Some(new_piece),
            });
        }

        self.start_playing();
        if should_flip && !self.is_draw() && !self.is_checkmate() {
            self.flip();
        }
    }

    /// Move a piece from a cell to another
    pub fn execute_move(&mut self, from: Square, to: Square) {
        // Check if moving a piece and get the piece type
        let Some(role_from) = self.get_role_at_square(&from) else {
            return;
        };

        let role_to = self.get_role_at_square(&to);

        let executed_move = self.play_and_record(from, to, None, |executed_move| {
            // If this is a pawn reaching the last row, store it without promotion piece
            // so that is_latest_move_promotion() returns true and the popup appears.
            if role_from == Role::Pawn
                && (executed_move.to().rank() == shakmaty::Rank::First
                    || executed_move.to().rank() == shakmaty::Rank::Eighth)
                && executed_move.promotion().is_some()
            {
                // from() returns None only for drops (non-Normal moves), but we know this is Normal
                let from_square = executed_move.from().unwrap_or_else(|| executed_move.to());
                shakmaty::Move::Normal {
                    role: Role::Pawn,
                    from: from_square,
                    capture: executed_move.capture(),
                    to: executed_move.to(),
                    promotion: None,
                }
            } else {
                executed_move.clone()
            }
        });

        if executed_move.is_some() {
            // Play move sound
            crate::sound::play_move_sound();

            // We increment the consecutive_non_pawn_or_capture if the piece type is a pawn or if there is no capture
            self.record_move_for_draw_rule(role_from, role_to);
        }
    }

    /// Parse a move string in chess notation (e.g., "e2e4" or "e7e8q")
    /// Returns (from_square, to_square, promotion_piece) or None if invalid
    fn parse_opponent_move_string(move_str: &str) -> Option<(Square, Square, Option<Role>)> {
        if move_str.len() < 4 {
            return None;
        }

        let mut chars = move_str.chars();
        let from_file_char = chars.next();
        let from_rank_char = chars.next();
        let to_file_char = chars.next();
        let to_rank_char = chars.next();

        // Parse promotion piece if present (5th character)
        let promotion_piece: Option<Role> = if move_str.chars().count() == 5 {
            match move_str.chars().nth(4) {
                Some('q') => Some(Role::Queen),
                Some('r') => Some(Role::Rook),
                Some('b') => Some(Role::Bishop),
                Some('n') => Some(Role::Knight),
                _ => None,
            }
        } else {
            None
        };

        // The move is in standard chess notation (e.g., "e2e4")
        // Files are letters (a-h), ranks are digits (1-8)
        let from_str = format!(
            "{}{}",
            from_file_char.unwrap_or('a'),
            from_rank_char.unwrap_or('1')
        );
        let to_str = format!(
            "{}{}",
            to_file_char.unwrap_or('a'),
            to_rank_char.unwrap_or('1')
        );

        let from = Square::from_ascii(from_str.as_bytes()).ok()?;
        let to = Square::from_ascii(to_str.as_bytes()).ok()?;

        // Debug log for rook moves during parsing
        if move_str.len() >= 4 {
            // Check if this looks like a rook move (from a1, h1, a8, h8 or any file/rank move)
            let from_file = from_str.chars().next().unwrap_or('a');
            let from_rank = from_str.chars().nth(1).unwrap_or('1');
            if (from_file == 'a' || from_file == 'h') && (from_rank == '1' || from_rank == '8') {
                log::debug!(
                    "Parsing potential rook move: {} -> {} (raw: {})",
                    from_str,
                    to_str,
                    move_str
                );
            }
        }

        Some((from, to, promotion_piece))
    }

    /// Method to apply the opponent move (multiplayer game mode)
    pub fn execute_opponent_move(&mut self) -> bool {
        let opponent = if let Some(opp) = self.opponent.as_mut() {
            opp
        } else {
            return false;
        };
        let session = self.runtime.session_mut();

        // Read move from stream
        let opponent_move = match opponent.read_stream() {
            Ok(m) => m,
            Err(e) => {
                log::error!("Error reading opponent move: {}", e);
                return false;
            }
        };

        if opponent_move.is_empty() {
            return false;
        }

        // Handle control messages (INIT_MOVES, GAME_STATUS, CLOCK_SYNC)
        if Self::handle_opponent_control_message(opponent, session, &opponent_move) {
            return false;
        }

        // Increment moves received counter
        opponent.moves_received += 1;

        // Handle historical moves
        if Self::handle_historical_move(opponent, session, &opponent_move) {
            return false;
        }

        // Process live move
        Self::process_live_move(opponent, session, &opponent_move)
    }

    fn process_live_move(
        opponent: &mut Opponent,
        game_board: &mut GameBoard,
        move_str: &str,
    ) -> bool {
        log::info!("Executing opponent move: {}", move_str);

        // Parse move string
        let (from, to, promotion_piece) = match Self::parse_opponent_move_string(move_str) {
            Some(m) => m,
            None => {
                log::error!("Failed to parse opponent move: {}", move_str);
                return false;
            }
        };

        // Get the piece type at the source square to store it in history
        let piece_type_from = game_board.get_role_at_square(&from);
        let piece_color_from = game_board.get_piece_color_at_square(&from);

        // Debug log for rook moves
        if piece_type_from == Some(shakmaty::Role::Rook) {
            log::info!(
                "ROOK MOVE detected: {} -> {} (move string: {})",
                from,
                to,
                move_str
            );
        }

        // Check if move is already in history (duplicate detection)
        let move_already_played = game_board
            .moves()
            .iter()
            .any(|m| m.from() == Some(from) && m.to() == to);

        if move_already_played {
            log::warn!("Move {} already in history, skipping duplicate", move_str);
            return false;
        }

        // Special case: If history is empty but this move should be historical (based on initial_move_count),
        // try to add it to history even if execution fails
        // This handles the case when joining an ongoing game where history wasn't set up correctly
        let history_is_empty = game_board.moves().is_empty();
        if history_is_empty && opponent.moves_received <= opponent.initial_move_count {
            return Self::handle_forced_historical_move(
                game_board,
                move_str,
                from,
                to,
                promotion_piece,
                piece_type_from,
            );
        }

        // For castling moves, also check if any castling move has been played
        if piece_type_from == Some(shakmaty::Role::King) {
            let is_potential_castling = {
                let from_file = from.file();
                let to_file = to.file();
                let from_rank = from.rank();
                let to_rank = to.rank();
                from_rank == to_rank && (from_file as i8 - to_file as i8).abs() == 2
            };

            if is_potential_castling {
                log::info!(
                    "CASTLING MOVE detected: {} -> {} (move string: {})",
                    from,
                    to,
                    move_str
                );
                // Check if castling has already been played by checking if king has moved
                let king_has_moved = game_board.moves().iter().any(|m| {
                    m.from() == Some(from) // King has moved from its starting square
                });
                if king_has_moved {
                    log::warn!(
                        "King has already moved from {}, castling not possible",
                        from
                    );
                }
            }
        }

        // Log current board state for debugging
        log::debug!(
            "Before move execution - from: {}, to: {}, piece: {:?}, color: {:?}, current turn: {:?}",
            from,
            to,
            piece_type_from,
            piece_color_from,
            game_board.current_position().turn()
        );

        // Check if there's a piece at the destination
        let piece_at_dest = game_board.get_role_at_square(&to);
        let color_at_dest = game_board.get_piece_color_at_square(&to);
        log::debug!(
            "Destination square {} has piece: {:?}, color: {:?}",
            to,
            piece_at_dest,
            color_at_dest
        );

        let executed_move = game_board.play_and_record(from, to, promotion_piece, |executed| {
            if let Some(piece_type) = piece_type_from {
                Move::Normal {
                    role: piece_type,
                    from,
                    capture: executed.capture(),
                    to,
                    promotion: executed.promotion(),
                }
            } else {
                executed.clone()
            }
        });

        // Store in history (use visual coordinates for history)
        if let Some(move_to_store) = executed_move {
            // Debug log for rook moves execution
            if piece_type_from == Some(shakmaty::Role::Rook) {
                log::info!(
                    "ROOK MOVE executed successfully: {:?} (from: {}, to: {})",
                    move_to_store,
                    from,
                    to
                );
            }
            // If the move was executed successfully, we can stop waiting for the opponent
            opponent.opponent_will_move = false;

            // Play move sound
            crate::sound::play_move_sound();
            true
        } else {
            // Detailed error logging
            if piece_type_from == Some(shakmaty::Role::Rook) {
                log::error!(
                    "ROOK MOVE failed to execute: {} (from: {}, to: {})",
                    move_str,
                    from,
                    to
                );
            }

            // Check why the move failed
            let legal_moves = game_board.current_position().legal_moves();
            let matching_moves: Vec<_> = legal_moves
                .iter()
                .filter(|m| m.from() == Some(from))
                .collect();

            log::error!(
                "Move {} failed - piece at {}: {:?}, legal moves from {}: {} (showing first 5)",
                move_str,
                from,
                piece_type_from,
                from,
                matching_moves.len()
            );
            if !matching_moves.is_empty() {
                for m in matching_moves.iter().take(5) {
                    log::error!("  Legal move: {:?} -> {}", m.from(), m.to());
                }
            }

            log::warn!(
                "Failed to execute move on board: {} (from: {}, to: {})",
                move_str,
                from,
                to
            );
            false
        }
    }

    fn handle_opponent_control_message(
        opponent: &mut Opponent,
        session: &mut GameSession,
        message: &str,
    ) -> bool {
        // Check if this is a control message to update initial_move_count
        if message.starts_with("INIT_MOVES:") {
            if let Some(count_str) = message.strip_prefix("INIT_MOVES:")
                && let Ok(count) = count_str.parse::<usize>()
            {
                log::info!(
                    "Updating initial_move_count to {} (turns from stream), current moves_received: {}",
                    count,
                    opponent.moves_received
                );
                opponent.initial_move_count = count;
                // Set moves_received to at least initial_move_count so that the next move we receive
                // will be considered new (moves_received > initial_move_count)
                // But don't decrease it if moves have already been received
                if opponent.moves_received < count {
                    opponent.moves_received = count;
                }
            }
            return true;
        }

        // Resync the clock with the server-reported remaining times (Lichess)
        if message.starts_with("CLOCK_SYNC:") {
            let parts: Vec<&str> = message.split(':').collect();
            if let [_, w, b, color] = parts.as_slice() {
                let white_ms = w.parse::<u64>().unwrap_or(0);
                let black_ms = b.parse::<u64>().unwrap_or(0);
                let active_color = if *color == "white" {
                    Color::White
                } else {
                    Color::Black
                };
                session.clock_mut().sync_from_server(
                    white_ms,
                    black_ms,
                    active_color,
                    Instant::now(),
                );
            }
            return true;
        }

        // Check if this is a game status update from Lichess (draw, checkmate, etc.)
        if message.starts_with("GAME_STATUS:") {
            if let Some(status_str) = message.strip_prefix("GAME_STATUS:") {
                log::info!("Received game status update from Lichess: {}", status_str);
                match status_str {
                    "checkmate" => {
                        log::info!("Game ended by checkmate - updating game state");
                        session.finish(GameOutcome::Checkmate {
                            winner: session.turn().other(),
                        });
                    }
                    "draw" | "stalemate" | "repetition" | "insufficient" | "fifty" => {
                        log::info!("Game ended by draw ({}) - updating game state", status_str);
                        session.finish(GameOutcome::Draw);
                    }
                    "resign" => {
                        log::info!("Game ended by resignation - updating game state");
                        session.finish(GameOutcome::Resignation { winner: None });
                    }
                    "aborted" => {
                        log::info!("Game was aborted - updating game state");
                        session.finish(GameOutcome::Aborted);
                    }
                    _ => {
                        log::warn!("Unknown game status: {}", status_str);
                    }
                }
            }
            return true;
        }

        false
    }

    fn handle_historical_move(
        opponent: &mut Opponent,
        game_board: &mut GameBoard,
        move_str: &str,
    ) -> bool {
        // Handle historical moves - when joining an ongoing game, the stream replays all moves
        // If move_history is empty, we need to populate it from the stream
        // Otherwise, we skip historical moves since we've already set up the board
        let history_is_empty = game_board.moves().is_empty();

        // For ongoing games with populated history, check if the move is already in history
        let move_already_in_history = if !history_is_empty {
            if let Some((from, to, _)) = Self::parse_opponent_move_string(move_str) {
                game_board
                    .moves()
                    .iter()
                    .any(|m| m.from() == Some(from) && m.to() == to)
            } else {
                false
            }
        } else {
            false
        };

        let is_historical = if history_is_empty {
            // History is empty - if initial_move_count > 0, we're joining an ongoing game
            // In this case, treat moves as historical if:
            // 1. moves_received <= initial_move_count (within expected range), OR
            // 2. initial_move_count > 0 (ongoing game, so all early moves are historical)
            if opponent.initial_move_count > 0 {
                // Joining ongoing game - all moves should be historical until history is built
                true
            } else {
                // New game - moves are live
                false
            }
        } else {
            // History is already populated (ongoing game), so moves are historical if:
            // 1. moves_received <= initial_move_count (within historical range), OR
            // 2. The move is already in the history (duplicate detection)
            opponent.moves_received <= opponent.initial_move_count || move_already_in_history
        };

        log::debug!(
            "Move processing: move={}, moves_received={}, initial_move_count={}, is_historical={}, history_is_empty={}",
            move_str,
            opponent.moves_received,
            opponent.initial_move_count,
            is_historical,
            history_is_empty
        );

        if is_historical {
            if history_is_empty {
                // History is empty, we need to build it from stream moves
                log::info!(
                    "Building history from stream move {}/{}: {}",
                    opponent.moves_received,
                    opponent.initial_move_count,
                    move_str
                );

                // Ensure we have at least the initial position in history.
                if game_board.positions().is_empty() {
                    log::warn!("position history is empty! Initializing with starting position.");
                    game_board.ensure_initial_position();
                }

                // Parse and apply the move to build history
                if let Some((from, to, promotion_piece)) =
                    Self::parse_opponent_move_string(move_str)
                {
                    // Get the piece type at the source square
                    let piece_type_from = game_board.get_role_at_square(&from);

                    // Debug log for rook moves in history
                    if piece_type_from == Some(shakmaty::Role::Rook) {
                        log::info!(
                            "ROOK MOVE in history: {} -> {} (move string: {})",
                            from,
                            to,
                            move_str
                        );
                    }

                    // Execute the move and record it in history.
                    let executed_move =
                        game_board.play_and_record(from, to, promotion_piece, |executed| {
                            if let Some(piece_type) = piece_type_from {
                                Move::Normal {
                                    role: piece_type,
                                    from,
                                    capture: executed.capture(),
                                    to,
                                    promotion: executed.promotion(),
                                }
                            } else {
                                executed.clone()
                            }
                        });

                    if executed_move.is_some() {
                        if piece_type_from == Some(shakmaty::Role::Rook) {
                            log::info!(
                                "ROOK MOVE added to history (now {} moves, {} positions)",
                                game_board.moves().len(),
                                game_board.positions().len()
                            );
                        } else {
                            log::info!(
                                "Added historical move to history (now {} moves, {} positions)",
                                game_board.moves().len(),
                                game_board.positions().len()
                            );
                        }
                    } else {
                        // Move failed to execute - check if board is already in post-move state
                        // (i.e., the piece is at the destination, meaning the move was already played)
                        let piece_type_to = game_board.get_role_at_square(&to);

                        if piece_type_from.is_none() && piece_type_to.is_some() {
                            // No piece at source, but piece at destination - move was already played
                            // Work backwards: the piece at 'to' came from 'from'
                            log::info!(
                                "Move {} already applied to board (piece at {} but not at {}). Adding to history by working backwards.",
                                move_str,
                                to,
                                from
                            );

                            // Get the piece that moved (it's at the destination)
                            if let Some(piece_type) = piece_type_to {
                                // Create the move in history - we know the piece at 'to' came from 'from'
                                let historical_move = Move::Normal {
                                    role: piece_type,
                                    from,
                                    capture: None, // We don't know if it was a capture
                                    to,
                                    promotion: promotion_piece,
                                };
                                game_board.record_recovery_move(historical_move);

                                log::info!(
                                    "Added historical move {} to history by working backwards (now {} moves, {} positions)",
                                    move_str,
                                    game_board.moves().len(),
                                    game_board.positions().len()
                                );
                            }
                        } else {
                            if piece_type_from == Some(shakmaty::Role::Rook) {
                                log::error!(
                                    "ROOK MOVE failed to execute in history: {} (from: {}, to: {})",
                                    move_str,
                                    from,
                                    to
                                );
                            }
                            log::warn!("Failed to execute historical move: {}", move_str);
                        }
                    }
                } else {
                    log::warn!("Failed to parse historical move: {}", move_str);
                }
            } else {
                // History is already populated (ongoing game), skip this historical move
                log::debug!(
                    "Skipping historical move {}/{}: {} (history already populated)",
                    opponent.moves_received,
                    opponent.initial_move_count,
                    move_str
                );
            }
            return true; // Treated as a historical move (handled or skipped)
        }

        false // Not a historical move
    }

    fn handle_forced_historical_move(
        game_board: &mut GameBoard,
        move_str: &str,
        from: Square,
        to: Square,
        promotion_piece: Option<Role>,
        piece_type_from: Option<Role>,
    ) -> bool {
        log::info!(
            "Move {} should be historical but history is empty. Attempting to add to history.",
            move_str
        );

        // Try to execute the move first
        let piece_type = piece_type_from
            .or_else(|| game_board.get_role_at_square(&to))
            .unwrap_or(shakmaty::Role::Pawn); // Fallback to pawn if we can't determine

        let executed_move =
            game_board.play_and_record(from, to, promotion_piece, |executed| Move::Normal {
                role: piece_type,
                from,
                capture: executed.capture(),
                to,
                promotion: executed.promotion(),
            });

        if executed_move.is_some() {
            log::info!(
                "Added historical move {} to history (now {} moves)",
                move_str,
                game_board.moves().len()
            );
            return false; // Don't treat as a new move
        }

        // Move failed to execute, but we know it should be historical.
        // Try to add it to history anyway by using the piece at destination.
        let piece_at_to = game_board.get_role_at_square(&to);
        if let Some(role) = piece_at_to.or(piece_type_from) {
            // Create a move object - best-effort reconstruction
            let reconstructed_move = Move::Normal {
                role,
                from,
                capture: None, // We don't know if it was a capture
                to,
                promotion: promotion_piece,
            };
            game_board.record_recovery_move(reconstructed_move);
            log::warn!(
                "Failed to execute historical move {}, but added to history anyway (reconstructed, now {} moves)",
                move_str,
                game_board.moves().len()
            );
            return false; // Don't treat as a new move
        }
        false
    }

    pub fn handle_multiplayer_promotion(&mut self) {
        let last_move = match self.runtime.session().moves().last().cloned() {
            Some(m) => m,
            None => {
                log::error!("handle_multiplayer_promotion called but move history is empty");
                return;
            }
        };

        let opponent = match self.opponent.as_mut() {
            Some(opp) => opp,
            None => {
                log::error!("handle_multiplayer_promotion called but no opponent exists");
                return;
            }
        };

        opponent.send_move_to_server(&last_move, last_move.promotion());
        opponent.opponent_will_move = true;

        // For Lichess games, signal the polling thread that player made a move
        // This resets the polling skip flag so polling resumes for opponent's turn
        if let Some(crate::game_logic::opponent::OpponentKind::Lichess {
            player_move_tx: Some(tx),
            ..
        }) = &opponent.kind
        {
            if let Err(e) = tx.send(()) {
                log::warn!("Failed to signal player move to polling thread: {}", e);
            } else {
                log::debug!("Signaled polling thread that player made a move (promotion)");
            }
        }
    }

    /// Navigate to the next position in history (forward in time)
    pub fn navigate_history_next(&mut self) {
        let is_solo_mode = self.history_is_solo();
        if self.session_mut().navigate_history_next(is_solo_mode) {
            // Update player_turn to match the position's turn
            self.sync_player_turn_with_position();
        }
    }

    /// Navigate to the previous position in history (backward in time)
    pub fn navigate_history_previous(&mut self) {
        let is_solo_mode = self.history_is_solo();
        if self.session_mut().navigate_history_previous(is_solo_mode) {
            // Update player_turn to match the position's turn
            self.sync_player_turn_with_position();
        }
    }
}
