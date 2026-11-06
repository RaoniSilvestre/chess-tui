//! Application state, page framework, and shared services.

pub mod animations;
pub mod app;
pub mod config;
pub mod constants;
pub mod game;
pub mod game_mode;
pub mod game_mode_menu;
pub mod modal;
pub mod puzzle;
pub mod state;

pub use app::{App, AppResult};
pub use game_mode::{AvailableGameMode, GameModeState};

use ratatui::Frame;
use ratatui::crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use std::any::Any;

/// Navigation result returned by a page.
pub enum Nav {
    None,
    Replace(Box<dyn Page>),
    Quit,
}

/// Object-safe downcasting helper for pages.
pub trait AsAny {
    fn as_any(&self) -> &dyn Any;
}

impl<T: 'static> AsAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A full-screen page. Pages own their local state and receive the concrete
/// application directly.
pub trait Page: AsAny {
    fn handle_key(&mut self, key: KeyEvent, app: &mut App) -> Nav;

    fn handle_mouse(&mut self, _mouse: MouseEvent, _app: &mut App) -> Nav {
        Nav::None
    }

    fn tick(&mut self, _app: &mut App) -> Nav {
        Nav::None
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, app: &mut App);

    /// Present when this page is one of the game pages.
    fn game_kind(&self) -> Option<GamePageKind> {
        None
    }

    /// Used by services that must not drive the PGN replay board.
    fn is_pgn_viewer(&self) -> bool {
        false
    }

    fn is_home(&self) -> bool {
        false
    }

    fn is_ongoing_games(&self) -> bool {
        false
    }
}

/// Result of a modal key event.
pub enum ModalResult {
    None,
    Close,
    Replace(Box<dyn Modal>),
}

/// A modal overlay. Modals own their local state and receive the application.
pub trait Modal: AsAny {
    fn handle_key(&mut self, key: KeyEvent, app: &mut App) -> ModalResult;
    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, app: &App);
}

/// Owns the active page and applies navigation results.
pub struct Router {
    page: Box<dyn Page>,
}

impl Router {
    pub fn new(page: Box<dyn Page>) -> Self {
        Self { page }
    }

    pub fn page(&self) -> &dyn Page {
        self.page.as_ref()
    }

    pub fn page_mut(&mut self) -> &mut dyn Page {
        self.page.as_mut()
    }

    pub fn handle_key(&mut self, key: KeyEvent, app: &mut App) {
        let nav = self.page.handle_key(key, app);
        self.apply(nav, app);
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent, app: &mut App) {
        let nav = self.page.handle_mouse(mouse, app);
        self.apply(nav, app);
    }

    pub fn tick(&mut self, app: &mut App) {
        let nav = self.page.tick(app);
        self.apply(nav, app);
    }

    pub fn render(&mut self, frame: &mut Frame<'_>, area: Rect, app: &mut App) {
        self.page.render(frame, area, app);
    }

    fn apply(&mut self, nav: Nav, app: &mut App) {
        match nav {
            Nav::None => {
                if let Some(page) = app.take_pending_page() {
                    self.page = page;
                }
            }
            Nav::Replace(page) => self.page = page,
            Nav::Quit => app.quit(),
        }
    }
}

/// Constructs page instances. Implemented by `chess-tui-pages`; the app crate
/// depends only on this interface, which keeps the dependency one-way.
pub trait PageFactory {
    fn home(&self) -> Box<dyn Page>;
    fn solo(&self) -> Box<dyn Page>;
    fn bot(&self) -> Box<dyn Page>;
    fn multiplayer(&self) -> Box<dyn Page>;
    fn lichess(&self) -> Box<dyn Page>;
    fn lichess_menu(&self, time_control: u8) -> Box<dyn Page>;
    fn ongoing_games(&self) -> Box<dyn Page>;
    fn game_mode_menu(&self) -> Box<dyn Page>;
    fn settings(&self) -> Box<dyn Page>;
    fn credits(&self) -> Box<dyn Page>;
    fn pgn_viewer(&self, games: Vec<chess_tui_core::PgnViewer>, game_idx: usize) -> Box<dyn Page>;
}

/// Placeholder used by `App::default()` before the real factory is installed.
pub struct NoopPageFactory;

impl PageFactory for NoopPageFactory {
    fn home(&self) -> Box<dyn Page> {
        Box::new(NoopPage)
    }
    fn solo(&self) -> Box<dyn Page> {
        Box::new(NoopPage)
    }
    fn bot(&self) -> Box<dyn Page> {
        Box::new(NoopPage)
    }
    fn multiplayer(&self) -> Box<dyn Page> {
        Box::new(NoopPage)
    }
    fn lichess(&self) -> Box<dyn Page> {
        Box::new(NoopPage)
    }
    fn lichess_menu(&self, _time_control: u8) -> Box<dyn Page> {
        Box::new(NoopPage)
    }
    fn ongoing_games(&self) -> Box<dyn Page> {
        Box::new(NoopPage)
    }
    fn game_mode_menu(&self) -> Box<dyn Page> {
        Box::new(NoopPage)
    }
    fn settings(&self) -> Box<dyn Page> {
        Box::new(NoopPage)
    }
    fn credits(&self) -> Box<dyn Page> {
        Box::new(NoopPage)
    }
    fn pgn_viewer(
        &self,
        _games: Vec<chess_tui_core::PgnViewer>,
        _game_idx: usize,
    ) -> Box<dyn Page> {
        Box::new(NoopPage)
    }
}

struct NoopPage;

impl Page for NoopPage {
    fn handle_key(&mut self, _key: KeyEvent, _app: &mut App) -> Nav {
        Nav::None
    }

    fn render(&mut self, _frame: &mut Frame<'_>, _area: Rect, _app: &mut App) {}
}

/// The concrete game page kind.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GamePageKind {
    Solo,
    Bot,
    Multiplayer,
    Lichess,
}
