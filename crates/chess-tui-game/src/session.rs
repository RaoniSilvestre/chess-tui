//! Game session state and move application.

use chess_tui_core::{Clock, GameBoard, GameState};
use shakmaty::{Color, Position, Square};
use std::ops::{Deref, DerefMut};
use std::time::Instant;

/// The mutable state of one chess game: board, clock, turn, and game state.
///
/// The board is exposed through `Deref` for convenience while the application
/// is migrated to explicit session methods. Prefer [`GameSession::board`] in
/// new code.
#[derive(Clone, Debug)]
pub struct GameSession {
    board: GameBoard,
    clock: Clock,
    turn: Color,
    state: GameState,
    pending_promotion: Option<(Square, Square)>,
    ended_by_time: bool,
}

impl Default for GameSession {
    fn default() -> Self {
        Self {
            board: GameBoard::default(),
            clock: Clock::default(),
            turn: Color::White,
            state: GameState::Playing,
            pending_promotion: None,
            ended_by_time: false,
        }
    }
}

impl GameSession {
    pub fn new(board: GameBoard, turn: Color) -> Self {
        Self {
            board,
            turn,
            ..Self::default()
        }
    }

    pub fn board(&self) -> &GameBoard {
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

    pub fn set_state(&mut self, state: GameState) {
        self.state = state;
    }

    pub fn pending_promotion(&self) -> Option<(Square, Square)> {
        self.pending_promotion
    }

    pub fn set_pending_promotion(&mut self, pending: Option<(Square, Square)>) {
        self.pending_promotion = pending;
    }

    pub fn take_pending_promotion(&mut self) -> Option<(Square, Square)> {
        self.pending_promotion.take()
    }

    pub fn ended_by_time(&self) -> bool {
        self.ended_by_time
    }

    pub fn set_ended_by_time(&mut self, ended_by_time: bool) {
        self.ended_by_time = ended_by_time;
    }

    pub fn is_game_over(&self) -> bool {
        matches!(self.state, GameState::Checkmate | GameState::Draw)
    }

    pub fn is_promotion(&self) -> bool {
        self.state == GameState::Promotion
    }

    /// Switch the side to move and the active clock.
    pub fn switch_turn(&mut self) {
        self.turn = self.turn.other();
        self.clock.switch_turn(Instant::now());
    }

    /// Recompute the terminal/promotion state from the board.
    pub fn update_game_state(&mut self) {
        let new_state = if self.board.is_checkmate() {
            GameState::Checkmate
        } else if self.board.is_draw() {
            GameState::Draw
        } else if self.board.is_latest_move_promotion() {
            GameState::Promotion
        } else {
            self.clock.resume(Instant::now());
            return;
        };

        self.state = new_state;
        self.clock.pause(Instant::now());
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
    type Target = GameBoard;

    fn deref(&self) -> &Self::Target {
        &self.board
    }
}

impl DerefMut for GameSession {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.board
    }
}
