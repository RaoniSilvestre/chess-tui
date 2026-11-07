//! Game-mode selection and configuration page.
//!
//! The page owns the form draft. It emits a `PageRequest` carrying the final
//! `MatchConfig`/`BotGameConfig`; the game page owns that state afterwards.

mod config;
mod render;

pub use config::{
    AvailableGameMode, BOT_DIFFICULTY_COUNT, BOT_DIFFICULTY_NAMES, MatchConfig,
    TIME_CONTROL_CUSTOM_INDEX, TIME_CONTROL_OPTIONS,
};

use model::TimeControl;
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap};
use shakmaty::Color;
use ui::prompt::Prompt;

use crate::game_page::BotGameConfig;
use crate::globals::Globals;
use crate::page::{Page, PageAction, PageKind, PageRequest};
use crate::resume::{self, ResumeMode};
use ui::prompt::PromptResult;

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
    confirm_start: Option<AvailableGameMode>,
    pgn_prompt: Option<Prompt>,
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
            confirm_start: None,
            pgn_prompt: None,
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

    fn match_config(&mut self, mode: AvailableGameMode) -> MatchConfig {
        let color = match mode {
            AvailableGameMode::Local | AvailableGameMode::PGNLoader => None,
            AvailableGameMode::Multiplayer | AvailableGameMode::Bot => self.resolved_color(),
        };
        MatchConfig {
            time_control: self.get_time_control(),
            clock_cursor: self.clock_cursor,
            custom_time_minutes: self.custom_time_minutes,
            color,
            random_color: self.is_random_color,
            hosting: self.hosting,
        }
    }

    fn start_action(
        &mut self,
        mode: AvailableGameMode,
        globals: &mut Globals,
        resume: bool,
    ) -> PageAction {
        let config = self.match_config(mode);
        match mode {
            AvailableGameMode::Local => {
                if resume {
                    PageAction::Navigate(PageRequest::LocalGame {
                        config,
                        resume: true,
                    })
                } else if resume::has_save(ResumeMode::Local) {
                    self.confirm_start = Some(mode);
                    PageAction::None
                } else {
                    resume::delete(ResumeMode::Local);
                    PageAction::Navigate(PageRequest::LocalGame {
                        config,
                        resume: false,
                    })
                }
            }
            AvailableGameMode::Bot => {
                let bot = BotGameConfig {
                    settings: globals.bot_settings(),
                    player_color: config.color.unwrap_or(Color::White),
                    random_color: config.random_color,
                };
                if resume {
                    PageAction::Navigate(PageRequest::BotGame {
                        config,
                        bot,
                        resume: true,
                    })
                } else if resume::has_save(ResumeMode::Bot) {
                    self.confirm_start = Some(mode);
                    PageAction::None
                } else {
                    resume::delete(ResumeMode::Bot);
                    PageAction::Navigate(PageRequest::BotGame {
                        config,
                        bot,
                        resume: false,
                    })
                }
            }
            AvailableGameMode::Multiplayer => {
                PageAction::Navigate(PageRequest::MultiplayerGame { config })
            }
            AvailableGameMode::PGNLoader => {
                let mut prompt = Prompt::new();
                prompt.message = "Enter the path to a PGN file or directory:".to_string();
                self.pgn_prompt = Some(prompt);
                PageAction::None
            }
        }
    }

    fn form_left(&mut self, globals: &mut Globals, kind: AvailableGameMode) -> PageAction {
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
                    } else {
                        let mut settings = globals.bot_settings();
                        if settings.depth > 1 {
                            settings.depth -= 1;
                            globals.set_bot_settings(&settings);
                        }
                    }
                }
                BotFormField::BotDepthSelection => {
                    if self.clock_cursor == TIME_CONTROL_CUSTOM_INDEX {
                        let mut settings = globals.bot_settings();
                        if settings.depth > 1 {
                            settings.depth -= 1;
                            globals.set_bot_settings(&settings);
                        }
                    } else {
                        cycle_difficulty(globals, false);
                    }
                }
                BotFormField::DifficultySelection => cycle_difficulty(globals, false),
            },
            AvailableGameMode::PGNLoader => {}
        }
        PageAction::None
    }

    fn form_right(&mut self, globals: &mut Globals, kind: AvailableGameMode) -> PageAction {
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
                    } else {
                        let mut settings = globals.bot_settings();
                        if settings.depth < 20 {
                            settings.depth += 1;
                            globals.set_bot_settings(&settings);
                        }
                    }
                }
                BotFormField::BotDepthSelection => {
                    if self.clock_cursor == TIME_CONTROL_CUSTOM_INDEX {
                        let mut settings = globals.bot_settings();
                        if settings.depth < 20 {
                            settings.depth += 1;
                            globals.set_bot_settings(&settings);
                        }
                    } else {
                        cycle_difficulty(globals, true);
                    }
                }
                BotFormField::DifficultySelection => cycle_difficulty(globals, true),
            },
            AvailableGameMode::PGNLoader => {}
        }
        PageAction::None
    }

    fn form_enter(&mut self, kind: AvailableGameMode, globals: &mut Globals) -> PageAction {
        match kind {
            AvailableGameMode::Local => match LocalFormField::from(self.form_cursor) {
                LocalFormField::TimeControl => {
                    if self.clock_cursor == TIME_CONTROL_CUSTOM_INDEX {
                        self.form_cursor = 1;
                        PageAction::None
                    } else {
                        self.start_action(AvailableGameMode::Local, globals, false)
                    }
                }
                LocalFormField::CustomTime => {
                    self.start_action(AvailableGameMode::Local, globals, false)
                }
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
                        self.start_action(AvailableGameMode::Multiplayer, globals, false)
                    }
                }
                MultiplayerFormField::ColorSelection => {
                    self.start_action(AvailableGameMode::Multiplayer, globals, false)
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
                        self.start_action(AvailableGameMode::Bot, globals, false)
                    }
                }
                BotFormField::DifficultySelection => {
                    self.start_action(AvailableGameMode::Bot, globals, false)
                }
            },
            AvailableGameMode::PGNLoader => PageAction::None,
        }
    }

    fn has_resume(&self, kind: AvailableGameMode) -> bool {
        match kind {
            AvailableGameMode::Local => resume::has_save(ResumeMode::Local),
            AvailableGameMode::Bot => resume::has_save(ResumeMode::Bot),
            _ => false,
        }
    }

    fn handle_form(
        &mut self,
        key: KeyEvent,
        globals: &mut Globals,
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
            KeyCode::Left | KeyCode::Char('h') => self.form_left(globals, kind),
            KeyCode::Right | KeyCode::Char('l') => self.form_right(globals, kind),
            KeyCode::Char(' ') | KeyCode::Enter => self.form_enter(kind, globals),
            KeyCode::Char('r' | 'R') => {
                if self.has_resume(kind) {
                    self.form_active = false;
                    self.form_cursor = 0;
                    self.start_action(kind, globals, true)
                } else {
                    PageAction::None
                }
            }
            KeyCode::Char('?') => {
                globals.show_help();
                PageAction::None
            }
            KeyCode::Char('q') => PageAction::Quit,
            _ => PageAction::None,
        }
    }

    fn handle_menu(
        &mut self,
        key: KeyEvent,
        globals: &mut Globals,
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
                AvailableGameMode::PGNLoader => {
                    let mut prompt = Prompt::new();
                    prompt.message = "Enter the path to a PGN file or directory:".to_string();
                    self.pgn_prompt = Some(prompt);
                }
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
                if self.has_resume(kind) {
                    return self.start_action(kind, globals, true);
                }
            }
            KeyCode::Esc | KeyCode::Char('b') => {
                self.form_active = false;
                self.form_cursor = 0;
                self.clock_cursor = 3;
                self.custom_time_minutes = 10;
                return PageAction::Navigate(PageRequest::Home);
            }
            KeyCode::Char('?') => globals.show_help(),
            KeyCode::Char('q') => return PageAction::Quit,
            _ => {}
        }
        PageAction::None
    }

    fn handle_pgn_prompt(&mut self, key: KeyEvent) -> PageAction {
        let Some(prompt) = self.pgn_prompt.as_mut() else {
            return PageAction::None;
        };
        match ui::prompt::handle_key(prompt, key) {
            PromptResult::Cancel => {
                self.pgn_prompt = None;
                PageAction::None
            }
            PromptResult::Submit => {
                let path = prompt.input.trim().to_string();
                if path.is_empty() {
                    self.pgn_prompt = None;
                    return PageAction::None;
                }
                match model::PgnViewer::from_file(&path) {
                    Ok(games) => {
                        self.pgn_prompt = None;
                        PageAction::Navigate(PageRequest::PgnViewer { games, index: 0 })
                    }
                    Err(e) => {
                        prompt.error = Some(format!("Failed to load PGN: {e}"));
                        PageAction::None
                    }
                }
            }
            PromptResult::None => PageAction::None,
        }
    }
}

fn cycle_difficulty(globals: &mut Globals, next: bool) {
    let mut settings = globals.bot_settings();
    if next {
        settings.difficulty = match settings.difficulty {
            None => Some(0),
            Some(i) if i + 1 >= BOT_DIFFICULTY_COUNT as u8 => None,
            Some(i) => Some(i + 1),
        };
    } else {
        settings.difficulty = match settings.difficulty {
            None => Some((BOT_DIFFICULTY_COUNT - 1) as u8),
            Some(0) => None,
            Some(i) => Some(i - 1),
        };
    }
    globals.set_bot_settings(&settings);
}

impl Default for GameModeMenuPage {
    fn default() -> Self {
        Self::new()
    }
}

impl Page for GameModeMenuPage {
    fn kind(&self) -> PageKind {
        PageKind::GameModeMenu
    }
    fn is_modal_active(&self) -> bool {
        self.pgn_prompt.is_some() || self.confirm_start.is_some()
    }

    fn handle_key(&mut self, key: KeyEvent, globals: &mut Globals) -> PageAction {
        if self.pgn_prompt.is_some() {
            return self.handle_pgn_prompt(key);
        }
        if let Some(mode) = self.confirm_start {
            match key.code {
                KeyCode::Char('y' | 'Y') => {
                    match mode {
                        AvailableGameMode::Local => resume::delete(ResumeMode::Local),
                        AvailableGameMode::Bot => resume::delete(ResumeMode::Bot),
                        _ => {}
                    }
                    self.confirm_start = None;
                    return self.start_action(mode, globals, false);
                }
                KeyCode::Char('r' | 'R') => {
                    self.confirm_start = None;
                    return self.start_action(mode, globals, true);
                }
                KeyCode::Char('n' | 'N') | KeyCode::Esc => {
                    self.confirm_start = None;
                }
                _ => {}
            }
            return PageAction::None;
        }
        if self.cursor >= AvailableGameMode::COUNT {
            self.cursor = 0;
        }
        let Some(kind) = self.current_mode() else {
            return PageAction::None;
        };
        if self.form_active {
            self.handle_form(key, globals, kind)
        } else {
            self.handle_menu(key, globals, kind)
        }
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, globals: &mut Globals) {
        render::render_game_mode_menu(
            frame,
            area,
            self,
            globals.bot_settings(),
            resume::has_save(ResumeMode::Local),
            resume::has_save(ResumeMode::Bot),
        );
        if self.confirm_start.is_some() {
            ui::overlay::new_game::render_new_game_confirmation_popup(frame, false);
        }
        if let Some(prompt) = &self.pgn_prompt {
            render_pgn_path_prompt(frame, prompt);
        }
    }
}

// Keep the constants module referenced so downstream pages can import them.
const _: usize = BOT_DIFFICULTY_COUNT;

fn render_pgn_path_prompt(frame: &mut Frame<'_>, prompt: &Prompt) {
    let block = Block::default()
        .title("Load PGN")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .padding(Padding::horizontal(1))
        .border_style(Style::default().fg(ui::color::board_white()));
    let area = ui::components::centered_rect::centered_rect(60, 35, frame.area());

    let text = vec![
        Line::from("Enter the absolute path to a .pgn file:")
            .alignment(ratatui::layout::Alignment::Center),
        Line::from(""),
        Line::from(prompt.input.as_str()),
        Line::from(""),
        Line::from(""),
        Line::from("Multi-game PGN files are supported."),
        Line::from(""),
        Line::from("Press `Enter` to load, `Esc` to cancel.")
            .alignment(ratatui::layout::Alignment::Center),
    ];

    let paragraph = Paragraph::new(text)
        .block(block.clone())
        .alignment(ratatui::layout::Alignment::Left)
        .wrap(Wrap { trim: true });

    frame.set_cursor_position(Position::new(
        area.x + prompt.character_index as u16 + 2,
        area.y + 3,
    ));

    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::{Page, PageAction, PageRequest};
    use crate::resume::{ResumeMode, SavedGame, has_save, save};
    use crate::test_support::{ResumeDirGuard, send, test_globals};
    use ratatui::crossterm::event::KeyCode;

    fn existing_local_save() {
        let saved = SavedGame {
            mode: ResumeMode::Local,
            fen: "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2".into(),
            is_flipped: false,
            taken_pieces: vec![],
            bot: None,
            clock: None,
            moves_uci: vec!["e2e4".into(), "e7e5".into()],
        };
        save(ResumeMode::Local, &saved);
    }

    fn is_local_game(action: PageAction, resume: bool) -> bool {
        matches!(
            action,
            PageAction::Navigate(PageRequest::LocalGame {
                resume: expected,
                ..
            }) if expected == resume
        )
    }

    #[test]
    fn esc_closes_pgn_popup_and_returns_control_to_menu() {
        let mut globals = test_globals();
        let mut page = GameModeMenuPage::new();

        send(
            &mut page,
            &mut globals,
            &[KeyCode::Down, KeyCode::Down, KeyCode::Down, KeyCode::Enter],
        );
        assert!(page.pgn_prompt.is_some());

        send(&mut page, &mut globals, &[KeyCode::Esc]);
        assert!(page.pgn_prompt.is_none());

        send(&mut page, &mut globals, &[KeyCode::Down]);
        assert_eq!(page.cursor, 0, "menu must stay navigable after closing");
    }

    #[test]
    fn empty_pgn_path_closes_popup_and_returns_control_to_menu() {
        let mut globals = test_globals();
        let mut page = GameModeMenuPage::new();

        send(
            &mut page,
            &mut globals,
            &[KeyCode::Down, KeyCode::Down, KeyCode::Down, KeyCode::Enter],
        );
        assert!(page.pgn_prompt.is_some());

        send(&mut page, &mut globals, &[KeyCode::Enter]);
        assert!(page.pgn_prompt.is_none());

        send(&mut page, &mut globals, &[KeyCode::Down]);
        assert_eq!(page.cursor, 0);
    }

    #[test]
    fn pressing_r_in_local_form_resumes() {
        let _guard = ResumeDirGuard::new();
        existing_local_save();
        let mut globals = test_globals();
        let mut page = GameModeMenuPage::new();

        send(&mut page, &mut globals, &[KeyCode::Enter]);
        assert!(page.form_active);

        let action = page.handle_key(crate::test_support::key(KeyCode::Char('r')), &mut globals);
        assert!(is_local_game(action, true));
        assert!(has_save(ResumeMode::Local));
    }

    #[test]
    fn pressing_r_in_menu_navigation_resumes() {
        let _guard = ResumeDirGuard::new();
        existing_local_save();
        let mut globals = test_globals();
        let mut page = GameModeMenuPage::new();

        let action = page.handle_key(crate::test_support::key(KeyCode::Char('r')), &mut globals);
        assert!(is_local_game(action, true));
    }

    #[test]
    fn r_with_no_save_is_inert() {
        let _guard = ResumeDirGuard::new();
        let mut globals = test_globals();
        let mut page = GameModeMenuPage::new();

        let action = page.handle_key(crate::test_support::key(KeyCode::Char('r')), &mut globals);
        assert!(matches!(action, PageAction::None));
        assert_eq!(page.cursor, 0);
    }

    #[test]
    fn new_game_over_a_save_can_be_cancelled() {
        let _guard = ResumeDirGuard::new();
        existing_local_save();

        for cancel in [KeyCode::Char('n'), KeyCode::Esc] {
            let mut globals = test_globals();
            let mut page = GameModeMenuPage::new();
            send(&mut page, &mut globals, &[KeyCode::Enter, KeyCode::Enter]);
            assert!(page.confirm_start.is_some());
            send(&mut page, &mut globals, &[cancel]);
            assert!(page.confirm_start.is_none());
            assert!(has_save(ResumeMode::Local), "{cancel:?} must keep the save");
        }
    }

    #[test]
    fn confirming_new_game_clears_the_old_save() {
        let _guard = ResumeDirGuard::new();
        existing_local_save();
        let mut globals = test_globals();
        let mut page = GameModeMenuPage::new();

        send(&mut page, &mut globals, &[KeyCode::Enter, KeyCode::Enter]);
        assert!(page.confirm_start.is_some());

        let action = page.handle_key(crate::test_support::key(KeyCode::Char('y')), &mut globals);
        assert!(is_local_game(action, false));
        assert!(!has_save(ResumeMode::Local));
    }

    #[test]
    fn confirmation_r_resumes_the_save() {
        let _guard = ResumeDirGuard::new();
        existing_local_save();
        let mut globals = test_globals();
        let mut page = GameModeMenuPage::new();

        send(&mut page, &mut globals, &[KeyCode::Enter, KeyCode::Enter]);
        let action = page.handle_key(crate::test_support::key(KeyCode::Char('r')), &mut globals);
        assert!(is_local_game(action, true));
        assert!(has_save(ResumeMode::Local));
    }
}
