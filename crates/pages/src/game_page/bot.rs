//! Bot game page.

use page::{GameModeKind, Page, PageAction, PageContext};
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{Frame, layout::Rect};
use session::GameHandle;

use super::board::GameBoard;

pub struct BotGamePage {
    board: GameBoard,
}

impl BotGamePage {
    pub fn new(game: GameHandle) -> Self {
        Self {
            board: GameBoard::new(game),
        }
    }
}

impl Page for BotGamePage {
    fn handle_key(&mut self, key: KeyEvent, ctx: &PageContext<'_>) -> PageAction {
        if self.board.end_screen_visible {
            return self.board.handle_end_screen_key(key);
        }
        match key.code {
            KeyCode::Char('r') => PageAction::Game(page::GameAction::RequestRestart),
            KeyCode::Char('b') => PageAction::Game(page::GameAction::GoHome),
            _ => self.board.handle_common_key(key, ctx),
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, ctx: &PageContext<'_>) -> PageAction {
        self.board.handle_mouse(mouse, ctx)
    }

    fn tick(&mut self, _ctx: &PageContext<'_>) -> PageAction {
        self.board.tick();
        PageAction::None
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, ctx: &mut PageContext<'_>) {
        self.board.render(frame, area, ctx);
    }

    fn submit_move(
        &mut self,
        chess_move: &shakmaty::Move,
        ctx: &PageContext<'_>,
    ) -> PageAction {
        self.board.submit_move(chess_move, ctx)
    }

    fn game_mode(&self) -> Option<GameModeKind> {
        Some(GameModeKind::Bot)
    }
}
