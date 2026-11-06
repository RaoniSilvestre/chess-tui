//! Shared application services.

use crate::animations::AnimationState;
use crate::game_mode::GameModeState;
use crate::state::bot_state::BotState;
use crate::state::lichess_state::LichessState;
use crate::state::multiplayer_state::MultiplayerState;
use crate::state::theme_state::ThemeState;
use crate::Modal;
use log::LevelFilter;
use session::{Game, GameHandle, game_handle};
use std::cell::{Ref, RefMut};
use std::time::Instant;
use ui::graphics::KittyPieces;

/// Shared state and services used by pages and the app dispatcher.
pub struct Services {
    pub kitty_pieces: Option<KittyPieces>,
    pub game: GameHandle,
    pub log_level: LevelFilter,
    pub sound_enabled: bool,
    pub animations_enabled: bool,
    pub kitty_graphics_enabled: bool,
    pub theme_state: ThemeState,
    pub bot_state: BotState,
    pub multiplayer_state: MultiplayerState,
    pub game_mode_state: GameModeState,
    pub lichess_state: LichessState,
    pub animations: AnimationState,
    pub last_saved_move_count: Option<usize>,
    pub last_resume_flush_at: Option<Instant>,
    pub modal: Option<Box<dyn Modal>>,
}

impl Services {
    pub fn game(&self) -> Ref<'_, Game> {
        self.game.borrow()
    }

    pub fn game_mut(&self) -> RefMut<'_, Game> {
        self.game.borrow_mut()
    }

    pub fn replace_game(&mut self, game: Game) {
        *self.game.borrow_mut() = game;
    }
}

impl Default for Services {
    fn default() -> Self {
        Self {
            kitty_pieces: None,
            game: game_handle(Game::default()),
            log_level: LevelFilter::Off,
            sound_enabled: true,
            animations_enabled: false,
            kitty_graphics_enabled: true,
            theme_state: ThemeState::default(),
            bot_state: BotState::default(),
            multiplayer_state: MultiplayerState::default(),
            game_mode_state: GameModeState::default(),
            lichess_state: LichessState::default(),
            animations: AnimationState::default(),
            last_saved_move_count: None,
            last_resume_flush_at: None,
            modal: None,
        }
    }
}
