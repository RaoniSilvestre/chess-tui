//! Persisted settings loaded from and written to `config.toml`.

use std::{
    fs,
    io::{self},
    path::Path,
};

use log::LevelFilter;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;

use crate::Args;

/// Persisted settings loaded from and written to `config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub engine_path: Option<String>,
    pub display_mode: Option<String>,
    pub log_level: Option<String>,
    pub bot_depth: Option<u8>,
    /// Bot difficulty preset: None = Off, Some(0..=3) = Easy/Medium/Hard/Magnus.
    pub bot_difficulty: Option<u8>,
    pub selected_skin_name: Option<String>,
    pub lichess_token: Option<String>,
    pub sound_enabled: Option<bool>,
    pub animations_enabled: Option<bool>,
    pub kitty_graphics_enabled: Option<bool>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            engine_path: None,
            display_mode: Some("DEFAULT".to_string()),
            log_level: Some("OFF".to_string()),
            bot_depth: Some(10),
            bot_difficulty: None,
            selected_skin_name: Some("Default".to_string()),
            lichess_token: None,
            sound_enabled: Some(true),
            animations_enabled: Some(false),
            kitty_graphics_enabled: Some(true),
        }
    }
}

impl Config {
    /// Checks if an IO error indicates a read-only filesystem or permission issue.
    /// This handles both EROFS (error code 30) and permission denied errors.
    pub fn is_readonly_error(e: &io::Error) -> bool {
        e.raw_os_error() == Some(30) || e.kind() == io::ErrorKind::PermissionDenied
    }

    /// Handles config file write errors gracefully, logging appropriate warnings.
    /// This allows the application to work with read-only config files (e.g., from NixOS/home-manager).
    pub fn handle_config_write_error(e: io::Error, config_path: &Path) {
        if Self::is_readonly_error(&e) {
            log::warn!(
                "Config file at {:?} is read-only. Settings changes will not be persisted.",
                config_path
            );
        } else {
            log::warn!(
                "Could not write to config file at {:?}: {}. Settings changes will not be persisted.",
                config_path,
                e
            );
        }
    }

    /// Creates or updates the config file, merging CLI `args` on top of any existing values.
    /// Tolerates a read-only config: write errors are logged but never propagated.
    pub fn config_create(args: &Args, folder_path: &Path, config_path: &Path) -> io::Result<Self> {
        std::fs::create_dir_all(folder_path)?;

        let mut config: Config = match fs::read_to_string(config_path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => Config::default(),
        };

        if config.engine_path.as_ref().is_none_or(String::is_empty) {
            if args.engine_path.is_empty() {
                config.engine_path = Some(String::new());
            } else {
                config.engine_path = Some(args.engine_path.clone());
            }
        }

        if config.display_mode.is_none() {
            config.display_mode = Some("DEFAULT".to_string());
        }
        if config.log_level.is_none() {
            config.log_level = Some(LevelFilter::Off.to_string());
        }
        if config.bot_depth.is_none() {
            config.bot_depth = Some(10);
        }
        if config.selected_skin_name.is_none() {
            config.selected_skin_name = Some("Default".to_string());
        }
        if config.sound_enabled.is_none() {
            config.sound_enabled = Some(true);
        }
        if config.animations_enabled.is_none() {
            config.animations_enabled = Some(false);
        }
        if config.kitty_graphics_enabled.is_none() {
            config.kitty_graphics_enabled = Some(true);
        }

        if !args.engine_path.is_empty() {
            config.engine_path = Some(args.engine_path.clone());
        }

        if let Some(token) = &args.lichess_token {
            config.lichess_token = Some(token.clone());
        }

        if let Some(depth) = args.depth {
            config.bot_depth = Some(depth);
        }

        if let Some(ref d) = args.difficulty {
            let idx = match d.to_lowercase().as_str() {
                "easy" => Some(0),
                "medium" => Some(1),
                "hard" => Some(2),
                "magnus" => Some(3),
                _ => None,
            };
            if let Some(i) = idx {
                config.bot_difficulty = Some(i);
            }
        }

        if args.no_sound {
            config.sound_enabled = Some(false);
        }

        let toml_string = toml::to_string(&config).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize config to TOML: {e}"),
            )
        })?;

        match File::create(config_path) {
            Ok(mut file) => {
                if let Err(e) = file.write_all(toml_string.as_bytes()) {
                    Self::handle_config_write_error(e, config_path);
                }
            }
            Err(e) => Self::handle_config_write_error(e, config_path),
        }

        Ok(config)
    }
}

/// Convenience used by the root bootstrap.
pub fn load_or_create(args: &Args) -> io::Result<Config> {
    let folder_path =
        crate::paths::chess_tui_dir().map_err(|e| io::Error::new(io::ErrorKind::NotFound, e))?;
    let config_path = folder_path.join("config.toml");
    Config::config_create(args, &folder_path, &config_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args() -> crate::Args {
        crate::Args {
            engine_path: "test_engine_path".to_string(),
            depth: None,
            difficulty: None,
            lichess_token: None,
            no_sound: false,
            skin: None,
            update_skins: false,
            pgn: None,
        }
    }

    #[test]
    fn config_create_writes_expected_values() {
        let dir = tempfile::tempdir().unwrap();
        let folder_path = dir.path().join("chess-tui");
        let config_path = folder_path.join("config.toml");

        let result = Config::config_create(&args(), &folder_path, &config_path);
        assert!(result.is_ok());
        assert!(config_path.exists());

        let content = std::fs::read_to_string(config_path).unwrap();
        let config: Config = toml::from_str(&content).unwrap();
        assert_eq!(config.engine_path.unwrap(), "test_engine_path");
        assert_eq!(config.display_mode.unwrap(), "DEFAULT");
        assert_eq!(config.bot_depth.unwrap(), 10);
    }
}
