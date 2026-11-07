//! Read-modify-write access to `config.toml`.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::Config;

/// A handle to the persisted configuration file.
///
/// `disabled()` keeps values in memory only. That is what tests and embedded
/// use cases get by default; the real application injects a store backed by
/// the user's config path.
#[derive(Clone, Debug)]
pub struct ConfigStore {
    path: Option<PathBuf>,
    memory: Arc<Mutex<Config>>,
}

impl ConfigStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: Some(path.into()),
            memory: Arc::new(Mutex::new(Config::default())),
        }
    }

    /// A store that never touches disk but still preserves updates in memory.
    pub fn disabled() -> Self {
        Self {
            path: None,
            memory: Arc::new(Mutex::new(Config::default())),
        }
    }

    /// Loads the file, falling back to defaults when it is missing or invalid.
    pub fn load(&self) -> Config {
        let Some(path) = &self.path else {
            return self
                .memory
                .lock()
                .map(|config| config.clone())
                .unwrap_or_default();
        };
        match std::fs::read_to_string(path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => Config::default(),
        }
    }

    /// Persists `config`, logging but never propagating write failures.
    pub fn save(&self, config: &Config) {
        if let Ok(mut memory) = self.memory.lock() {
            *memory = config.clone();
        }
        let Some(path) = &self.path else {
            return;
        };
        let toml_string = toml::to_string(config).unwrap_or_default();
        match std::fs::File::create(path) {
            Ok(mut file) => {
                use std::io::Write;
                if let Err(e) = file.write_all(toml_string.as_bytes()) {
                    Config::handle_config_write_error(e, path);
                }
            }
            Err(e) => Config::handle_config_write_error(e, path),
        }
    }

    /// Re-reads the file, applies `f`, then writes it back.
    pub fn update(&self, f: impl FnOnce(&mut Config)) {
        let mut config = self.load();
        f(&mut config);
        self.save(&config);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_store_keeps_updates_in_memory() {
        let store = ConfigStore::disabled();
        store.update(|config| {
            config.bot_depth = Some(42);
            config.sound_enabled = Some(false);
        });
        let loaded = store.load();
        assert_eq!(loaded.bot_depth, Some(42));
        assert_eq!(loaded.sound_enabled, Some(false));
    }
}
