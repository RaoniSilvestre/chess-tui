//! Runtime that combines a session with a mode.

use crate::{GameEvent, GameKind, GameMode, GameSession};
use std::ops::{Deref, DerefMut};

pub struct GameRuntime {
    session: GameSession,
    mode: Box<dyn GameMode>,
    events: Vec<GameEvent>,
}

impl GameRuntime {
    pub fn new(session: GameSession, mode: Box<dyn GameMode>) -> Self {
        Self {
            session,
            mode,
            events: Vec::new(),
        }
    }

    pub fn session(&self) -> &GameSession {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut GameSession {
        &mut self.session
    }

    pub fn mode(&self) -> &dyn GameMode {
        self.mode.as_ref()
    }

    pub fn mode_mut(&mut self) -> &mut dyn GameMode {
        self.mode.as_mut()
    }

    pub fn set_mode(&mut self, mode: Box<dyn GameMode>) {
        self.mode = mode;
    }

    pub fn kind(&self) -> GameKind {
        self.mode.kind()
    }

    pub fn can_player_move(&self) -> bool {
        self.mode.can_player_move(&self.session)
    }

    pub fn flip_after_move(&self) -> bool {
        self.mode.flip_after_move(&self.session)
    }

    pub fn history_is_solo(&self) -> bool {
        self.mode.history_is_solo()
    }

    pub fn take_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn tick(&mut self) {
        let mut events = Vec::new();
        self.mode.tick(&mut self.session, &mut events);
        self.events.extend(events);
    }

    pub fn on_player_move(&mut self) {
        let mut events = Vec::new();
        self.mode.on_player_move(&mut self.session, &mut events);
        self.events.extend(events);
    }

    pub fn on_promotion(&mut self) {
        let mut events = Vec::new();
        self.mode.on_promotion(&mut self.session, &mut events);
        self.events.extend(events);
    }
}

impl Deref for GameRuntime {
    type Target = GameSession;

    fn deref(&self) -> &Self::Target {
        &self.session
    }
}

impl DerefMut for GameRuntime {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.session
    }
}
