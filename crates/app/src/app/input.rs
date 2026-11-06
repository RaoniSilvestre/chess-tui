//! Board cursor movement and move submission.

use crate::App;
use model::Coord;
use model::square_utils::flip_square_if_needed;
use shakmaty::CastlingMode;

impl App {
    /// Moves the board cursor one square to the left.
    pub fn go_left_in_game(&mut self) {
        self.game_mut().ui.cursor_left();
    }

    /// Moves the board cursor one square to the right.
    pub fn go_right_in_game(&mut self) {
        self.game_mut().ui.cursor_right();
    }

    /// Moves the board cursor one square up.
    pub fn go_up_in_game(&mut self) {
        self.game_mut().ui.cursor_up();
    }

    /// Moves the board cursor one square down.
    pub fn go_down_in_game(&mut self) {
        self.game_mut().ui.cursor_down();
    }

    /// Handles a confirm/select action on the current cursor square. Dispatches to promotion
    /// handling or normal move logic depending on game state, and validates puzzle moves after execution.
    pub fn process_cell_click(&mut self) {
        // Handle promotion directly (like mouse handler does)
        // Note: Promotion state should always allow input, even if turn has switched
        // because the player needs to select the promotion piece after making the move
        if self.game_mut().runtime.state().is_promotion() {
            // Track if the move was correct (for puzzle mode)
            let mut move_was_correct = true;

            // If we have a pending promotion move, validate it now with the selected promotion piece
            let pending = self.game().runtime.promotion();
            if let Some(pending) = pending {
                let from = pending.from();
                let to = pending.to();
                // Get the promotion piece from the cursor
                let promotion_char = match self.game_mut().ui.promotion_cursor {
                    0 => 'q', // Queen
                    1 => 'r', // Rook
                    2 => 'b', // Bishop
                    3 => 'n', // Knight
                    _ => 'q', // Default to queen
                };

                // Construct full UCI move with promotion piece
                let move_uci = format!("{}{}{}", from, to, promotion_char);

                // Validate the puzzle move with the complete UCI
                let mut puzzle_game = self.lichess_state.puzzle_game.take();
                if let Some(puzzle_game) = puzzle_game.as_mut() {
                    let token = self.lichess_state.token.clone();
                    let (is_correct, message) = {
                        let mut game = self.game_mut();
                        puzzle_game.validate_move(move_uci, &mut game, token)
                    };

                    move_was_correct = is_correct;

                    if let Some(msg) = message {
                        if is_correct {
                            self.show_puzzle_end(msg);
                        } else {
                            self.show_error(msg);
                        }
                    }
                }
                self.lichess_state.puzzle_game = puzzle_game;
            }
            // The promotion state carries the move squares. The move is already in
            // history and only needs to be replayed with the selected piece.

            // Only handle promotion if the move was correct (or not in puzzle mode)
            // If incorrect, reset_last_move already removed the move and reset the state
            if move_was_correct || self.lichess_state.puzzle_game.is_none() {
                // Don't flip board in puzzle mode or in multiplayer/Lichess mode
                let should_flip =
                    self.lichess_state.puzzle_game.is_none() && self.game_mut().runtime.flip_after_move();
                self.game_mut().handle_promotion(should_flip);
            } else {
                // Move was incorrect in puzzle mode - ensure game state is reset
                // reset_last_move should have already handled this, but make sure
                if self.game_mut().runtime.state().is_promotion() {
                    self.game_mut().runtime.start_playing();
                }
            }

            self.check_and_show_game_end();
        } else {
            // In multiplayer/Lichess mode, only allow input if it's our turn (but not for promotion, handled above)
            // Check authorized positions before taking any action.

            // Store move info before execution for puzzle validation
            let puzzle_move_info = if self.lichess_state.puzzle_game.is_some()
                && self.game_mut().ui.is_cell_selected()
            {
                if let Some(selected_square) = self.game_mut().ui.selected_square {
                    let cursor_square = self.game_mut().ui.cursor_coordinates.into();

                    let from =
                        flip_square_if_needed(selected_square, self.game_mut().runtime.is_flipped());
                    let to = flip_square_if_needed(cursor_square, self.game_mut().runtime.is_flipped());

                    // Check if it is a valid move. This could be improved by having a single source
                    // of truth. Maybe the game return an Option<(Square, Square)> so anyone can see
                    // if a move was actually made by a movement.
                    let authorized_positions = self.game_mut()
                        .runtime
                        .get_authorized_positions(self.game_mut().runtime.turn(), &from);

                    if authorized_positions.contains(&to) {
                        Some((from, to))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            self.game_mut().handle_cell_click();

            // Validate puzzle moves after execution. Promotion validation happens
            // when the player chooses the piece.
            if !self.game_mut().runtime.state().is_promotion()
                && let Some((from, to)) = puzzle_move_info
            {
                self.validate_puzzle_move_after_execution(from, to);
            }

            // Ensure board stays unflipped in puzzle mode
            if self.lichess_state.puzzle_game.is_some() {
                self.game_mut().runtime.set_flipped(false);
            }

            self.check_and_show_game_end();
        }
    }

    /// Plays a move typed in the move input popup. In puzzle mode the move is validated
    /// against the solution and the board stays unflipped. Returns `true` if the move was played.
    pub fn submit_typed_move(&mut self, chess_move: &shakmaty::Move) -> bool {
        let Some(from) = chess_move.from() else {
            return false;
        };

        if !self.game_mut()
            .apply_player_move(from, chess_move.to(), chess_move.promotion())
        {
            return false;
        }

        if self.lichess_state.puzzle_game.is_some() {
            // Standard UCI so castling is "e1g1" (not king to rook) and promotions keep their piece
            let move_uci = chess_move.to_uci(CastlingMode::Standard).to_string();
            self.validate_puzzle_move_uci(move_uci);
            self.game_mut().runtime.set_flipped(false);
        }

        self.check_and_show_game_end();
        true
    }

    /// Attempts to move the selected piece to `target_square` via mouse click.
    /// Returns `true` if the target is a legal destination and the move was executed.
    pub fn try_mouse_move(&mut self, target_square: shakmaty::Square, coords: Coord) -> bool {
        if self.game_mut().ui.selected_square.is_none() {
            return false;
        }

        let Some(selected_square) = self.game_mut().ui.selected_square else {
            return false;
        };
        let authorized_positions = self.game_mut().runtime.get_authorized_positions(
            self.game_mut().runtime.turn(),
            &flip_square_if_needed(selected_square, self.game_mut().runtime.is_flipped()),
        );

        // Check if target square is a valid move destination
        if authorized_positions.contains(&flip_square_if_needed(
            target_square,
            self.game_mut().runtime.is_flipped(),
        )) {
            self.game_mut().ui.cursor_coordinates = coords;
            self.process_cell_click();
            return true;
        }
        false
    }
}
