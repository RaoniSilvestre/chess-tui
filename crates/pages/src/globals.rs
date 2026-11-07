//! Truly application-wide state: persisted config, loaded theme, terminal
//! capabilities, sound/animation flags, and global overlays.
//!
//! Page-specific state must not live here.

use config::{Args, Config, ConfigStore};
use theme::{DisplayMode, PieceStyle, Skin};
use ui::graphics::KittyPieces;

use crate::overlay::Overlay;

pub struct Theme {
    pub loaded_skin: Option<Skin>,
    pub available_skins: Vec<Skin>,
    pub available_piece_styles: Vec<PieceStyle>,
    pub selected_skin_name: String,
    pub display_mode: DisplayMode,
}

impl Theme {
    pub fn get_skin(&self, next: bool) -> Skin {
        if self.available_skins.is_empty() {
            return Skin::default();
        }

        let current_index = self
            .available_skins
            .iter()
            .position(|s| s.name == self.selected_skin_name)
            .unwrap_or(0);

        if next {
            let next_index = (current_index + 1) % self.available_skins.len();
            self.available_skins[next_index].clone()
        } else {
            let previous_index = if current_index == 0 {
                self.available_skins.len() - 1
            } else {
                current_index - 1
            };
            self.available_skins[previous_index].clone()
        }
    }

    pub fn cycle(&mut self, next: bool) {
        let skin = self.get_skin(next);
        self.selected_skin_name = skin.name.clone();
        self.display_mode = match skin.name.as_str() {
            "Default" => DisplayMode::DEFAULT,
            "ASCII" => DisplayMode::ASCII,
            _ => DisplayMode::CUSTOM,
        };
        self.loaded_skin = Some(skin);
    }

    pub fn apply_to_game(&self, game: &mut session::Game) {
        if let Some(skin) = &self.loaded_skin {
            game.ui.skin = skin.clone();
        }
        game.ui.display_mode = self.display_mode;
        game.ui.available_piece_styles = self.available_piece_styles.clone();
    }
}

pub struct Globals {
    pub config: ConfigStore,
    pub theme: Theme,
    pub sound_enabled: bool,
    pub animations_enabled: bool,
    pub kitty_graphics_enabled: bool,
    pub kitty_pieces: Option<KittyPieces>,
    pub overlay: Option<Overlay>,
}

impl Globals {
    /// Builds the application-wide state from the persisted config and CLI args.
    pub fn from_config(config: &Config, args: &Args) -> Self {
        let store = config::config_path()
            .map(ConfigStore::new)
            .unwrap_or_else(|_| ConfigStore::disabled());
        let mut globals = Self {
            config: store,
            theme: Theme {
                loaded_skin: Some(Skin::default()),
                available_skins: Vec::new(),
                available_piece_styles: Vec::new(),
                selected_skin_name: "Default".to_string(),
                display_mode: DisplayMode::DEFAULT,
            },
            sound_enabled: true,
            animations_enabled: false,
            kitty_graphics_enabled: true,
            kitty_pieces: None,
            overlay: None,
        };

        globals.sound_enabled = config.sound_enabled.unwrap_or(true);
        if !ui::sound::check_audio_availability() || args.no_sound {
            globals.sound_enabled = false;
        }
        ui::sound::set_sound_enabled(globals.sound_enabled);

        globals.animations_enabled = config.animations_enabled.unwrap_or(false);
        globals.kitty_graphics_enabled = config.kitty_graphics_enabled.unwrap_or(true);
        globals.kitty_pieces = ui::graphics::KittyPieces::detect();

        if let Some(mode) = &config.display_mode {
            globals.theme.display_mode = match mode.as_str() {
                "ASCII" => DisplayMode::ASCII,
                "CUSTOM" => DisplayMode::CUSTOM,
                _ => DisplayMode::DEFAULT,
            };
        }

        globals.load_skins();
        globals.select_skin(config.selected_skin_name.as_deref(), args.skin.as_deref());
        globals
    }

    fn load_skins(&mut self) {
        self.theme.available_skins.push(Skin::default());
        self.theme.available_skins.push(Skin::ascii_display_mode());

        let Ok(skins_path) = config::skins_path() else {
            return;
        };
        if !skins_path.exists()
            && let Err(e) = Skin::create_default_skins_file(&skins_path)
        {
            log::error!("Error creating default skin file: {e}");
        }

        match Skin::load_all_skins(&skins_path) {
            Ok(skins) => self.theme.available_skins.extend(
                skins
                    .into_iter()
                    .filter(|s| s.name != "Default" && s.name != "ASCII"),
            ),
            Err(e) => log::error!("Error retrieving skins from file: {e}"),
        }
        match Skin::load_all_piece_styles(&skins_path) {
            Ok(styles) => self.theme.available_piece_styles.extend(
                styles
                    .into_iter()
                    .filter(|s| s.name != "Default" && s.name != "ASCII"),
            ),
            Err(e) => log::error!("Error retrieving the piece styles from file: {e}"),
        }
    }

    fn select_skin(&mut self, configured: Option<&str>, cli_override: Option<&str>) {
        let name = cli_override.or(configured).unwrap_or("Default").to_string();
        self.theme.selected_skin_name = name.clone();
        let skin = Skin::get_skin_by_name(&self.theme.available_skins, &name)
            .or_else(|| self.theme.available_skins.first().cloned());
        if let Some(skin) = skin {
            self.theme.selected_skin_name = skin.name.clone();
            self.theme.display_mode = match skin.name.as_str() {
                "Default" => DisplayMode::DEFAULT,
                "ASCII" => DisplayMode::ASCII,
                _ => DisplayMode::CUSTOM,
            };
            self.theme.loaded_skin = Some(skin);
        }
    }

    /// Persists the UI-only settings. Bot/engine/Lichess fields are written by
    /// the page that edits them so unrelated settings are never overwritten.
    pub fn persist_ui_config(&self) {
        self.config.update(|config| {
            config.display_mode = Some(self.theme.display_mode.to_string());
            config.selected_skin_name = Some(self.theme.selected_skin_name.clone());
            config.sound_enabled = Some(self.sound_enabled);
            config.animations_enabled = Some(self.animations_enabled);
            config.kitty_graphics_enabled = Some(self.kitty_graphics_enabled);
        });
    }

    pub fn bot_settings(&self) -> BotSettings {
        let config = self.config.load();
        BotSettings {
            engine_path: config.engine_path.unwrap_or_default(),
            depth: config.bot_depth.unwrap_or(10),
            difficulty: config.bot_difficulty,
        }
    }

    pub fn set_bot_settings(&self, settings: &BotSettings) {
        self.config.update(|config| {
            config.engine_path = Some(settings.engine_path.clone());
            config.bot_depth = Some(settings.depth);
            config.bot_difficulty = settings.difficulty;
        });
    }

    pub fn lichess_token(&self) -> Option<String> {
        self.config
            .load()
            .lichess_token
            .filter(|token| !token.is_empty())
    }

    pub fn set_lichess_token(&self, token: Option<String>) {
        self.config.update(|config| {
            config.lichess_token = token;
        });
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BotSettings {
    pub engine_path: String,
    pub depth: u8,
    pub difficulty: Option<u8>,
}
