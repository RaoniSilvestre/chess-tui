//! Live Lichess game page. Owns the game session and event stream.

use game::LichessMode;
use lichess::LichessClient;
use model::{Clock, GameState, TimeControl};
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{Frame, layout::Rect};
use shakmaty::{CastlingMode, Chess, Color, fen::Fen};

use crate::globals::Globals;
use crate::page::{Page, PageAction, PageKind, PageRequest};

use super::LichessGameSpec;
use crate::game_page::board::{BoardAction, GameBoard};

pub struct LichessGamePage {
    board: GameBoard,
    client: LichessClient,
    spec: LichessGameSpec,
}

impl LichessGamePage {
    pub fn new(spec: LichessGameSpec, globals: &mut Globals) -> Self {
        let client = super::client_from_globals(globals)
            .unwrap_or_else(|| LichessClient::new(String::new()));
        Self {
            board: GameBoard::new(),
            client,
            spec,
        }
    }

    pub fn game(&self) -> &session::Game {
        self.board.game()
    }

    pub fn game_mut(&mut self) -> &mut session::Game {
        self.board.game_mut()
    }

    fn start_game(&mut self, globals: &mut Globals) -> Result<(), String> {
        let mut fen = self.spec.fen.clone();
        if fen.is_none()
            && let Ok(games) = self.client.get_ongoing_games()
        {
            fen = games
                .iter()
                .find(|g| g.game_id == self.spec.game_id)
                .map(|g| g.fen.clone());
        }

        let mut initial_move_count = 0;
        let mut immediate_last_move = None;

        if let Some(ref fen_str) = fen {
            if let Ok((turns, last_move)) = self
                .client
                .get_game_turn_count_and_last_move(&self.spec.game_id)
            {
                initial_move_count = turns;
                immediate_last_move = last_move;
            }

            let position = Fen::from_ascii(fen_str.as_bytes())
                .ok()
                .and_then(|f| f.into_position::<Chess>(CastlingMode::Standard).ok())
                .ok_or_else(|| format!("Failed to parse FEN: {fen_str}"))?;
            let halfmove_clock = self.board.game.runtime.halfmove_clock();
            self.board
                .game
                .runtime
                .load_history(vec![position], vec![], vec![], halfmove_clock)
                .map_err(|e| format!("Failed to load position: {e}"))?;
            self.board.game.runtime.sync_turn_with_position();
        }

        self.board.game.runtime.start_playing();
        self.board
            .game
            .runtime
            .set_clock(Clock::new(TimeControl::Rapid));

        let mode = LichessMode::new(
            self.client.clone(),
            self.spec.game_id.clone(),
            self.spec.color,
            initial_move_count,
            immediate_last_move,
        )
        .map_err(|e| format!("Failed to stream game: {e}"))?;
        self.board.game.runtime.set_mode(Box::new(mode));

        if self.spec.color == Color::Black {
            self.board.game.runtime.flip();
        }
        globals.theme.apply_to_game(&mut self.board.game);
        Ok(())
    }

    fn handle_board_action(&mut self, action: BoardAction) -> PageAction {
        match action {
            BoardAction::None => PageAction::None,
            BoardAction::Page(action) => action,
            BoardAction::Restart | BoardAction::OpenMoveInput | BoardAction::PuzzleMove { .. } => {
                PageAction::None
            }
        }
    }
}

impl Page for LichessGamePage {
    fn kind(&self) -> PageKind {
        PageKind::LichessGame
    }

    fn on_enter(&mut self, globals: &mut Globals) -> PageAction {
        if let Err(e) = self.start_game(globals) {
            log::error!("{e}");
            globals.show_error_then(e, PageRequest::LichessMenu);
        }
        PageAction::None
    }

    fn on_exit(&mut self, _globals: &mut Globals) {
        self.board.game.runtime.disconnect();
    }

    fn handle_key(&mut self, key: KeyEvent, globals: &mut Globals) -> PageAction {
        if self.board.end_screen_visible {
            let action = self.board.handle_end_screen_key(key);
            return self.handle_board_action(action);
        }
        if key.code == KeyCode::Char('b') {
            return PageAction::Navigate(PageRequest::Home);
        }
        let action = self.board.handle_common_key(key, globals);
        self.handle_board_action(action)
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, globals: &mut Globals) -> PageAction {
        let action = self.board.handle_mouse(mouse);
        let _ = globals;
        self.handle_board_action(action)
    }

    fn tick(&mut self, _globals: &mut Globals) -> PageAction {
        self.board.tick();
        PageAction::None
    }

    fn update(&mut self, _globals: &mut Globals) -> PageAction {
        if self.board.game.runtime.state() == GameState::Playing {
            self.board.update();
        }
        PageAction::None
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, globals: &mut Globals) {
        self.board.render(frame, area, globals);
    }
}
