//! Shared game page for Solo, Bot, Multiplayer, and Lichess games.

use model::GameState;
use model::square_utils::flip_square_if_needed;
use page::{
    GameAction, GamePageKind, Page, PageAction, PageContext, SettingsAction,
};
use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::{Frame, layout::Rect};
use session::GameHandle;
use shakmaty::Square;

pub use render::render_game_ui;

mod popup_end;
mod popup_promotion;
mod render;

/// A game page owns the active game handle while it handles input and render.
pub struct GamePage {
    kind: GamePageKind,
    game: GameHandle,
    end_screen_visible: bool,
    end_screen_dismissed: bool,
}

impl GamePage {
    pub fn new(kind: GamePageKind, game: GameHandle) -> Self {
        Self {
            kind,
            game,
            end_screen_visible: false,
            end_screen_dismissed: false,
        }
    }

    fn handle_solo_key(&mut self, key: KeyEvent, ctx: &PageContext<'_>) -> PageAction {
        match key.code {
            KeyCode::Char('r') => PageAction::Game(GameAction::RequestRestart),
            KeyCode::Char('b') => PageAction::Game(GameAction::GoHome),
            KeyCode::Char('n' | 'N') => {
                let mut game = self.game.borrow_mut();
                if !ctx.game.is_puzzle && !game.runtime.state().is_finished() {
                    game.navigate_history_next();
                }
                PageAction::None
            }
            KeyCode::Char('p' | 'P') => {
                let mut game = self.game.borrow_mut();
                if !ctx.game.is_puzzle && !game.runtime.state().is_finished() {
                    game.navigate_history_previous();
                }
                PageAction::None
            }
            KeyCode::Char('t' | 'T') if ctx.game.is_puzzle => {
                PageAction::Game(GameAction::PuzzleHint)
            }
            _ => self.handle_chess_key(key, ctx),
        }
    }

    fn handle_bot_key(&mut self, key: KeyEvent, ctx: &PageContext<'_>) -> PageAction {
        match key.code {
            KeyCode::Char('r') => PageAction::Game(GameAction::RequestRestart),
            KeyCode::Char('b') => PageAction::Game(GameAction::GoHome),
            _ => self.handle_chess_key(key, ctx),
        }
    }

    fn handle_multiplayer_key(&mut self, key: KeyEvent, ctx: &PageContext<'_>) -> PageAction {
        if key.code == KeyCode::Char('b') {
            return PageAction::Game(GameAction::GoHome);
        }
        self.handle_chess_key(key, ctx)
    }

    fn handle_chess_key(&mut self, key: KeyEvent, ctx: &PageContext<'_>) -> PageAction {
        let mut game = self.game.borrow_mut();
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
                    if !ctx.game.is_puzzle {
                        return PageAction::Game(GameAction::CheckEnd);
                    }
                }
            },
            KeyCode::Char(' ') | KeyCode::Enter => {
                let action = Self::process_cell_click(&mut game, ctx);
                if game.runtime.state().is_finished() && !self.end_screen_dismissed {
                    self.end_screen_visible = true;
                }
                return action;
            }
            KeyCode::Char('?') => return PageAction::OpenHelp,
            KeyCode::Char('s' | 'S') => {
                return PageAction::Settings(SettingsAction::CycleSkin(true));
            }
            KeyCode::Char('m') => return PageAction::Game(GameAction::OpenMoveInput),
            KeyCode::Esc => game.ui.unselect_cell(),
            KeyCode::Char('q') => return PageAction::Quit,
            KeyCode::Char('c' | 'C') if key.modifiers == KeyModifiers::CONTROL => {
                return PageAction::Quit;
            }
            _ => {}
        }

        PageAction::None
    }

    fn process_cell_click(
        game: &mut session::Game,
        ctx: &PageContext<'_>,
    ) -> PageAction {
        if game.runtime.state().is_promotion() {
            let Some(pending) = game.runtime.promotion() else {
                return PageAction::None;
            };
            let promotion_char = match game.ui.promotion_cursor {
                0 => 'q',
                1 => 'r',
                2 => 'b',
                3 => 'n',
                _ => 'q',
            };
            let move_uci = format!("{}{}{}", pending.from(), pending.to(), promotion_char);

            if ctx.game.is_puzzle {
                return PageAction::Game(GameAction::ValidatePromotion { move_uci });
            }

            let should_flip = game.runtime.flip_after_move();
            game.handle_promotion(should_flip);
            return PageAction::Game(GameAction::CheckEnd);
        }

        let puzzle_move_info = if ctx.game.is_puzzle && game.ui.is_cell_selected() {
            game.ui.selected_square.and_then(|selected_square| {
                let cursor_square = game.ui.cursor_coordinates.into();
                let from = flip_square_if_needed(selected_square, game.runtime.is_flipped());
                let to = flip_square_if_needed(cursor_square, game.runtime.is_flipped());
                let authorized = game
                    .runtime
                    .get_authorized_positions(game.runtime.turn(), &from);
                authorized.contains(&to).then_some((from, to))
            })
        } else {
            None
        };

        game.handle_cell_click();

        if ctx.game.is_puzzle {
            game.runtime.set_flipped(false);
        }

        if !game.runtime.state().is_promotion()
            && let Some((from, to)) = puzzle_move_info
        {
            return PageAction::Game(GameAction::ValidateMove { from, to });
        }

        PageAction::Game(GameAction::CheckEnd)
    }

    fn handle_end_screen_key(&mut self, key: KeyEvent) -> PageAction {
        match key.code {
            KeyCode::Char('h' | 'H') | KeyCode::Esc => {
                self.end_screen_visible = false;
                self.end_screen_dismissed = true;
                PageAction::None
            }
            KeyCode::Char('r' | 'R') => PageAction::Game(GameAction::RequestRestart),
            KeyCode::Char('b' | 'B') => PageAction::Game(GameAction::GoHome),
            _ => PageAction::None,
        }
    }

    fn try_mouse_move(
        game: &mut session::Game,
        target_square: Square,
        coords: model::Coord,
        ctx: &PageContext<'_>,
    ) -> PageAction {
        let Some(selected_square) = game.ui.selected_square else {
            return PageAction::None;
        };
        let authorized = game.runtime.get_authorized_positions(
            game.runtime.turn(),
            &flip_square_if_needed(selected_square, game.runtime.is_flipped()),
        );

        if authorized.contains(&flip_square_if_needed(
            target_square,
            game.runtime.is_flipped(),
        )) {
            game.ui.cursor_coordinates = coords;
            return Self::process_cell_click(game, ctx);
        }
        PageAction::None
    }

    fn handle_game_mouse(&mut self, mouse: MouseEvent, ctx: &PageContext<'_>) -> PageAction {
        if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
            return PageAction::None;
        }

        let kind = self.kind;
        let mut game = self.game.borrow_mut();

        if game.runtime.state().is_finished() {
            return PageAction::None;
        }

        if game.runtime.state().is_promotion() {
            let x = (mouse.column.saturating_sub(game.ui.top_x)) / game.ui.width.max(1);
            let y = (mouse.row.saturating_sub(game.ui.top_y)) / game.ui.height.max(1);
            if x > 3 || y > 0 {
                return PageAction::None;
            }
            game.ui.promotion_cursor = x as i8;

            let Some(pending) = game.runtime.promotion() else {
                return PageAction::None;
            };
            let promotion_char = match game.ui.promotion_cursor {
                0 => 'q',
                1 => 'r',
                2 => 'b',
                3 => 'n',
                _ => 'q',
            };
            let move_uci = format!("{}{}{}", pending.from(), pending.to(), promotion_char);

            if ctx.game.is_puzzle {
                return PageAction::Game(GameAction::ValidatePromotion { move_uci });
            }

            let should_flip = game.runtime.flip_after_move();
            game.handle_promotion(should_flip);
            return PageAction::Game(GameAction::CheckEnd);
        }

        if kind == GamePageKind::Lichess
            && let Some(my_color) = ctx.game.selected_color
            && game.runtime.turn() != my_color
        {
            return PageAction::None;
        }

        if mouse.column < game.ui.top_x || mouse.row < game.ui.top_y {
            return PageAction::None;
        }
        if game.ui.width == 0 || game.ui.height == 0 {
            return PageAction::None;
        }

        let x = (mouse.column - game.ui.top_x) / game.ui.width;
        let y = (mouse.row - game.ui.top_y) / game.ui.height;
        if x > 7 || y > 7 {
            return PageAction::None;
        }

        game.ui.mouse_used = true;
        let coords = model::Coord::new(y as u8, x as u8);
        let square: Square = coords.into();
        let piece_color = game
            .runtime
            .get_piece_color_at_square(&flip_square_if_needed(square, game.runtime.is_flipped()));

        if piece_color.is_none() {
            if game.ui.selected_square.is_none() {
                return PageAction::None;
            }
            return Self::try_mouse_move(&mut game, square, coords, ctx);
        }

        if piece_color == Some(game.runtime.turn()) {
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
                            let action = Self::try_mouse_move(&mut game, square, coords, ctx);
                            if !matches!(action, PageAction::None) {
                                return action;
                            }
                            game.ui.selected_square = Some(square);
                            return PageAction::None;
                        }
                    };

                    let authorized = game
                        .runtime
                        .get_authorized_positions(game.runtime.turn(), &actual_selected);
                    if authorized.contains(&castling_dest) {
                        let castling_coords = model::Coord::from(flip_square_if_needed(
                            castling_dest,
                            game.runtime.is_flipped(),
                        ));
                        let target = flip_square_if_needed(
                            castling_dest,
                            game.runtime.is_flipped(),
                        );
                        let action = Self::try_mouse_move(
                            &mut game,
                            target,
                            castling_coords,
                            ctx,
                        );
                        if !matches!(action, PageAction::None) {
                            return action;
                        }
                    }
                }

                let action = Self::try_mouse_move(&mut game, square, coords, ctx);
                if !matches!(action, PageAction::None) {
                    return action;
                }
            }
            game.ui.selected_square = Some(square);
            return PageAction::None;
        }

        if game.ui.selected_square.is_some() {
            return Self::try_mouse_move(&mut game, square, coords, ctx);
        }

        PageAction::None
    }
}

impl Page for GamePage {
    fn handle_key(&mut self, key: KeyEvent, ctx: &PageContext<'_>) -> PageAction {
        if self.end_screen_visible {
            return self.handle_end_screen_key(key);
        }
        match self.kind {
            GamePageKind::Solo => self.handle_solo_key(key, ctx),
            GamePageKind::Bot => self.handle_bot_key(key, ctx),
            GamePageKind::Multiplayer | GamePageKind::Lichess => {
                self.handle_multiplayer_key(key, ctx)
            }
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, ctx: &PageContext<'_>) -> PageAction {
        self.handle_game_mouse(mouse, ctx)
    }

    fn tick(&mut self, _ctx: &PageContext<'_>) -> PageAction {
        {
            let mut game = self.game.borrow_mut();
            game.runtime.tick();
            if game.runtime.state().is_finished() && !self.end_screen_dismissed {
                self.end_screen_visible = true;
            }
        }
        PageAction::None
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, ctx: &mut PageContext<'_>) {
        let mut game = self.game.borrow_mut();
        render_game_ui(
            frame,
            area,
            &mut game,
            ctx.kitty.take(),
            ctx.kitty_enabled,
            ctx.game.is_puzzle,
            self.end_screen_visible,
        );
    }

    fn submit_move(&mut self, chess_move: &shakmaty::Move, ctx: &PageContext<'_>) -> PageAction {
        let Some(from) = chess_move.from() else {
            return PageAction::None;
        };
        let mut game = self.game.borrow_mut();
        if !game.apply_player_move(from, chess_move.to(), chess_move.promotion()) {
            return PageAction::None;
        }
        if ctx.game.is_puzzle {
            game.runtime.set_flipped(false);
            let move_uci = chess_move
                .to_uci(shakmaty::CastlingMode::Standard)
                .to_string();
            return PageAction::Game(GameAction::ValidateMoveUci { move_uci });
        }
        if game.runtime.state().is_finished() && !self.end_screen_dismissed {
            self.end_screen_visible = true;
        }
        PageAction::Game(GameAction::CheckEnd)
    }

    fn game_kind(&self) -> Option<GamePageKind> {
        Some(self.kind)
    }
}
