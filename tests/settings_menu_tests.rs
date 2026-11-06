use chess_tui::{
    handlers::handler::handle_key_events,
    runtime::{App, AppResult},
};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: ratatui::crossterm::event::KeyEventState::empty(),
    }
}

fn send(app: &mut App, keys: &[KeyCode]) -> AppResult<()> {
    for &k in keys {
        handle_key_events(key(k), app)?;
    }
    Ok(())
}

fn open_settings_menu() -> AppResult<App> {
    let mut app = chess_tui::runtime::test_app();
    send(&mut app, &[KeyCode::Down, KeyCode::Down, KeyCode::Enter])?;
    Ok(app)
}

#[test]
fn open_close_settings_menu() -> AppResult<()> {
    let mut app = open_settings_menu()?;
    assert!(app.is_page::<pages::SettingsPage>());

    send(&mut app, &[KeyCode::Esc])?;
    assert!(app.is_home());
    Ok(())
}

#[test]
#[cfg(feature = "sound")]
fn toggle_sound() -> AppResult<()> {
    let mut app = open_settings_menu()?;
    assert!(app.is_page::<pages::SettingsPage>());

    let original_sound_status = app.sound_enabled;
    send(&mut app, &[KeyCode::Down, KeyCode::Enter])?;
    let new_sound_status = app.sound_enabled;
    assert_ne!(new_sound_status, original_sound_status);

    send(&mut app, &[KeyCode::Enter])?;
    assert_eq!(original_sound_status, app.sound_enabled);

    Ok(())
}

#[test]
fn toggle_animations() -> AppResult<()> {
    let mut app = open_settings_menu()?;
    assert!(app.is_page::<pages::SettingsPage>());

    let original_anim_status = app.animations_enabled;
    send(
        &mut app,
        &[
            KeyCode::Down,
            #[cfg(feature = "sound")]
            KeyCode::Down,
            KeyCode::Enter,
        ],
    )?;
    let new_anim_status = app.animations_enabled;
    assert_ne!(new_anim_status, original_anim_status);

    send(&mut app, &[KeyCode::Enter])?;
    assert_eq!(original_anim_status, app.animations_enabled);

    Ok(())
}

#[test]
fn open_close_chess_engine_path_popup() -> AppResult<()> {
    let mut app = open_settings_menu()?;
    assert!(app.is_page::<pages::SettingsPage>());

    send(
        &mut app,
        &[
            KeyCode::Down,
            KeyCode::Down,
            #[cfg(feature = "sound")]
            KeyCode::Down,
            KeyCode::Enter,
        ],
    )?;
    assert!(app.has_modal::<app::modal::PromptModal>());

    send(&mut app, &[KeyCode::Esc])?;
    assert!(!app.has_any_modal());
    assert!(app.is_page::<pages::SettingsPage>());

    Ok(())
}

#[test]
fn change_bot_difficulty() -> AppResult<()> {
    let mut app = open_settings_menu()?;
    assert!(app.is_page::<pages::SettingsPage>());

    let original_difficulty = app.bot_state.bot_difficulty;
    send(
        &mut app,
        &[
            KeyCode::Down,
            KeyCode::Down,
            KeyCode::Down,
            #[cfg(feature = "sound")]
            KeyCode::Down,
            KeyCode::Down,
            KeyCode::Right,
        ],
    )?;
    let next_difficulty = app.bot_state.bot_difficulty;
    assert_ne!(next_difficulty, original_difficulty);

    send(&mut app, &[KeyCode::Left])?;
    assert_eq!(app.bot_state.bot_difficulty, original_difficulty);

    send(&mut app, &[KeyCode::Left])?;
    let prev_difficulty = app.bot_state.bot_difficulty;
    assert_ne!(prev_difficulty, original_difficulty);
    assert_ne!(prev_difficulty, next_difficulty);

    send(&mut app, &[KeyCode::Right])?;
    assert_eq!(app.bot_state.bot_difficulty, original_difficulty);

    Ok(())
}

#[test]
fn kitty_graphics_hidden_outside_kitty() -> AppResult<()> {
    // chess_tui::runtime::test_app() has no Kitty pieces loaded, as in a non-Kitty terminal.
    let mut app = open_settings_menu()?;

    // Up from the first item wraps to the last visible one, which must not be Kitty Graphics.
    send(&mut app, &[KeyCode::Up, KeyCode::Enter])?;
    assert!(app.kitty_graphics_enabled);

    Ok(())
}
