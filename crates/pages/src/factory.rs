//! Default page factory used by the application bootstrap.

use page::{GamePageKind, Page, PageFactory};
use session::GameHandle;
use model::PgnViewer;

use crate::{
    CreditsPage, GameModeMenuPage, GamePage, HomePage, LichessMenuPage, OngoingGamesPage,
    PgnViewerPage, SettingsPage,
};

/// Builds the concrete pages shipped with chess-tui.
pub struct DefaultPageFactory;

impl PageFactory for DefaultPageFactory {
    fn home(&self) -> Box<dyn Page> {
        Box::new(HomePage::new())
    }

    fn solo(&self, game: GameHandle) -> Box<dyn Page> {
        Box::new(GamePage::new(GamePageKind::Solo, game))
    }

    fn bot(&self, game: GameHandle) -> Box<dyn Page> {
        Box::new(GamePage::new(GamePageKind::Bot, game))
    }

    fn multiplayer(&self, game: GameHandle) -> Box<dyn Page> {
        Box::new(GamePage::new(GamePageKind::Multiplayer, game))
    }

    fn lichess(&self, game: GameHandle) -> Box<dyn Page> {
        Box::new(GamePage::new(GamePageKind::Lichess, game))
    }

    fn lichess_menu(&self) -> Box<dyn Page> {
        Box::new(LichessMenuPage::new())
    }

    fn ongoing_games(&self) -> Box<dyn Page> {
        Box::new(OngoingGamesPage::new())
    }

    fn game_mode_menu(&self) -> Box<dyn Page> {
        Box::new(GameModeMenuPage::new())
    }

    fn settings(&self) -> Box<dyn Page> {
        Box::new(SettingsPage::new())
    }

    fn credits(&self) -> Box<dyn Page> {
        Box::new(CreditsPage::new())
    }

    fn pgn_viewer(&self, games: Vec<PgnViewer>, game_idx: usize) -> Box<dyn Page> {
        Box::new(PgnViewerPage::new(games, game_idx))
    }
}
