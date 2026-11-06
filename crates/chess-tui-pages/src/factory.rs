//! Default page factory used by the application bootstrap.

use chess_tui_app::{GamePageKind, Page, PageFactory};
use chess_tui_core::PgnViewer;

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

    fn solo(&self) -> Box<dyn Page> {
        Box::new(GamePage::new(GamePageKind::Solo))
    }

    fn bot(&self) -> Box<dyn Page> {
        Box::new(GamePage::new(GamePageKind::Bot))
    }

    fn multiplayer(&self) -> Box<dyn Page> {
        Box::new(GamePage::new(GamePageKind::Multiplayer))
    }

    fn lichess(&self) -> Box<dyn Page> {
        Box::new(GamePage::new(GamePageKind::Lichess))
    }

    fn lichess_menu(&self, time_control: u8) -> Box<dyn Page> {
        Box::new(LichessMenuPage::new(time_control))
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
