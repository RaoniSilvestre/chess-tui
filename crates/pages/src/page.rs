//! Page API and navigation requests.
//!
//! Pages own their own state and mutate [`Globals`] directly. They only return
//! [`PageAction`] for application-level transitions (navigate/quit).

use ratatui::Frame;
use ratatui::crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::Rect;

use crate::game_mode::MatchConfig;
use crate::game_page::BotGameConfig;
use crate::globals::Globals;
use crate::lichess_pages::LichessGameSpec;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PageKind {
    Home,
    GameModeMenu,
    Settings,
    Credits,
    LichessMenu,
    OngoingGames,
    LocalGame,
    BotGame,
    MultiplayerGame,
    LichessGame,
    PuzzleGame,
    PgnViewer,
}

/// What a page asks the application shell to do after an input/lifecycle event.
pub enum PageAction {
    None,
    /// The page consumed the key but has no navigation action.
    Handled,
    Quit,
    Navigate(PageRequest),
}

/// A request to replace the active page. The request carries everything the
/// new page needs; the page itself owns that state afterwards.
pub enum PageRequest {
    Home,
    GameModeMenu,
    Settings,
    Credits,
    LichessMenu,
    OngoingGames,
    LocalGame {
        config: MatchConfig,
        resume: bool,
    },
    BotGame {
        config: MatchConfig,
        bot: BotGameConfig,
        resume: bool,
    },
    MultiplayerGame {
        config: MatchConfig,
    },
    LichessGame {
        spec: LichessGameSpec,
    },
    PuzzleGame {
        puzzle: lichess::Puzzle,
    },
    PgnViewer {
        games: Vec<model::PgnViewer>,
        index: usize,
    },
}

/// A full-screen page.
pub trait Page {
    fn kind(&self) -> PageKind;

    /// Called once after the page becomes active.
    fn on_enter(&mut self, _globals: &mut Globals) -> PageAction {
        PageAction::None
    }

    /// Called before the page is replaced.
    fn on_exit(&mut self, _globals: &mut Globals) {}

    fn handle_key(&mut self, key: KeyEvent, globals: &mut Globals) -> PageAction;

    fn handle_mouse(&mut self, _mouse: MouseEvent, _globals: &mut Globals) -> PageAction {
        PageAction::None
    }

    fn tick(&mut self, _globals: &mut Globals) -> PageAction {
        PageAction::None
    }

    /// Called once per event-loop iteration, after input handling.
    fn update(&mut self, _globals: &mut Globals) -> PageAction {
        PageAction::None
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, globals: &mut Globals);

    /// Context for the global help overlay: (history navigation available, puzzle mode).
    fn help_context(&self) -> (bool, bool) {
        (false, false)
    }

    /// True while a page-local prompt/modal should receive keys before global fallbacks.
    fn is_modal_active(&self) -> bool {
        false
    }
}
