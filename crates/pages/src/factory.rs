//! Default page factory used by the application bootstrap.

use page::{Page, PageFactory};
use session::GameHandle;
use model::PgnViewer;

use crate::{
    BotGamePage, CreditsPage, GameModeMenuPage, HomePage, LichessGamePage, LichessMenuPage,
    LocalGamePage, MultiplayerGamePage, OngoingGamesPage, PgnViewerPage, SettingsPage,
};

/// Builds the concrete pages shipped with chess-tui.
pub struct DefaultPageFactory;

impl PageFactory for DefaultPageFactory {
    fn home(&self) -> Box<dyn Page> {
        Box::new(HomePage::new())
    }

    fn solo(&self, game: GameHandle) -> Box<dyn Page> {
        Box::new(LocalGamePage::new(game))
    }

    fn bot(&self, game: GameHandle) -> Box<dyn Page> {
        Box::new(BotGamePage::new(game))
    }

    fn multiplayer(&self, game: GameHandle) -> Box<dyn Page> {
        Box::new(MultiplayerGamePage::new(game))
    }

    fn lichess(&self, game: GameHandle) -> Box<dyn Page> {
        Box::new(LichessGamePage::new(game))
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
