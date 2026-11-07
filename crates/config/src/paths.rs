//! Filesystem locations used by chess-tui.

use std::path::PathBuf;

/// Returns the user's config directory path.
///
/// # Errors
///
/// Returns an error if the config directory cannot be determined.
pub fn config_dir() -> Result<PathBuf, &'static str> {
    dirs::config_dir().ok_or("Could not get config directory")
}

/// Directory owned by chess-tui inside the platform config directory.
pub fn chess_tui_dir() -> Result<PathBuf, &'static str> {
    Ok(config_dir()?.join("chess-tui"))
}

/// Path of the persisted `config.toml`.
pub fn config_path() -> Result<PathBuf, &'static str> {
    Ok(chess_tui_dir()?.join("config.toml"))
}

/// Path of the user's `skins.json`.
pub fn skins_path() -> Result<PathBuf, &'static str> {
    Ok(chess_tui_dir()?.join("skins.json"))
}

/// Directory containing resume saves.
///
/// `CHESS_TUI_RESUME_DIR` overrides the location entirely so integration tests
/// can redirect persistence into a tempdir without touching the real config.
pub fn resume_dir() -> Result<PathBuf, &'static str> {
    if let Ok(custom) = std::env::var("CHESS_TUI_RESUME_DIR") {
        return Ok(PathBuf::from(custom));
    }
    chess_tui_dir()
}
