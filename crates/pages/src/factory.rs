//! Page construction from navigation requests.

use crate::credits::CreditsPage;
use crate::game_mode::GameModeMenuPage;
use crate::game_page::{BotGamePage, LocalGamePage, MultiplayerGamePage};
use crate::globals::Globals;
use crate::home::HomePage;
use crate::lichess_pages::{LichessGamePage, LichessMenuPage, OngoingGamesPage, PuzzleGamePage};
use crate::page::{Page, PageRequest};
use crate::pgn::PgnViewerPage;
use crate::settings::SettingsPage;

pub fn create_page(request: PageRequest, globals: &mut Globals) -> Box<dyn Page> {
    match request {
        PageRequest::Home => Box::new(HomePage::new()),
        PageRequest::GameModeMenu => Box::new(GameModeMenuPage::new()),
        PageRequest::Settings => Box::new(SettingsPage::new()),
        PageRequest::Credits => Box::new(CreditsPage::new()),
        PageRequest::LichessMenu => Box::new(LichessMenuPage::new(globals)),
        PageRequest::OngoingGames => Box::new(OngoingGamesPage::new(globals)),
        PageRequest::LocalGame { config, resume } => {
            Box::new(LocalGamePage::new(config, resume, globals))
        }
        PageRequest::BotGame {
            config,
            bot,
            resume,
        } => Box::new(BotGamePage::new(config, bot, resume, globals)),
        PageRequest::MultiplayerGame { config } => {
            Box::new(MultiplayerGamePage::new(config, globals))
        }
        PageRequest::LichessGame { spec } => Box::new(LichessGamePage::new(spec, globals)),
        PageRequest::PuzzleGame { puzzle } => Box::new(PuzzleGamePage::new(puzzle, globals)),
        PageRequest::PgnViewer { games, index } => Box::new(PgnViewerPage::new(games, index)),
    }
}
