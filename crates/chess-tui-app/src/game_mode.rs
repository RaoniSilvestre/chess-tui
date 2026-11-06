//! Game-mode selection and configuration form state.

use chess_tui_core::TimeControl;
use shakmaty::Color;

pub const TIME_CONTROL_CUSTOM_INDEX: u8 = 6;
pub const TIME_CONTROL_OPTIONS: &[&str] = &[
    "UltraBullet",
    "Bullet",
    "Blitz",
    "Rapid",
    "Classical",
    "No clock",
    "Custom",
];
pub const BOT_DIFFICULTY_NAMES: [&str; 4] =
    ["Easy (400)", "Medium (900)", "Hard (1500)", "Magnus (2700)"];
pub const BOT_DIFFICULTY_COUNT: usize = 4;

/// The four game modes available from the game-mode selection menu.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum AvailableGameMode {
    Local,
    Multiplayer,
    Bot,
    PGNLoader,
}

impl AvailableGameMode {
    pub const COUNT: u8 = 4;
}

/// Form state and settings captured during game-mode configuration.
pub struct GameModeState {
    pub selection: Option<AvailableGameMode>,
    pub form_cursor: u8,
    pub form_active: bool,
    pub clock_cursor: u8,
    pub custom_time_minutes: u16,
    pub is_random_color: bool,
    pub selected_color: Option<Color>,
}

impl Default for GameModeState {
    fn default() -> Self {
        Self {
            selection: None,
            form_cursor: 0,
            form_active: false,
            clock_cursor: 3,
            custom_time_minutes: 10,
            selected_color: None,
            is_random_color: false,
        }
    }
}

impl GameModeState {
    pub fn reset_selected_color(&mut self) {
        self.selected_color = None;
        self.is_random_color = false;
    }

    pub fn get_time_control_name(&self) -> &'static str {
        match self.clock_cursor {
            0 => "UltraBullet",
            1 => "Bullet",
            2 => "Blitz",
            3 => "Rapid",
            4 => "Classical",
            5 => "No clock",
            x if x == TIME_CONTROL_CUSTOM_INDEX => "Custom",
            _ => "Rapid",
        }
    }

    pub fn get_time_control(&self) -> TimeControl {
        match self.clock_cursor {
            0 => TimeControl::UltraBullet,
            1 => TimeControl::Bullet,
            2 => TimeControl::Blitz,
            3 => TimeControl::Rapid,
            4 => TimeControl::Classical,
            5 => TimeControl::NoClock,
            x if x == TIME_CONTROL_CUSTOM_INDEX => {
                TimeControl::Custom(self.custom_time_minutes as u64)
            }
            _ => TimeControl::Rapid,
        }
    }

    pub fn get_time_control_seconds(&self) -> Option<u32> {
        match self.clock_cursor {
            0 => Some(15),
            1 => Some(60),
            2 => Some(5 * 60),
            3 => Some(10 * 60),
            4 => Some(60 * 60),
            5 => None,
            x if x == TIME_CONTROL_CUSTOM_INDEX => Some((self.custom_time_minutes * 60).into()),
            _ => Some(10 * 60),
        }
    }

    pub fn get_time_control_description(&self) -> String {
        match self.clock_cursor {
            0 => "Lightning fast (15 seconds per side)".to_string(),
            1 => "Very short games (e.g., 1 minute per side)".to_string(),
            2 => "Fast games (e.g., 5 minutes)".to_string(),
            3 => "Medium games (e.g., 10 minutes)".to_string(),
            4 => "Longer games (e.g., 60 minutes)".to_string(),
            5 => "Play without any time limits".to_string(),
            x if x == TIME_CONTROL_CUSTOM_INDEX => {
                format!("Custom time: {} minutes per side", self.custom_time_minutes)
            }
            _ => "Medium games (e.g., 10 minutes)".to_string(),
        }
    }

    pub fn select_previous_color_option(&mut self) {
        if self.selected_color == Some(Color::White)
            || self.selected_color.is_none() && !self.is_random_color
        {
            self.selected_color = None;
            self.is_random_color = true;
        } else if self.is_random_color {
            self.selected_color = Some(Color::Black);
            self.is_random_color = false;
        } else if self.selected_color == Some(Color::Black) {
            self.selected_color = Some(Color::White);
            self.is_random_color = false;
        }
    }

    pub fn select_next_color_option(&mut self) {
        if self.selected_color == Some(Color::White)
            || self.selected_color.is_none() && !self.is_random_color
        {
            self.selected_color = Some(Color::Black);
            self.is_random_color = false;
        } else if self.selected_color == Some(Color::Black) {
            self.selected_color = None;
            self.is_random_color = true;
        } else if self.is_random_color {
            self.selected_color = Some(Color::White);
            self.is_random_color = false;
        }
    }

    pub fn resolve_selected_color(&mut self) {
        if self.is_random_color && self.selected_color.is_none() {
            self.selected_color = Some(if rand::random::<bool>() {
                Color::White
            } else {
                Color::Black
            });
        } else if !self.is_random_color && self.selected_color.is_none() {
            self.selected_color = Some(Color::White);
        }
    }
}
