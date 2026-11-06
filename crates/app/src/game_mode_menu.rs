//! Shared game-mode helpers used by the page router and popup handlers.

use crate::{
    App, GameModeKind,
    constants::BOT_DIFFICULTY_COUNT,
    state::resume::ResumeMode,
};

pub use crate::game_mode::AvailableGameMode;

/// A new game that would discard a resume save, held until the user confirms it.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum NewGame {
    Start(GameModeKind),
    Restart,
}

pub fn cycle_difficulty_prev(app: &mut App) {
    match app.services.bot_state.bot_difficulty {
        None => app.services.bot_state.bot_difficulty = Some((BOT_DIFFICULTY_COUNT - 1) as u8),
        Some(0) => app.services.bot_state.bot_difficulty = None,
        Some(i) => app.services.bot_state.bot_difficulty = Some(i - 1),
    }
    app.update_config_from_app();
}

pub fn cycle_difficulty_next(app: &mut App) {
    match app.services.bot_state.bot_difficulty {
        None => app.services.bot_state.bot_difficulty = Some(0),
        Some(i) if i + 1 >= BOT_DIFFICULTY_COUNT as u8 => app.services.bot_state.bot_difficulty = None,
        Some(i) => app.services.bot_state.bot_difficulty = Some(i + 1),
    }
    app.update_config_from_app();
}

pub(crate) fn finish_form(app: &mut App, page: GameModeKind) {
    // A saved game in this mode would be lost: ask first.
    let mode = match page {
        GameModeKind::Local => Some(ResumeMode::Local),
        GameModeKind::Bot => Some(ResumeMode::Bot),
        _ => None,
    };
    if let Some(mode) = mode
        && app.has_resume_save(mode)
    {
        app.show_new_game_confirmation(NewGame::Start(page));
        return;
    }
    start_game(app, page);
}

/// Leaves the form for `page`'s game. Starting a brand-new resumable game invalidates any
/// previous save.
pub fn start_game(app: &mut App, page: GameModeKind) {
    match page {
        GameModeKind::Local => app.clear_resume_state(ResumeMode::Local),
        GameModeKind::Bot => app.clear_resume_state(ResumeMode::Bot),
        _ => {}
    }

    match page {
        GameModeKind::Local => app.show_solo(),
        GameModeKind::Bot => app.show_bot(),
        GameModeKind::Multiplayer => app.show_multiplayer(),
        _ => {}
    }
}
