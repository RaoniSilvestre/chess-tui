//! Shared game-mode helpers used by the page router and popup handlers.

use crate::{
    App, GamePageKind,
    constants::{BOT_DIFFICULTY_COUNT, Popups},
    state::{resume::ResumeMode, ui_state::NewGame},
};

pub use crate::game_mode::AvailableGameMode;

pub fn cycle_difficulty_prev(app: &mut App) {
    match app.bot_state.bot_difficulty {
        None => app.bot_state.bot_difficulty = Some((BOT_DIFFICULTY_COUNT - 1) as u8),
        Some(0) => app.bot_state.bot_difficulty = None,
        Some(i) => app.bot_state.bot_difficulty = Some(i - 1),
    }
    app.update_config_from_app();
}

pub fn cycle_difficulty_next(app: &mut App) {
    match app.bot_state.bot_difficulty {
        None => app.bot_state.bot_difficulty = Some(0),
        Some(i) if i + 1 >= BOT_DIFFICULTY_COUNT as u8 => app.bot_state.bot_difficulty = None,
        Some(i) => app.bot_state.bot_difficulty = Some(i + 1),
    }
    app.update_config_from_app();
}

pub(crate) fn finish_form(app: &mut App, page: GamePageKind) {
    // A saved game in this mode would be lost: ask first.
    let mode = match page {
        GamePageKind::Solo => Some(ResumeMode::Local),
        GamePageKind::Bot => Some(ResumeMode::Bot),
        _ => None,
    };
    if let Some(mode) = mode
        && app.has_resume_save(mode)
    {
        app.ui_state.pending_new_game = Some(NewGame::Start(page));
        app.ui_state.current_popup = Some(Popups::NewGameConfirmation);
        return;
    }
    start_game(app, page);
}

/// Leaves the form for `page`'s game. Starting a brand-new resumable game invalidates any
/// previous save.
pub fn start_game(app: &mut App, page: GamePageKind) {
    match page {
        GamePageKind::Solo => app.clear_resume_state(ResumeMode::Local),
        GamePageKind::Bot => app.clear_resume_state(ResumeMode::Bot),
        _ => {}
    }

    match page {
        GamePageKind::Solo => app.show_solo(),
        GamePageKind::Bot => app.show_bot(),
        GamePageKind::Multiplayer => app.show_multiplayer(),
        _ => {}
    }
    app.game_mode_state.selection = None;
    app.game_mode_state.form_cursor = 0;
    app.game_mode_state.form_active = false;
}
