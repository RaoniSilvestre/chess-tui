//! Game session state and move application.

use chess_tui_core::{
    Board, Clock, DrawReason, GameOutcome, GameState, HistoryError, PendingPromotion,
};
use shakmaty::{Chess, Color, Move, Piece, Position, Role, Square};
use std::time::Instant;

/// The mutable state of one chess game: board, clock, turn, and phase/outcome.
///
/// The board is exposed through `Deref` for convenience while the application
/// is migrated to explicit session methods. Prefer [`GameSession::board`] in
/// new code.
#[derive(Clone, Debug)]
pub struct GameSession {
    board: Board,
    clock: Clock,
    turn: Color,
    state: GameState,
}

impl Default for GameSession {
    fn default() -> Self {
        Self {
            board: Board::default(),
            clock: Clock::default(),
            turn: Color::White,
            state: GameState::Playing,
        }
    }
}

impl GameSession {
    pub fn new(board: Board, turn: Color) -> Self {
        Self {
            board,
            turn,
            ..Self::default()
        }
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn board_mut(&mut self) -> &mut Board {
        &mut self.board
    }

    pub fn clock(&self) -> &Clock {
        &self.clock
    }

    pub fn clock_mut(&mut self) -> &mut Clock {
        &mut self.clock
    }

    pub fn set_clock(&mut self, clock: Clock) {
        self.clock = clock;
    }

    pub fn turn(&self) -> Color {
        self.turn
    }

    pub fn set_turn(&mut self, turn: Color) {
        self.turn = turn;
    }

    pub fn state(&self) -> GameState {
        self.state
    }

    pub fn promotion(&self) -> Option<PendingPromotion> {
        self.state.promotion()
    }

    /// Reset the game phase to playing.
    pub fn start_playing(&mut self) {
        self.state = GameState::Playing;
    }

    /// Enter the promotion phase for `promotion`.
    pub fn request_promotion(&mut self, promotion: PendingPromotion) {
        self.state = GameState::Promotion(promotion);
    }

    /// Finish the game with a specific outcome.
    pub fn finish(&mut self, outcome: GameOutcome) {
        self.state = GameState::Finished(outcome);
        self.clock.pause(Instant::now());
    }

    pub fn is_game_over(&self) -> bool {
        self.state.is_finished()
    }

    pub fn is_promotion(&self) -> bool {
        self.state.is_promotion()
    }

    /// Switch the side to move and the active clock.
    pub fn switch_turn(&mut self) {
        self.turn = self.turn.other();
        self.clock.switch_turn(Instant::now());
    }

    /// Recompute the terminal/promotion state from the board.
    pub fn update_game_state(&mut self) {
        if self.state.is_finished() {
            return;
        }

        let new_state = if self.board.is_checkmate() {
            GameState::Finished(GameOutcome::Checkmate {
                winner: self.turn.other(),
            })
        } else if self.board.is_draw() {
            GameState::Finished(GameOutcome::Draw {
                reason: self.draw_reason(),
            })
        } else if let Some(promotion) = self.pending_promotion_from_board() {
            GameState::Promotion(promotion)
        } else {
            self.clock.resume(Instant::now());
            return;
        };

        self.state = new_state;
        self.clock.pause(Instant::now());
    }

    fn draw_reason(&self) -> DrawReason {
        let chess = self.board.current_position();
        if chess.is_stalemate() {
            DrawReason::Stalemate
        } else if self.board.halfmove_clock() >= 50 {
            DrawReason::FiftyMove
        } else if self.board.is_draw_by_repetition() {
            DrawReason::Repetition
        } else if chess.is_insufficient_material() {
            DrawReason::InsufficientMaterial
        } else {
            DrawReason::Unknown
        }
    }

    fn pending_promotion_from_board(&self) -> Option<PendingPromotion> {
        if !self.board.is_latest_move_promotion() {
            return None;
        }
        let last_move = self.board.moves().last()?;
        let from = last_move.from()?;
        Some(PendingPromotion::new(from, last_move.to()))
    }

    /// Start the clock for the side to move if it has not started yet.
    pub fn start_clock_if_needed(&mut self) {
        self.clock.start(Instant::now());
    }

    pub fn pause_clock(&mut self) {
        self.clock.pause(Instant::now());
    }

    pub fn force_time_up(&mut self, color: Color) {
        self.clock.force_time_up(color);
    }

    /// Sync the turn with the current board position.
    pub fn sync_turn_with_position(&mut self) {
        self.turn = self.board.current_position().turn();
    }

    pub fn navigate_history_next(&mut self, is_solo: bool) -> bool {
        if self.board.navigate_history_next(is_solo) {
            self.sync_turn_with_position();
            true
        } else {
            false
        }
    }

    pub fn navigate_history_previous(&mut self, is_solo: bool) -> bool {
        if self.board.navigate_history_previous(is_solo) {
            self.sync_turn_with_position();
            true
        } else {
            false
        }
    }

    pub fn moves(&self) -> &[Move] {
        self.board.moves()
    }

    pub fn positions(&self) -> &[Chess] {
        self.board.positions()
    }

    pub fn current_position(&self) -> &Chess {
        self.board.current_position()
    }

    pub fn taken_pieces(&self) -> &[Piece] {
        self.board.taken_pieces()
    }

    pub fn black_taken_pieces(&self) -> Vec<Role> {
        self.board.black_taken_pieces()
    }

    pub fn white_taken_pieces(&self) -> Vec<Role> {
        self.board.white_taken_pieces()
    }

    pub fn halfmove_clock(&self) -> i32 {
        self.board.halfmove_clock()
    }

    pub fn is_flipped(&self) -> bool {
        self.board.is_flipped()
    }

    pub fn set_flipped(&mut self, flipped: bool) {
        self.board.set_flipped(flipped);
    }

    pub fn flip(&mut self) {
        self.board.flip();
    }

    pub fn history_position(&self) -> Option<usize> {
        self.board.history_position()
    }

    pub fn truncate_history_at(&mut self, index: usize) {
        self.board.truncate_history_at(index);
    }

    pub fn undo_last_move(&mut self) -> Option<Move> {
        self.board.undo_last_move()
    }

    pub fn play(&mut self, from: Square, to: Square, promotion: Option<Role>) -> Option<Move> {
        self.board.play(from, to, promotion)
    }

    pub fn play_and_record<F>(
        &mut self,
        from: Square,
        to: Square,
        promotion: Option<Role>,
        record: F,
    ) -> Option<Move>
    where
        F: FnOnce(&Move) -> Move,
    {
        self.board.play_and_record(from, to, promotion, record)
    }

    pub fn load_history(
        &mut self,
        positions: Vec<Chess>,
        moves: Vec<Move>,
        taken_pieces: Vec<Piece>,
        halfmove_clock: i32,
    ) -> Result<(), HistoryError> {
        self.board
            .load_history(positions, moves, taken_pieces, halfmove_clock)
    }

    pub fn reset(&mut self) {
        self.board.reset();
    }

    pub fn ensure_initial_position(&mut self) {
        self.board.ensure_initial_position();
    }

    pub fn reconstruct_history(&mut self, moves: &str, expected_fen: Option<&str>) {
        self.board.reconstruct_history(moves, expected_fen);
    }

    pub fn record_recovery_move(&mut self, mv: Move) {
        self.board.record_recovery_move(mv);
    }

    pub fn is_draw(&self) -> bool {
        self.board.is_draw()
    }

    pub fn is_checkmate(&self) -> bool {
        self.board.is_checkmate()
    }

    pub fn is_latest_move_promotion(&self) -> bool {
        self.board.is_latest_move_promotion()
    }

    pub fn get_authorized_positions(&self, color: Color, square: &Square) -> Vec<Square> {
        self.board.get_authorized_positions(color, square)
    }

    pub fn get_piece_color_at_square(&self, square: &Square) -> Option<Color> {
        self.board.get_piece_color_at_square(square)
    }

    pub fn get_role_at_square(&self, square: &Square) -> Option<Role> {
        self.board.get_role_at_square(square)
    }

    pub fn fen(&self) -> String {
        self.board.fen()
    }
}
