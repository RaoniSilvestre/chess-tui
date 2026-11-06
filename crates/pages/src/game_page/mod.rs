//! Shared game page for Solo, Bot, Multiplayer, and Lichess games.

mod popup_end;
mod popup_promotion;
mod render;

pub use render::render_game_ui;

use app::game_state::Game;
use app::{App, GamePageKind, Nav, Page};
use model::Coord;
use model::GameState;
use model::square_utils::flip_square_if_needed;
use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::{Frame, layout::Rect};
use shakmaty::Square;

/// A game page owns the active [`Game`] while it handles input, rendering, and
/// ticking. Between events the game is handed back to the application so global
/// popups and services can access it.
pub struct GamePage {
    kind: GamePageKind,
    game: Option<Game>,
}

impl GamePage {
    pub fn new(kind: GamePageKind) -> Self {
        Self { kind, game: None }
    }

    fn take_game(&mut self, app: &mut App) -> &mut Game {
        if self.game.is_none() {
            self.game = Some(app.game_take());
        }
        self.game
            .as_mut()
            .expect("game page always holds a game after take_game")
    }

    fn put_game(&mut self, app: &mut App) {
        if let Some(game) = self.game.take() {
            app.game_put(game);
        }
    }

    fn handle_solo_key(&mut self, key: KeyEvent, app: &mut App) {
        match key.code {
            KeyCode::Char('r') => {
                self.put_game(app);
                app.game_request_restart();
            }
            KeyCode::Char('b') => {
                self.put_game(app);
                app.game_reset_home();
            }
            KeyCode::Char('n' | 'N') => {
                let game = self.take_game(app);
                if !app.game_is_puzzle() && !game.runtime.state().is_finished() {
                    game.navigate_history_next();
                }
            }
            KeyCode::Char('p' | 'P') => {
                let game = self.take_game(app);
                if !app.game_is_puzzle() && !game.runtime.state().is_finished() {
                    game.navigate_history_previous();
                }
            }
            KeyCode::Char('t' | 'T') if app.game_is_puzzle() => {
                let game = self.take_game(app);
                app.game_puzzle_hint(game);
            }
            _ => {
                let game = self.take_game(app);
                Self::handle_chess_key(game, key, app);
            }
        }
    }

    fn handle_bot_key(&mut self, key: KeyEvent, app: &mut App) {
        match key.code {
            KeyCode::Char('r') => {
                self.put_game(app);
                app.game_request_restart();
            }
            KeyCode::Char('b') => {
                self.put_game(app);
                app.game_reset_home();
            }
            _ => {
                let game = self.take_game(app);
                Self::handle_chess_key(game, key, app);
            }
        }
    }

    fn handle_multiplayer_key(&mut self, key: KeyEvent, app: &mut App) {
        if key.code == KeyCode::Char('b') {
            self.put_game(app);
            app.game_reset_home();
            return;
        }
        let game = self.take_game(app);
        Self::handle_chess_key(game, key, app);
    }

    fn handle_chess_key(game: &mut Game, key: KeyEvent, app: &mut App) {
        let is_playing = game.runtime.state().is_playing();

        match key.code {
            KeyCode::Up | KeyCode::Char('k') if is_playing => game.ui.cursor_up(),
            KeyCode::Down | KeyCode::Char('j') if is_playing => game.ui.cursor_down(),

            KeyCode::Right | KeyCode::Char('l') => match game.runtime.state() {
                GameState::Promotion(_) => game.ui.cursor_right_promotion(),
                GameState::Playing => game.ui.cursor_right(),
                _ => {}
            },
            KeyCode::Left | KeyCode::Char('h') => match game.runtime.state() {
                GameState::Promotion(_) => game.ui.cursor_left_promotion(),
                GameState::Playing => game.ui.cursor_left(),
                GameState::Finished(_) => {
                    if !app.game_is_puzzle() {
                        app.game_toggle_end_screen();
                    }
                }
            },
            KeyCode::Char(' ') | KeyCode::Enter => {
                Self::process_cell_click(game, app);
            }
            KeyCode::Char('?') => app.open_help(),
            KeyCode::Char('s' | 'S') => app.game_cycle_skin(true),
            KeyCode::Char('m') => app.game_open_move_input(),
            KeyCode::Esc => game.ui.unselect_cell(),
            KeyCode::Char('q') => app.quit(),
            KeyCode::Char('c' | 'C') if key.modifiers == KeyModifiers::CONTROL => app.quit(),
            _ => {}
        }
    }

    fn process_cell_click(game: &mut Game, app: &mut App) {
        if game.runtime.state().is_promotion() {
            let mut move_was_correct = true;

            if let Some(pending) = game.runtime.promotion() {
                let from = pending.from();
                let to = pending.to();
                let promotion_char = match game.ui.promotion_cursor {
                    0 => 'q',
                    1 => 'r',
                    2 => 'b',
                    3 => 'n',
                    _ => 'q',
                };

                let move_uci = format!("{}{}{}", from, to, promotion_char);
                if app.game_is_puzzle() {
                    move_was_correct = app.game_validate_promotion(game, move_uci);
                }
            }

            if move_was_correct || !app.game_is_puzzle() {
                let should_flip = !app.game_is_puzzle() && game.runtime.flip_after_move();
                game.handle_promotion(should_flip);
            } else if game.runtime.state().is_promotion() {
                game.runtime.start_playing();
            }

            app.game_check_end(game);
        } else {
            let puzzle_move_info = if app.game_is_puzzle() && game.ui.is_cell_selected() {
                if let Some(selected_square) = game.ui.selected_square {
                    let cursor_square = game.ui.cursor_coordinates.into();
                    let from = flip_square_if_needed(selected_square, game.runtime.is_flipped());
                    let to = flip_square_if_needed(cursor_square, game.runtime.is_flipped());
                    let authorized_positions = game
                        .runtime
                        .get_authorized_positions(game.runtime.turn(), &from);
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

            game.handle_cell_click();

            if !game.runtime.state().is_promotion()
                && let Some((from, to)) = puzzle_move_info
            {
                app.game_validate_move(game, from, to);
            }

            if app.game_is_puzzle() {
                game.runtime.set_flipped(false);
            }

            app.game_check_end(game);
        }
    }

    fn try_mouse_move(
        game: &mut Game,
        target_square: Square,
        coords: Coord,
        app: &mut App,
    ) -> bool {
        let Some(selected_square) = game.ui.selected_square else {
            return false;
        };
        let authorized_positions = game.runtime.get_authorized_positions(
            game.runtime.turn(),
            &flip_square_if_needed(selected_square, game.runtime.is_flipped()),
        );

        if authorized_positions.contains(&flip_square_if_needed(
            target_square,
            game.runtime.is_flipped(),
        )) {
            game.ui.cursor_coordinates = coords;
            Self::process_cell_click(game, app);
            return true;
        }
        false
    }

    fn handle_game_mouse(&mut self, mouse: MouseEvent, app: &mut App) {
        if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
            return;
        }

        let kind = self.kind;
        let game = self.take_game(app);

        if game.runtime.state().is_finished() {
            return;
        }

        if game.runtime.state().is_promotion() {
            let x = (mouse.column.saturating_sub(game.ui.top_x)) / game.ui.width.max(1);
            let y = (mouse.row.saturating_sub(game.ui.top_y)) / game.ui.height.max(1);
            if x > 3 || y > 0 {
                return;
            }
            game.ui.promotion_cursor = x as i8;

            let mut move_was_correct = true;
            if let Some(pending) = game.runtime.promotion() {
                let from = pending.from();
                let to = pending.to();
                let promotion_char = match game.ui.promotion_cursor {
                    0 => 'q',
                    1 => 'r',
                    2 => 'b',
                    3 => 'n',
                    _ => 'q',
                };
                let move_uci = format!("{}{}{}", from, to, promotion_char);
                if app.game_is_puzzle() {
                    move_was_correct = app.game_validate_promotion(game, move_uci);
                }
            }

            if move_was_correct || !app.game_is_puzzle() {
                let should_flip = !app.game_is_puzzle() && game.runtime.flip_after_move();
                game.handle_promotion(should_flip);
            } else if game.runtime.state().is_promotion() {
                game.runtime.start_playing();
            }
            return;
        }

        if kind == GamePageKind::Lichess
            && let Some(my_color) = app.game_selected_color()
            && game.runtime.turn() != my_color
        {
            return;
        }

        if mouse.column < game.ui.top_x || mouse.row < game.ui.top_y {
            return;
        }
        if game.ui.width == 0 || game.ui.height == 0 {
            return;
        }

        let x = (mouse.column - game.ui.top_x) / game.ui.width;
        let y = (mouse.row - game.ui.top_y) / game.ui.height;
        if x > 7 || y > 7 {
            return;
        }

        game.ui.mouse_used = true;
        let coords = Coord::new(y as u8, x as u8);
        let square: Square = coords.into();
        let piece_color = game
            .runtime
            .get_piece_color_at_square(&flip_square_if_needed(square, game.runtime.is_flipped()));

        if piece_color.is_none() {
            if game.ui.selected_square.is_none() {
                return;
            }
            Self::try_mouse_move(game, square, coords, app);
        } else if piece_color == Some(game.runtime.turn()) {
            if let Some(selected_square) = game.ui.selected_square {
                let actual_selected =
                    flip_square_if_needed(selected_square, game.runtime.is_flipped());
                let actual_clicked = flip_square_if_needed(square, game.runtime.is_flipped());
                let selected_role = game.runtime.get_role_at_square(&actual_selected);
                let clicked_role = game.runtime.get_role_at_square(&actual_clicked);

                if selected_role == Some(shakmaty::Role::King)
                    && clicked_role == Some(shakmaty::Role::Rook)
                {
                    let castling_dest = match actual_clicked {
                        Square::H1 => Square::G1,
                        Square::A1 => Square::C1,
                        Square::H8 => Square::G8,
                        Square::A8 => Square::C8,
                        _ => {
                            if Self::try_mouse_move(game, square, coords, app) {
                                return;
                            }
                            game.ui.selected_square = Some(square);
                            return;
                        }
                    };

                    let authorized_positions = game
                        .runtime
                        .get_authorized_positions(game.runtime.turn(), &actual_selected);
                    if authorized_positions.contains(&castling_dest) {
                        let castling_coords = Coord::from(flip_square_if_needed(
                            castling_dest,
                            game.runtime.is_flipped(),
                        ));
                        if Self::try_mouse_move(
                            game,
                            flip_square_if_needed(castling_dest, game.runtime.is_flipped()),
                            castling_coords,
                            app,
                        ) {
                            return;
                        }
                    }
                }

                if Self::try_mouse_move(game, square, coords, app) {
                    return;
                }
            }
            game.ui.selected_square = Some(square);
        } else if game.ui.selected_square.is_some() {
            Self::try_mouse_move(game, square, coords, app);
        }
    }
}

impl Page for GamePage {
    fn handle_key(&mut self, key: KeyEvent, app: &mut App) -> Nav {
        match self.kind {
            GamePageKind::Solo => self.handle_solo_key(key, app),
            GamePageKind::Bot => self.handle_bot_key(key, app),
            GamePageKind::Multiplayer | GamePageKind::Lichess => {
                self.handle_multiplayer_key(key, app)
            }
        }
        self.put_game(app);
        Nav::None
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, app: &mut App) -> Nav {
        self.handle_game_mouse(mouse, app);
        self.put_game(app);
        Nav::None
    }

    fn tick(&mut self, app: &mut App) -> Nav {
        self.take_game(app).runtime.tick();
        self.put_game(app);
        Nav::None
    }

    fn game_kind(&self) -> Option<GamePageKind> {
        Some(self.kind)
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, app: &mut App) {
        self.put_game(app);
        if app.game_prepare(self.kind) {
            super::game_page::render_game_ui(
                frame,
                area,
                &mut app.game,
                &mut app.kitty_pieces,
                app.kitty_graphics_enabled,
                app.lichess_state.puzzle_game.is_some(),
                app.ui_state.end_screen_visible,
            );
        }
    }
}
