//! Game session state and move application.

use chess_tui_core::{Board, Clock, GameOutcome, GameState, PendingPromotion};
use shakmaty::{Color, Position};
use std::ops::{Deref, DerefMut};
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
            GameState::Finished(GameOutcome::Draw)
        } else if let Some(promotion) = self.pending_promotion_from_board() {
            GameState::Promotion(promotion)
        } else {
            self.clock.resume(Instant::now());
            return;
        };

        self.state = new_state;
        self.clock.pause(Instant::now());
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

    pub fn fen(&self) -> String {
        self.board.fen()
    }
}

impl Deref for GameSession {
    type Target = Board;

    fn deref(&self) -> &Self::Target {
        &self.board
    }
}

impl DerefMut for GameSession {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.board
    }
}
