//! Mode abstraction: each game kind reacts to moves and ticks.

use crate::GameSession;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameKind {
    Local,
    Bot,
    Tcp,
    Lichess,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndScreenKind {
    Standard,
    Lichess,
    Puzzle,
}

/// Events emitted by a mode for the application layer to translate into
/// sound, popups, or navigation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameEvent {
    MovePlayed,
    Error(String),
    OpponentJoined,
}

/// Behaviour specific to one kind of game.
///
/// The runtime owns the session; modes only react to it.
pub trait GameMode {
    fn kind(&self) -> GameKind;

    fn is_ready(&self) -> bool {
        true
    }

    fn can_player_move(&self, session: &GameSession) -> bool {
        !session.state().is_finished() && !session.state().is_promotion()
    }

    fn on_player_move(&mut self, session: &mut GameSession, events: &mut Vec<GameEvent>) {
        let _ = (session, events);
    }

    fn on_promotion(&mut self, session: &mut GameSession, events: &mut Vec<GameEvent>) {
        let _ = (session, events);
    }

    fn tick(&mut self, session: &mut GameSession, events: &mut Vec<GameEvent>) {
        let _ = (session, events);
    }

    fn flip_after_move(&self, session: &GameSession) -> bool {
        let _ = session;
        false
    }

    fn history_is_solo(&self) -> bool {
        false
    }

    fn shows_last_move_from(&self, _color: shakmaty::Color) -> bool {
        true
    }

    fn end_screen(&self) -> EndScreenKind {
        EndScreenKind::Standard
    }

    fn disconnect(&mut self) {}
}
