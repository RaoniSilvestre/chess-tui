//! Shared board controller used by all concrete game pages.
//!
//! The board owns the `Game` it displays. Page-specific concerns (bot engine,
//! network stream, puzzle validation, resume persistence) stay in the concrete
//! page and are reached through [`BoardAction`].

use model::square_utils::flip_square_if_needed;
use model::{ClockState, GameOutcome, GameState};
use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::{Frame, layout::Rect};
use session::Game;
use shakmaty::Square;

use crate::globals::Globals;
use crate::page::{PageAction, PageRequest};

/// Actions that need page-specific handling. Everything generic is handled
/// directly by the board.
pub(crate) enum BoardAction {
    None,
    Page(PageAction),
    Restart,
    OpenMoveInput,
    PuzzleMove { move_uci: String },
}

pub(crate) struct GameBoard {
    pub(crate) game: Game,
    pub(crate) end_screen_visible: bool,
    pub(crate) end_screen_dismissed: bool,
    puzzle: bool,
}

impl GameBoard {
    pub(crate) fn new() -> Self {
        Self {
            game: Game::default(),
            end_screen_visible: false,
            end_screen_dismissed: false,
            puzzle: false,
        }
    }

    pub(crate) fn new_puzzle() -> Self {
        let mut board = Self::new();
        board.puzzle = true;
        board
    }

    pub(crate) fn game(&self) -> &Game {
        &self.game
    }

    pub(crate) fn game_mut(&mut self) -> &mut Game {
        &mut self.game
    }

    pub(crate) fn handle_end_screen_key(&mut self, key: KeyEvent) -> BoardAction {
        match key.code {
            KeyCode::Char('h' | 'H') | KeyCode::Esc => {
                self.end_screen_visible = false;
                self.end_screen_dismissed = true;
                BoardAction::None
            }
            KeyCode::Char('r' | 'R') => BoardAction::Restart,
            KeyCode::Char('b' | 'B') => BoardAction::Page(PageAction::Navigate(PageRequest::Home)),
            _ => BoardAction::None,
        }
    }

    pub(crate) fn handle_common_key(
        &mut self,
        key: KeyEvent,
        globals: &mut Globals,
    ) -> BoardAction {
        let is_playing = self.game.runtime.state().is_playing();

        match key.code {
            KeyCode::Up | KeyCode::Char('k') if is_playing => self.game.ui.cursor_up(),
            KeyCode::Down | KeyCode::Char('j') if is_playing => self.game.ui.cursor_down(),
            KeyCode::Right | KeyCode::Char('l') => match self.game.runtime.state() {
                GameState::Promotion(_) => self.game.ui.cursor_right_promotion(),
                GameState::Playing => self.game.ui.cursor_right(),
                _ => {}
            },
            KeyCode::Left | KeyCode::Char('h') => match self.game.runtime.state() {
                GameState::Promotion(_) => self.game.ui.cursor_left_promotion(),
                GameState::Playing => self.game.ui.cursor_left(),
                _ => {}
            },
            KeyCode::Char(' ') | KeyCode::Enter => {
                let action = self.process_cell_click();
                if self.game.runtime.state().is_finished() && !self.end_screen_dismissed {
                    self.end_screen_visible = true;
                }
                return action;
            }
            KeyCode::Char('?') => {
                globals.show_help();
                return BoardAction::None;
            }
            KeyCode::Char('s' | 'S') => {
                globals.theme.cycle(true);
                globals.persist_ui_config();
                globals.theme.apply_to_game(&mut self.game);
                return BoardAction::Page(PageAction::Handled);
            }
            KeyCode::Char('m') => return BoardAction::OpenMoveInput,
            KeyCode::Esc => self.game.ui.unselect_cell(),
            KeyCode::Char('q') => return BoardAction::Page(PageAction::Quit),
            KeyCode::Char('c' | 'C') if key.modifiers == KeyModifiers::CONTROL => {
                return BoardAction::Page(PageAction::Quit);
            }
            _ => {}
        }

        BoardAction::None
    }

    pub(crate) fn navigate_history_next(&mut self) {
        if !self.puzzle && !self.game.runtime.state().is_finished() {
            self.game.navigate_history_next();
        }
    }

    pub(crate) fn navigate_history_previous(&mut self) {
        if !self.puzzle && !self.game.runtime.state().is_finished() {
            self.game.navigate_history_previous();
        }
    }

    fn process_cell_click(&mut self) -> BoardAction {
        if self.game.runtime.state().is_promotion() {
            let Some(pending) = self.game.runtime.promotion() else {
                return BoardAction::None;
            };
            let promotion_char = match self.game.ui.promotion_cursor {
                0 => 'q',
                1 => 'r',
                2 => 'b',
                3 => 'n',
                _ => 'q',
            };
            let move_uci = format!("{}{}{}", pending.from(), pending.to(), promotion_char);

            if self.puzzle {
                return BoardAction::PuzzleMove { move_uci };
            }

            let should_flip = self.game.runtime.flip_after_move();
            self.game.handle_promotion(should_flip);
            return BoardAction::None;
        }

        let puzzle_move_info = if self.puzzle && self.game.ui.is_cell_selected() {
            self.game.ui.selected_square.and_then(|selected_square| {
                let cursor_square = self.game.ui.cursor_coordinates.into();
                let from = flip_square_if_needed(selected_square, self.game.runtime.is_flipped());
                let to = flip_square_if_needed(cursor_square, self.game.runtime.is_flipped());
                let authorized = self
                    .game
                    .runtime
                    .get_authorized_positions(self.game.runtime.turn(), &from);
                authorized.contains(&to).then_some((from, to))
            })
        } else {
            None
        };

        self.game.handle_cell_click();

        if self.puzzle {
            self.game.runtime.set_flipped(false);
        }

        if !self.game.runtime.state().is_promotion()
            && let Some((from, to)) = puzzle_move_info
        {
            return BoardAction::PuzzleMove {
                move_uci: format!("{from}{to}"),
            };
        }

        BoardAction::None
    }

    fn try_mouse_move(&mut self, target_square: Square, coords: model::Coord) -> BoardAction {
        let Some(selected_square) = self.game.ui.selected_square else {
            return BoardAction::None;
        };
        let authorized = self.game.runtime.get_authorized_positions(
            self.game.runtime.turn(),
            &flip_square_if_needed(selected_square, self.game.runtime.is_flipped()),
        );

        if authorized.contains(&flip_square_if_needed(
            target_square,
            self.game.runtime.is_flipped(),
        )) {
            self.game.ui.cursor_coordinates = coords;
            return self.process_cell_click();
        }
        BoardAction::None
    }

    pub(crate) fn handle_mouse(&mut self, mouse: MouseEvent) -> BoardAction {
        if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
            return BoardAction::None;
        }

        if self.game.runtime.state().is_finished() || !self.game.runtime.can_player_move() {
            return BoardAction::None;
        }

        if self.game.runtime.state().is_promotion() {
            let x = (mouse.column.saturating_sub(self.game.ui.top_x)) / self.game.ui.width.max(1);
            let y = (mouse.row.saturating_sub(self.game.ui.top_y)) / self.game.ui.height.max(1);
            if x > 3 || y > 0 {
                return BoardAction::None;
            }
            self.game.ui.promotion_cursor = x as i8;

            let Some(pending) = self.game.runtime.promotion() else {
                return BoardAction::None;
            };
            let promotion_char = match self.game.ui.promotion_cursor {
                0 => 'q',
                1 => 'r',
                2 => 'b',
                3 => 'n',
                _ => 'q',
            };
            let move_uci = format!("{}{}{}", pending.from(), pending.to(), promotion_char);

            if self.puzzle {
                return BoardAction::PuzzleMove { move_uci };
            }

            let should_flip = self.game.runtime.flip_after_move();
            self.game.handle_promotion(should_flip);
            return BoardAction::None;
        }

        if mouse.column < self.game.ui.top_x || mouse.row < self.game.ui.top_y {
            return BoardAction::None;
        }
        if self.game.ui.width == 0 || self.game.ui.height == 0 {
            return BoardAction::None;
        }

        let x = (mouse.column - self.game.ui.top_x) / self.game.ui.width;
        let y = (mouse.row - self.game.ui.top_y) / self.game.ui.height;
        if x > 7 || y > 7 {
            return BoardAction::None;
        }

        self.game.ui.mouse_used = true;
        let coords = model::Coord::new(y as u8, x as u8);
        let square: Square = coords.into();
        let piece_color = self
            .game
            .runtime
            .get_piece_color_at_square(&flip_square_if_needed(
                square,
                self.game.runtime.is_flipped(),
            ));

        if piece_color.is_none() {
            if self.game.ui.selected_square.is_none() {
                return BoardAction::None;
            }
            return self.try_mouse_move(square, coords);
        }

        if piece_color == Some(self.game.runtime.turn()) {
            if let Some(selected_square) = self.game.ui.selected_square {
                let actual_selected =
                    flip_square_if_needed(selected_square, self.game.runtime.is_flipped());
                let actual_clicked = flip_square_if_needed(square, self.game.runtime.is_flipped());
                let selected_role = self.game.runtime.get_role_at_square(&actual_selected);
                let clicked_role = self.game.runtime.get_role_at_square(&actual_clicked);

                if selected_role == Some(shakmaty::Role::King)
                    && clicked_role == Some(shakmaty::Role::Rook)
                {
                    let castling_dest = match actual_clicked {
                        Square::H1 => Square::G1,
                        Square::A1 => Square::C1,
                        Square::H8 => Square::G8,
                        Square::A8 => Square::C8,
                        _ => {
                            let action = self.try_mouse_move(square, coords);
                            if !matches!(action, BoardAction::None) {
                                return action;
                            }
                            self.game.ui.selected_square = Some(square);
                            return BoardAction::None;
                        }
                    };

                    let authorized = self
                        .game
                        .runtime
                        .get_authorized_positions(self.game.runtime.turn(), &actual_selected);
                    if authorized.contains(&castling_dest) {
                        let castling_coords = model::Coord::from(flip_square_if_needed(
                            castling_dest,
                            self.game.runtime.is_flipped(),
                        ));
                        let target =
                            flip_square_if_needed(castling_dest, self.game.runtime.is_flipped());
                        let action = self.try_mouse_move(target, castling_coords);
                        if !matches!(action, BoardAction::None) {
                            return action;
                        }
                    }
                }

                let action = self.try_mouse_move(square, coords);
                if !matches!(action, BoardAction::None) {
                    return action;
                }
            }
            self.game.ui.selected_square = Some(square);
            return BoardAction::None;
        }

        if self.game.ui.selected_square.is_some() {
            return self.try_mouse_move(square, coords);
        }

        BoardAction::None
    }

    pub(crate) fn tick(&mut self) {
        self.game.runtime.tick();

        let clock = self.game.runtime.clock().clone();
        if let ClockState::Running { active_color, .. } = clock.state()
            && clock
                .get_time(active_color, std::time::Instant::now())
                .is_zero()
        {
            self.game.runtime.force_time_up(active_color);
        }

        if let ClockState::TimeUp { loser_color } = self.game.runtime.clock().state()
            && !self.game.runtime.state().is_finished()
        {
            self.game.runtime.finish(GameOutcome::Timeout {
                winner: loser_color.other(),
            });
        }

        if self.game.runtime.state().is_finished() && !self.end_screen_dismissed {
            self.end_screen_visible = true;
        }
    }

    /// Recomputes the terminal state after an event. Returns true when the game
    /// is finished (so the owning page can clear its resume save).
    pub(crate) fn update(&mut self) -> bool {
        self.game.runtime.update_game_state();
        if self.game.runtime.state().is_finished() {
            if !self.end_screen_dismissed {
                self.end_screen_visible = true;
            }
            true
        } else {
            false
        }
    }

    pub(crate) fn render(&mut self, frame: &mut Frame<'_>, area: Rect, globals: &mut Globals) {
        ui::game::render_game_ui(
            frame,
            area,
            &self.game.runtime,
            &mut self.game.ui,
            globals.kitty_pieces.as_mut(),
            globals.kitty_graphics_enabled,
            self.puzzle,
            self.end_screen_visible,
        );
    }

    pub(crate) fn submit_move(&mut self, chess_move: &shakmaty::Move) -> BoardAction {
        let Some(from) = chess_move.from() else {
            return BoardAction::None;
        };
        if !self
            .game
            .apply_player_move(from, chess_move.to(), chess_move.promotion())
        {
            return BoardAction::None;
        }
        if self.puzzle {
            self.game.runtime.set_flipped(false);
            let move_uci = chess_move
                .to_uci(shakmaty::CastlingMode::Standard)
                .to_string();
            return BoardAction::PuzzleMove { move_uci };
        }
        if self.game.runtime.state().is_finished() && !self.end_screen_dismissed {
            self.end_screen_visible = true;
        }
        BoardAction::None
    }
}
