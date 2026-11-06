//! Game state and move execution.

use super::ui::UI;
use chess_tui_core::{Board, ClockState, Coord, GameState, square_utils::flip_square_if_needed};
use chess_tui_game::{GameEvent, GameKind, GameMode, GameRuntime, GameSession, LocalMode};
use shakmaty::{Color, Move, Role, Square};
use std::ops::{Deref, DerefMut};

/// Application-level game logic.
pub struct GameLogic {
    runtime: GameRuntime,
}

impl Default for GameLogic {
    fn default() -> Self {
        Self {
            runtime: GameRuntime::new(GameSession::default(), Box::new(LocalMode)),
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
    pub fn new(board: Board, turn: Color) -> Self {
        Self {
            runtime: GameRuntime::new(GameSession::new(board, turn), Box::new(LocalMode)),
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
    pub fn new(game_board: Board, player_turn: Color) -> Self {
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
    pub fn handle_cell_click(&mut self) {
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

        self.logic.on_promotion();
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
