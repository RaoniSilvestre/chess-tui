//! Game-mode selection and configuration page.

mod render;

use model::TimeControl;
use page::{
    AvailableGameMode, BOT_DIFFICULTY_COUNT, GameModeAction, Page, PageAction, PageContext,
    SettingsAction, TIME_CONTROL_CUSTOM_INDEX,
};
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use shakmaty::Color;

#[derive(Clone, Copy, PartialEq, Eq)]
enum LocalFormField {
    TimeControl,
    CustomTime,
}

impl From<u8> for LocalFormField {
    fn from(value: u8) -> Self {
        match value {
            1 => LocalFormField::CustomTime,
            _ => LocalFormField::TimeControl,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MultiplayerFormField {
    HostSelection,
    ColorSelection,
}

impl From<u8> for MultiplayerFormField {
    fn from(value: u8) -> Self {
        match value {
            1 => MultiplayerFormField::ColorSelection,
            _ => MultiplayerFormField::HostSelection,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BotFormField {
    TimeControl,
    CustomTime,
    ColorSelection,
    BotDepthSelection,
    DifficultySelection,
}

impl From<u8> for BotFormField {
    fn from(value: u8) -> Self {
        match value {
            1 => BotFormField::CustomTime,
            2 => BotFormField::ColorSelection,
            3 => BotFormField::BotDepthSelection,
            4 => BotFormField::DifficultySelection,
            _ => BotFormField::TimeControl,
        }
    }
}

pub struct GameModeMenuPage {
    pub(crate) cursor: u8,
    pub(crate) form_active: bool,
    pub(crate) form_cursor: u8,
    pub(crate) clock_cursor: u8,
    pub(crate) custom_time_minutes: u16,
    pub(crate) selected_color: Option<Color>,
    pub(crate) is_random_color: bool,
    pub(crate) hosting: Option<bool>,
}

impl GameModeMenuPage {
    pub fn new() -> Self {
        Self {
            cursor: 0,
            form_active: false,
            form_cursor: 0,
            clock_cursor: 3,
            custom_time_minutes: 10,
            selected_color: None,
            is_random_color: false,
            hosting: None,
        }
    }

    fn current_mode(&self) -> Option<AvailableGameMode> {
        match self.cursor {
            0 => Some(AvailableGameMode::Local),
            1 => Some(AvailableGameMode::Multiplayer),
            2 => Some(AvailableGameMode::Bot),
            3 => Some(AvailableGameMode::PGNLoader),
            _ => None,
        }
    }

    pub(crate) fn get_time_control_name(&self) -> &'static str {
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

    pub(crate) fn get_time_control_seconds(&self) -> Option<u32> {
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

    pub(crate) fn get_time_control(&self) -> TimeControl {
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

    pub(crate) fn get_time_control_description(&self) -> String {
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

    fn select_previous_color_option(&mut self) {
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

    fn select_next_color_option(&mut self) {
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

    fn resolved_color(&mut self) -> Option<Color> {
        if !self.is_random_color && self.selected_color.is_none() {
            self.selected_color = Some(Color::White);
        }
        self.selected_color
    }

    fn start_action(&mut self, mode: AvailableGameMode) -> PageAction {
        let color = match mode {
            AvailableGameMode::Local => None,
            AvailableGameMode::Multiplayer | AvailableGameMode::Bot => self.resolved_color(),
            AvailableGameMode::PGNLoader => None,
        };
        PageAction::GameMode(GameModeAction::Start {
            mode,
            time_control: self.get_time_control(),
            color,
            random_color: self.is_random_color,
            hosting: self.hosting,
        })
    }

    fn form_left(&mut self, ctx: &PageContext<'_>, kind: AvailableGameMode) -> PageAction {
        match kind {
            AvailableGameMode::Local => match LocalFormField::from(self.form_cursor) {
                LocalFormField::TimeControl => {
                    if self.clock_cursor > 0 {
                        self.clock_cursor -= 1;
                    }
                }
                LocalFormField::CustomTime => {
                    if self.clock_cursor == TIME_CONTROL_CUSTOM_INDEX
                        && self.custom_time_minutes > 1
                    {
                        self.custom_time_minutes -= 1;
                    }
                }
            },
            AvailableGameMode::Multiplayer => match MultiplayerFormField::from(self.form_cursor) {
                MultiplayerFormField::HostSelection => self.hosting = Some(true),
                MultiplayerFormField::ColorSelection => {
                    if self.hosting == Some(true) {
                        self.select_previous_color_option();
                    }
                }
            },
            AvailableGameMode::Bot => match BotFormField::from(self.form_cursor) {
                BotFormField::TimeControl => {
                    if self.clock_cursor > 0 {
                        self.clock_cursor -= 1;
                    }
                }
                BotFormField::CustomTime => {
                    if self.clock_cursor == TIME_CONTROL_CUSTOM_INDEX {
                        if self.custom_time_minutes > 1 {
                            self.custom_time_minutes -= 1;
                        }
                    } else {
                        self.select_previous_color_option();
                    }
                }
                BotFormField::ColorSelection => {
                    if self.clock_cursor == TIME_CONTROL_CUSTOM_INDEX {
                        self.select_previous_color_option();
                    } else if ctx.game_mode.bot_depth > 1 {
                        return PageAction::Settings(SettingsAction::BotDepthDown);
                    }
                }
                BotFormField::BotDepthSelection => {
                    if self.clock_cursor == TIME_CONTROL_CUSTOM_INDEX {
                        if ctx.game_mode.bot_depth > 1 {
                            return PageAction::Settings(SettingsAction::BotDepthDown);
                        }
                    } else {
                        return PageAction::Settings(SettingsAction::CycleDifficulty(false));
                    }
                }
                BotFormField::DifficultySelection => {
                    return PageAction::Settings(SettingsAction::CycleDifficulty(false));
                }
            },
            AvailableGameMode::PGNLoader => {}
        }
        PageAction::None
    }

    fn form_right(&mut self, ctx: &PageContext<'_>, kind: AvailableGameMode) -> PageAction {
        match kind {
            AvailableGameMode::Local => match LocalFormField::from(self.form_cursor) {
                LocalFormField::TimeControl => {
                    if self.clock_cursor < TIME_CONTROL_CUSTOM_INDEX {
                        self.clock_cursor += 1;
                    }
                }
                LocalFormField::CustomTime => {
                    if self.clock_cursor == TIME_CONTROL_CUSTOM_INDEX
                        && self.custom_time_minutes < 120
                    {
                        self.custom_time_minutes += 1;
                    }
                }
            },
            AvailableGameMode::Multiplayer => match MultiplayerFormField::from(self.form_cursor) {
                MultiplayerFormField::HostSelection => self.hosting = Some(false),
                MultiplayerFormField::ColorSelection => {
                    if self.hosting == Some(true) {
                        self.select_next_color_option();
                    }
                }
            },
            AvailableGameMode::Bot => match BotFormField::from(self.form_cursor) {
                BotFormField::TimeControl => {
                    if self.clock_cursor < TIME_CONTROL_CUSTOM_INDEX {
                        self.clock_cursor += 1;
                    }
                }
                BotFormField::CustomTime => {
                    if self.clock_cursor == TIME_CONTROL_CUSTOM_INDEX {
                        if self.custom_time_minutes < 120 {
                            self.custom_time_minutes += 1;
                        }
                    } else {
                        self.select_next_color_option();
                    }
                }
                BotFormField::ColorSelection => {
                    if self.clock_cursor == TIME_CONTROL_CUSTOM_INDEX {
                        self.select_next_color_option();
                    } else if ctx.game_mode.bot_depth < 20 {
                        return PageAction::Settings(SettingsAction::BotDepthUp);
                    }
                }
                BotFormField::BotDepthSelection => {
                    if self.clock_cursor == TIME_CONTROL_CUSTOM_INDEX {
                        if ctx.game_mode.bot_depth < 20 {
                            return PageAction::Settings(SettingsAction::BotDepthUp);
                        }
                    } else {
                        return PageAction::Settings(SettingsAction::CycleDifficulty(true));
                    }
                }
                BotFormField::DifficultySelection => {
                    return PageAction::Settings(SettingsAction::CycleDifficulty(true));
                }
            },
            AvailableGameMode::PGNLoader => {}
        }
        PageAction::None
    }

    fn form_enter(&mut self, kind: AvailableGameMode) -> PageAction {
        match kind {
            AvailableGameMode::Local => match LocalFormField::from(self.form_cursor) {
                LocalFormField::TimeControl => {
                    if self.clock_cursor == TIME_CONTROL_CUSTOM_INDEX {
                        self.form_cursor = 1;
                        PageAction::None
                    } else {
                        self.start_action(AvailableGameMode::Local)
                    }
                }
                LocalFormField::CustomTime => self.start_action(AvailableGameMode::Local),
            },
            AvailableGameMode::Multiplayer => match MultiplayerFormField::from(self.form_cursor) {
                MultiplayerFormField::HostSelection => {
                    if self.hosting.is_none() {
                        self.hosting = Some(true);
                    }
                    if self.hosting == Some(true) {
                        self.form_cursor = 1;
                        PageAction::None
                    } else {
                        self.start_action(AvailableGameMode::Multiplayer)
                    }
                }
                MultiplayerFormField::ColorSelection => {
                    self.start_action(AvailableGameMode::Multiplayer)
                }
            },
            AvailableGameMode::Bot => match BotFormField::from(self.form_cursor) {
                BotFormField::TimeControl => {
                    self.form_cursor = 1;
                    PageAction::None
                }
                BotFormField::CustomTime => {
                    if self.clock_cursor != TIME_CONTROL_CUSTOM_INDEX {
                        self.resolved_color();
                    }
                    self.form_cursor = 2;
                    PageAction::None
                }
                BotFormField::ColorSelection => {
                    if self.clock_cursor == TIME_CONTROL_CUSTOM_INDEX {
                        self.resolved_color();
                    }
                    self.form_cursor = 3;
                    PageAction::None
                }
                BotFormField::BotDepthSelection => {
                    if self.clock_cursor == TIME_CONTROL_CUSTOM_INDEX {
                        self.form_cursor = 4;
                        PageAction::None
                    } else {
                        self.start_action(AvailableGameMode::Bot)
                    }
                }
                BotFormField::DifficultySelection => self.start_action(AvailableGameMode::Bot),
            },
            AvailableGameMode::PGNLoader => PageAction::None,
        }
    }

    fn has_resume(&self, ctx: &PageContext<'_>, kind: AvailableGameMode) -> bool {
        match kind {
            AvailableGameMode::Local => ctx.game_mode.has_local_resume,
            AvailableGameMode::Bot => ctx.game_mode.has_bot_resume,
            _ => false,
        }
    }

    fn handle_form(
        &mut self,
        key: KeyEvent,
        ctx: &PageContext<'_>,
        kind: AvailableGameMode,
    ) -> PageAction {
        match key.code {
            KeyCode::Esc => {
                self.form_active = false;
                self.form_cursor = 0;
                PageAction::None
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                PageAction::None
            }
            KeyCode::Left | KeyCode::Char('h') => self.form_left(ctx, kind),
            KeyCode::Right | KeyCode::Char('l') => self.form_right(ctx, kind),
            KeyCode::Char(' ') | KeyCode::Enter => self.form_enter(kind),
            KeyCode::Char('r' | 'R') => {
                if self.has_resume(ctx, kind) {
                    self.form_active = false;
                    self.form_cursor = 0;
                    PageAction::GameMode(GameModeAction::Resume(kind))
                } else {
                    PageAction::None
                }
            }
            KeyCode::Char('?') => PageAction::OpenHelp,
            KeyCode::Char('q') => PageAction::Quit,
            _ => PageAction::None,
        }
    }

    fn handle_menu(
        &mut self,
        key: KeyEvent,
        ctx: &PageContext<'_>,
        kind: AvailableGameMode,
    ) -> PageAction {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.cursor = if self.cursor == 0 {
                    AvailableGameMode::COUNT - 1
                } else {
                    self.cursor - 1
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.cursor = (self.cursor + 1) % AvailableGameMode::COUNT;
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.cursor < 2 {
                    self.cursor += 1;
                }
            }
            KeyCode::Char(' ') | KeyCode::Enter => match kind {
                AvailableGameMode::PGNLoader => return PageAction::GameMode(GameModeAction::OpenPgnLoader),
                _ => {
                    self.form_active = true;
                    self.form_cursor = 0;
                    self.hosting = None;
                    self.selected_color = None;
                    self.is_random_color = false;
                    if kind == AvailableGameMode::Local
                        && self.clock_cursor > TIME_CONTROL_CUSTOM_INDEX
                    {
                        self.clock_cursor = 3;
                    }
                }
            },
            KeyCode::Char('r' | 'R') => {
                if self.has_resume(ctx, kind) {
                    return PageAction::GameMode(GameModeAction::Resume(kind));
                }
            }
            KeyCode::Esc | KeyCode::Char('b') => {
                self.form_active = false;
                self.form_cursor = 0;
                self.clock_cursor = 3;
                self.custom_time_minutes = 10;
                return PageAction::OpenHome;
            }
            KeyCode::Char('?') => return PageAction::OpenHelp,
            KeyCode::Char('q') => return PageAction::Quit,
            _ => {}
        }
        PageAction::None
    }
}

impl Default for GameModeMenuPage {
    fn default() -> Self {
        Self::new()
    }
}

impl Page for GameModeMenuPage {
    fn handle_key(&mut self, key: KeyEvent, ctx: &PageContext<'_>) -> PageAction {
        if self.cursor >= AvailableGameMode::COUNT {
            self.cursor = 0;
        }
        let Some(kind) = self.current_mode() else {
            return PageAction::None;
        };
        if self.form_active {
            self.handle_form(key, ctx, kind)
        } else {
            self.handle_menu(key, ctx, kind)
        }
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, ctx: &mut PageContext<'_>) {
        render::render_game_mode_menu(frame, area, ctx, self);
    }

    fn menu_cursor(&self) -> Option<u8> {
        Some(self.cursor)
    }

    fn form_active(&self) -> Option<bool> {
        Some(self.form_active)
    }
}

const _: usize = BOT_DIFFICULTY_COUNT;
