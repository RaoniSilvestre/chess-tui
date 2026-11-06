//! Local two-player mode.

use crate::{EndScreenKind, GameEvent, GameKind, GameMode, GameSession};

#[derive(Default)]
pub struct LocalMode;

impl GameMode for LocalMode {
    fn kind(&self) -> GameKind {
        GameKind::Local
    }

    fn flip_after_move(&self, _session: &GameSession) -> bool {
        true
    }

    fn history_is_solo(&self) -> bool {
        true
    }

    fn end_screen(&self) -> EndScreenKind {
        EndScreenKind::Standard
    }

    fn on_player_move(&mut self, _session: &mut GameSession, _events: &mut Vec<GameEvent>) {}
}
