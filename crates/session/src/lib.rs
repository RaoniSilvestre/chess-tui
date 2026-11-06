//! Game state and move execution.

use game::{GameRuntime, GameSession, LocalMode};
use model::{Board, ClockState, Coord, GameState, square_utils::flip_square_if_needed};
use shakmaty::{Color, Move, Role, Square};
use std::cell::RefCell;
use std::rc::Rc;
use ui::board::BoardViewState as UI;

#[derive(Default)]
pub struct Game {
    pub runtime: GameRuntime,
    /// The struct to handle UI related stuff
    pub ui: UI,
}

impl Game {
    pub fn new(game_board: Board, player_turn: Color) -> Self {
        Self {
            runtime: GameRuntime::new(
                GameSession::new(game_board, player_turn),
                Box::new(LocalMode),
            ),
            ui: UI::default(),
        }
    }

    /// Switch the player turn
    pub fn switch_player_turn(&mut self) {
        self.runtime.switch_turn();
    }

    /// Methods to select a cell on the board
    pub fn handle_cell_click(&mut self) {
        // If we are viewing history and making a move, truncate history at this point
        if let Some(history_index) = self.runtime.history_position() {
            self.runtime.truncate_history_at(history_index);
        }

        match self.runtime.state() {
            GameState::Promotion(_) => {
                let should_flip = self.runtime.flip_after_move();
                self.handle_promotion(should_flip);
            }
            GameState::Playing => {
                if !self.runtime.can_player_move() {
                    return;
                }
                if self.ui.is_cell_selected() {
                    // Try to execute the move
                    self.already_selected_cell_action();
                } else {
                    // Selection Guard: Check if we SHOULD even call select_cell()
                    let square: shakmaty::Square = self.ui.cursor_coordinates.into();
                    let actual_square = flip_square_if_needed(square, self.runtime.is_flipped());

                    let piece_color = self.runtime.get_piece_color_at_square(&actual_square);

                    // Do nothing if clicking an empty square or the opponent's piece
                    if piece_color != Some(self.runtime.turn()) {
                        return;
                    }

                    let authorized_positions = self
                        .runtime
                        .get_authorized_positions(self.runtime.turn(), &actual_square);

                    if !authorized_positions.is_empty() {
                        self.select_cell();
                    }
                }
            }
            _ => {}
        };

        self.runtime.update_game_state();
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
        self.promote_piece(role, should_flip);
        self.ui.promotion_cursor = 0;

        // Switch player turn (this will start the opponent's clock)
        self.runtime.switch_turn();
        // Start clock on first move if needed (promotion can happen on first move)
        self.runtime.start_clock_if_needed();

        // Update game state after promotion
        self.runtime.update_game_state();

        self.runtime.on_promotion();
    }

    /// Handle board flipping logic after a move (only in single-player mode)
    fn handle_after_move_board_flip(&mut self) {
        if self.runtime.flip_after_move()
            && (!self.runtime.is_latest_move_promotion()
                || self.runtime.is_draw()
                || self.runtime.is_checkmate())
        {
            self.runtime.flip();
        }
    }

    /// Method to handle what we do after selecting a new cell with one already selected
    pub fn already_selected_cell_action(&mut self) {
        let Some(selected_square) = self.ui.selected_square else {
            return;
        };

        let cursor_square = self.ui.cursor_coordinates.into();

        let actual_selected_coords =
            flip_square_if_needed(selected_square, self.runtime.is_flipped());
        let actual_cursor_coords = flip_square_if_needed(cursor_square, self.runtime.is_flipped());

        if actual_cursor_coords == actual_selected_coords {
            self.ui.unselect_cell();
            return;
        }

        // Check authorized positions before taking any action.
        let authorized_positions = self
            .runtime
            .get_authorized_positions(self.runtime.turn(), &actual_selected_coords);

        if !authorized_positions.contains(&actual_cursor_coords) {
            let piece_at_destination = self
                .runtime
                .get_piece_color_at_square(&actual_cursor_coords);

            if piece_at_destination == Some(self.runtime.turn()) {
                self.select_cell();
            } else {
                self.ui.unselect_cell();
            }
            return;
        }

        self.execute_move(actual_selected_coords, actual_cursor_coords);
        self.ui.unselect_cell();

        self.runtime.update_game_state();

        if !self.runtime.state().is_promotion() {
            self.runtime.start_clock_if_needed();
            self.runtime.switch_turn();
        } else {
            self.runtime.pause_clock();
        }

        self.handle_after_move_board_flip();
        self.runtime.on_player_move();
    }

    /// Method applied after selecting a cell on the board
    pub fn select_cell(&mut self) {
        // Is really needed?
        let square: Square = self.ui.cursor_coordinates.into();
        let square: Square = Coord::from(square).into();

        let actual_square = flip_square_if_needed(square, self.runtime.is_flipped());

        // Check if there is a piece on the cell and if it's the right color
        let piece_color = self.runtime.get_piece_color_at_square(&actual_square);

        if self.runtime.get_role_at_square(&actual_square).is_none()
            || piece_color != Some(self.runtime.turn())
        {
            return;
        }

        // Check if the piece on the cell can move before selecting it
        let authorized_positions = self
            .runtime
            .get_authorized_positions(self.runtime.turn(), &actual_square);

        if authorized_positions.is_empty() {
            return;
        }

        // We already verified the piece color matches player_turn above, so we can proceed
        self.ui.selected_square = Some(square);
        self.ui.old_cursor_position = Some(self.ui.cursor_coordinates);
    }

    pub fn apply_player_move(&mut self, from: Square, to: Square, promotion: Option<Role>) -> bool {
        let executed_move = self.runtime.play(from, to, promotion);

        let Some(_executed_move) = executed_move else {
            return false;
        };

        ui::sound::play_move_sound();

        self.runtime.update_game_state();

        if !self.runtime.state().is_promotion() {
            self.runtime.start_clock_if_needed();
            self.runtime.switch_turn();
        } else {
            self.runtime.pause_clock();
        }

        self.handle_after_move_board_flip();
        self.runtime.on_player_move();

        true
    }

    pub fn clock_state(&self) -> ClockState {
        self.runtime.clock().state()
    }

    pub fn sync_player_turn_with_position(&mut self) {
        self.runtime.sync_turn_with_position();
    }

    pub fn navigate_history_next(&mut self) {
        let is_solo_mode = self.runtime.history_is_solo();
        if self.runtime.navigate_history_next(is_solo_mode) {
            self.runtime.sync_turn_with_position();
        }
    }

    pub fn navigate_history_previous(&mut self) {
        let is_solo_mode = self.runtime.history_is_solo();
        if self.runtime.navigate_history_previous(is_solo_mode) {
            self.runtime.sync_turn_with_position();
        }
    }

    /// Method to promote a pawn
    fn promote_piece(&mut self, role: Role, should_flip: bool) {
        if let Some(last_move) = self.runtime.moves().last().cloned() {
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
            self.runtime.undo_last_move();

            // Re-execute the move with the correct promotion
            self.runtime
                .play_and_record(from_square, to_square, Some(role), |_| Move::Normal {
                    role: Role::Pawn,
                    from: from_square,
                    capture,
                    to: to_square,
                    promotion: Some(role),
                });
        }

        self.runtime.start_playing();
        if should_flip && !self.runtime.is_draw() && !self.runtime.is_checkmate() {
            self.runtime.flip();
        }
    }

    /// Move a piece from a cell to another
    pub fn execute_move(&mut self, from: Square, to: Square) {
        // Check if moving a piece and get the piece type
        let Some(role_from) = self.runtime.get_role_at_square(&from) else {
            return;
        };

        let executed_move = self
            .runtime
            .play_and_record(from, to, None, |executed_move| {
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
            ui::sound::play_move_sound();
        }
    }
}

/// Shared handle to the active game. Pages and services borrow it for the
/// duration of one operation.
pub type GameHandle = Rc<RefCell<Game>>;

/// Create a shared game handle.
pub fn game_handle(game: Game) -> GameHandle {
    Rc::new(RefCell::new(game))
}
