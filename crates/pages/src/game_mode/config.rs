//! Game-mode selection state shared by the menu and the game pages.

use model::TimeControl;
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

/// Everything a game page needs to start a match. The page owns this after
/// navigation; the menu only builds it.
#[derive(Clone, Copy, Debug)]
pub struct MatchConfig {
    pub time_control: TimeControl,
    pub clock_cursor: u8,
    pub custom_time_minutes: u16,
    pub color: Option<Color>,
    pub random_color: bool,
    pub hosting: Option<bool>,
}

impl Default for MatchConfig {
    fn default() -> Self {
        Self {
            time_control: TimeControl::Rapid,
            clock_cursor: 3,
            custom_time_minutes: 10,
            color: None,
            random_color: false,
            hosting: None,
        }
    }
}

impl MatchConfig {
    pub fn resolve_color(&mut self) -> Color {
        if self.random_color && self.color.is_none() {
            self.color = Some(if rand::random::<bool>() {
                Color::White
            } else {
                Color::Black
            });
        } else if !self.random_color && self.color.is_none() {
            self.color = Some(Color::White);
        }
        self.color.unwrap_or(Color::White)
    }

    pub fn time_control_name(&self) -> &'static str {
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

    pub fn time_control_seconds(&self) -> Option<u32> {
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

    pub fn time_control_description(&self) -> String {
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
        if self.color == Some(Color::White) || self.color.is_none() && !self.random_color {
            self.color = None;
            self.random_color = true;
        } else if self.random_color {
            self.color = Some(Color::Black);
            self.random_color = false;
        } else if self.color == Some(Color::Black) {
            self.color = Some(Color::White);
            self.random_color = false;
        }
    }

    pub fn select_next_color_option(&mut self) {
        if self.color == Some(Color::White) || self.color.is_none() && !self.random_color {
            self.color = Some(Color::Black);
            self.random_color = false;
        } else if self.color == Some(Color::Black) {
            self.color = None;
            self.random_color = true;
        } else if self.random_color {
            self.color = Some(Color::White);
            self.random_color = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shakmaty::Color;

    #[test]
    fn select_previous_color_moves_random_to_black_then_white() {
        let mut config = MatchConfig {
            random_color: true,
            ..MatchConfig::default()
        };

        config.select_previous_color_option();
        assert_eq!(config.color, Some(Color::Black));
        assert!(!config.random_color);

        config.select_previous_color_option();
        assert_eq!(config.color, Some(Color::White));
        assert!(!config.random_color);
    }

    #[test]
    fn select_next_color_cycles_white_black_random() {
        let mut config = MatchConfig::default();

        config.select_next_color_option();
        assert_eq!(config.color, Some(Color::Black));
        assert!(!config.random_color);

        config.select_previous_color_option();
        assert_eq!(config.color, Some(Color::White));

        config.select_next_color_option();
        assert_eq!(config.color, Some(Color::Black));
        assert!(!config.random_color);

        config.select_next_color_option();
        assert_eq!(config.color, None);
        assert!(config.random_color);
    }

    #[test]
    fn resolve_selected_color_defaults_to_white() {
        let mut config = MatchConfig::default();

        assert_eq!(config.resolve_color(), Color::White);
        assert_eq!(config.color, Some(Color::White));
        assert!(!config.random_color);
    }

    #[test]
    fn resolve_selected_color_preserves_random_selection_state() {
        let mut config = MatchConfig {
            random_color: true,
            ..MatchConfig::default()
        };

        config.resolve_color();

        assert!(matches!(
            config.color,
            Some(Color::White) | Some(Color::Black)
        ));
        assert!(config.random_color);
    }
}
