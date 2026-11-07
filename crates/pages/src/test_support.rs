//! Test-only helpers. Nothing here is compiled into production builds.

use std::sync::{Mutex, MutexGuard};

use config::ConfigStore;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::globals::{Globals, Theme};
use crate::page::Page;

static RESUME_ENV_LOCK: Mutex<()> = Mutex::new(());

/// A `Globals` backed by an isolated in-memory config store.
pub(crate) fn test_globals() -> Globals {
    ui::sound::set_sound_enabled(false);
    Globals {
        config: ConfigStore::disabled(),
        theme: Theme {
            loaded_skin: Some(theme::Skin::default()),
            available_skins: vec![theme::Skin::default()],
            available_piece_styles: Vec::new(),
            selected_skin_name: "Default".to_string(),
            display_mode: theme::DisplayMode::DEFAULT,
        },
        sound_enabled: false,
        animations_enabled: false,
        kitty_graphics_enabled: true,
        kitty_pieces: None,
        overlay: None,
    }
}

pub(crate) fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: ratatui::crossterm::event::KeyEventState::empty(),
    }
}

pub(crate) fn send<P: Page>(page: &mut P, globals: &mut Globals, keys: &[KeyCode]) {
    for &code in keys {
        let _ = page.handle_key(key(code), globals);
    }
}

pub(crate) fn fake_engine_path() -> String {
    std::env::current_exe()
        .expect("current exe")
        .display()
        .to_string()
}

/// Redirects resume persistence into a per-test tempdir and restores the
/// previous `CHESS_TUI_RESUME_DIR` on drop. Tests that use it share a mutex
/// because environment variables are process-global.
pub(crate) struct ResumeDirGuard {
    _tempdir: tempfile::TempDir,
    _lock: MutexGuard<'static, ()>,
    previous: Option<String>,
}

impl ResumeDirGuard {
    pub(crate) fn new() -> Self {
        let lock = RESUME_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        ui::sound::set_sound_enabled(false);
        let previous = std::env::var("CHESS_TUI_RESUME_DIR").ok();
        let tempdir = tempfile::tempdir().expect("resume tempdir");
        // SAFETY: env vars are global; the lock above serialises access.
        unsafe {
            std::env::set_var("CHESS_TUI_RESUME_DIR", tempdir.path());
        }
        Self {
            _tempdir: tempdir,
            _lock: lock,
            previous,
        }
    }
}

impl Drop for ResumeDirGuard {
    fn drop(&mut self) {
        // SAFETY: see `new` — env vars are global, mutex is held.
        unsafe {
            match &self.previous {
                Some(value) => std::env::set_var("CHESS_TUI_RESUME_DIR", value),
                None => std::env::remove_var("CHESS_TUI_RESUME_DIR"),
            }
        }
    }
}
