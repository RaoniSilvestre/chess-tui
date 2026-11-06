//! Game-mode selection and configuration page.

mod render;

use chess_tui_app::game_mode::{BOT_DIFFICULTY_COUNT, TIME_CONTROL_CUSTOM_INDEX};
use chess_tui_app::{App, AvailableGameMode, Nav, Page};
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
    cursor: u8,
}

impl GameModeMenuPage {
    pub fn new() -> Self {
        Self { cursor: 0 }
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

    fn form_left(&self, app: &mut App, kind: AvailableGameMode) {
        let state = app.game_mode_state_mut();
        match kind {
            AvailableGameMode::Local => match LocalFormField::from(state.form_cursor) {
                LocalFormField::TimeControl => {
                    if state.clock_cursor > 0 {
                        state.clock_cursor -= 1;
                    }
                }
                LocalFormField::CustomTime => {
                    if state.clock_cursor == TIME_CONTROL_CUSTOM_INDEX
                        && state.custom_time_minutes > 1
                    {
                        state.custom_time_minutes -= 1;
                    }
                }
            },
            AvailableGameMode::Multiplayer => match MultiplayerFormField::from(state.form_cursor) {
                MultiplayerFormField::HostSelection => app.game_mode_set_hosting(Some(true)),
                MultiplayerFormField::ColorSelection => {
                    if app.game_mode_hosting() == Some(true) {
                        app.game_mode_state_mut().select_previous_color_option();
                    }
                }
            },
            AvailableGameMode::Bot => match BotFormField::from(state.form_cursor) {
                BotFormField::TimeControl => {
                    if state.clock_cursor > 0 {
                        state.clock_cursor -= 1;
                    }
                }
                BotFormField::CustomTime => {
                    if state.clock_cursor == TIME_CONTROL_CUSTOM_INDEX {
                        if state.custom_time_minutes > 1 {
                            state.custom_time_minutes -= 1;
                        }
                    } else {
                        app.game_mode_state_mut().select_previous_color_option();
                    }
                }
                BotFormField::ColorSelection => {
                    if state.clock_cursor == TIME_CONTROL_CUSTOM_INDEX {
                        app.game_mode_state_mut().select_previous_color_option();
                    } else if app.game_mode_bot_depth() > 1 {
                        let depth = app.game_mode_bot_depth() - 1;
                        app.game_mode_set_bot_depth(depth);
                    }
                }
                BotFormField::BotDepthSelection => {
                    if state.clock_cursor == TIME_CONTROL_CUSTOM_INDEX {
                        if app.game_mode_bot_depth() > 1 {
                            let depth = app.game_mode_bot_depth() - 1;
                            app.game_mode_set_bot_depth(depth);
                        }
                    } else {
                        app.game_mode_cycle_difficulty(false);
                    }
                }
                BotFormField::DifficultySelection => app.game_mode_cycle_difficulty(false),
            },
            AvailableGameMode::PGNLoader => {}
        }
    }

    fn form_right(&self, app: &mut App, kind: AvailableGameMode) {
        let state = app.game_mode_state_mut();
        match kind {
            AvailableGameMode::Local => match LocalFormField::from(state.form_cursor) {
                LocalFormField::TimeControl => {
                    if state.clock_cursor < TIME_CONTROL_CUSTOM_INDEX {
                        state.clock_cursor += 1;
                    }
                }
                LocalFormField::CustomTime => {
                    if state.clock_cursor == TIME_CONTROL_CUSTOM_INDEX
                        && state.custom_time_minutes < 120
                    {
                        state.custom_time_minutes += 1;
                    }
                }
            },
            AvailableGameMode::Multiplayer => match MultiplayerFormField::from(state.form_cursor) {
                MultiplayerFormField::HostSelection => app.game_mode_set_hosting(Some(false)),
                MultiplayerFormField::ColorSelection => {
                    if app.game_mode_hosting() == Some(true) {
                        app.game_mode_state_mut().select_next_color_option();
                    }
                }
            },
            AvailableGameMode::Bot => match BotFormField::from(state.form_cursor) {
                BotFormField::TimeControl => {
                    if state.clock_cursor < TIME_CONTROL_CUSTOM_INDEX {
                        state.clock_cursor += 1;
                    }
                }
                BotFormField::CustomTime => {
                    if state.clock_cursor == TIME_CONTROL_CUSTOM_INDEX {
                        if state.custom_time_minutes < 120 {
                            state.custom_time_minutes += 1;
                        }
                    } else {
                        app.game_mode_state_mut().select_next_color_option();
                    }
                }
                BotFormField::ColorSelection => {
                    if state.clock_cursor == TIME_CONTROL_CUSTOM_INDEX {
                        app.game_mode_state_mut().select_next_color_option();
                    } else if app.game_mode_bot_depth() < 20 {
                        let depth = app.game_mode_bot_depth() + 1;
                        app.game_mode_set_bot_depth(depth);
                    }
                }
                BotFormField::BotDepthSelection => {
                    if state.clock_cursor == TIME_CONTROL_CUSTOM_INDEX {
                        if app.game_mode_bot_depth() < 20 {
                            let depth = app.game_mode_bot_depth() + 1;
                            app.game_mode_set_bot_depth(depth);
                        }
                    } else {
                        app.game_mode_cycle_difficulty(true);
                    }
                }
                BotFormField::DifficultySelection => app.game_mode_cycle_difficulty(true),
            },
            AvailableGameMode::PGNLoader => {}
        }
    }

    fn form_enter(&self, app: &mut App, kind: AvailableGameMode) {
        let state = app.game_mode_state_mut();
        match kind {
            AvailableGameMode::Local => match LocalFormField::from(state.form_cursor) {
                LocalFormField::TimeControl => {
                    if state.clock_cursor == TIME_CONTROL_CUSTOM_INDEX {
                        state.form_cursor = 1;
                    } else {
                        app.game_mode_submit(AvailableGameMode::Local);
                    }
                }
                LocalFormField::CustomTime => app.game_mode_submit(AvailableGameMode::Local),
            },
            AvailableGameMode::Multiplayer => match MultiplayerFormField::from(state.form_cursor) {
                MultiplayerFormField::HostSelection => {
                    if app.game_mode_hosting().is_none() {
                        app.game_mode_set_hosting(Some(true));
                    }
                    if app.game_mode_hosting() == Some(true) {
                        app.game_mode_state_mut().form_cursor = 1;
                    } else {
                        app.game_mode_submit(AvailableGameMode::Multiplayer);
                    }
                }
                MultiplayerFormField::ColorSelection => {
                    let state = app.game_mode_state_mut();
                    if state.selected_color.is_none() && !state.is_random_color {
                        state.selected_color = Some(Color::White);
                    }
                    app.game_mode_submit(AvailableGameMode::Multiplayer);
                }
            },
            AvailableGameMode::Bot => match BotFormField::from(state.form_cursor) {
                BotFormField::TimeControl => state.form_cursor = 1,
                BotFormField::CustomTime => {
                    if state.clock_cursor != TIME_CONTROL_CUSTOM_INDEX
                        && state.selected_color.is_none()
                        && !state.is_random_color
                    {
                        state.selected_color = Some(Color::White);
                    }
                    state.form_cursor = 2;
                }
                BotFormField::ColorSelection => {
                    if state.clock_cursor == TIME_CONTROL_CUSTOM_INDEX
                        && state.selected_color.is_none()
                        && !state.is_random_color
                    {
                        state.selected_color = Some(Color::White);
                    }
                    state.form_cursor = 3;
                }
                BotFormField::BotDepthSelection => {
                    if state.clock_cursor == TIME_CONTROL_CUSTOM_INDEX {
                        state.form_cursor = 4;
                    } else {
                        app.game_mode_submit(AvailableGameMode::Bot);
                    }
                }
                BotFormField::DifficultySelection => app.game_mode_submit(AvailableGameMode::Bot),
            },
            AvailableGameMode::PGNLoader => {}
        }
    }

    fn handle_form(&mut self, key: KeyEvent, app: &mut App, kind: AvailableGameMode) {
        match key.code {
            KeyCode::Esc => {
                let state = app.game_mode_state_mut();
                state.form_active = false;
                state.form_cursor = 0;
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {}
            KeyCode::Left | KeyCode::Char('h') => self.form_left(app, kind),
            KeyCode::Right | KeyCode::Char('l') => self.form_right(app, kind),
            KeyCode::Char(' ') | KeyCode::Enter => self.form_enter(app, kind),
            KeyCode::Char('r' | 'R') => {
                if app.game_mode_has_resume(kind) {
                    let state = app.game_mode_state_mut();
                    state.form_active = false;
                    state.form_cursor = 0;
                    app.game_mode_resume(kind);
                }
            }
            KeyCode::Char('?') => app.open_help(),
            KeyCode::Char('q') => app.quit(),
            _ => {}
        }
    }

    fn handle_menu(&mut self, key: KeyEvent, app: &mut App, kind: AvailableGameMode) {
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
            KeyCode::Char(' ') | KeyCode::Enter => {
                let state = app.game_mode_state_mut();
                state.form_active = true;
                state.form_cursor = 0;
                state.selection = Some(kind);
                match kind {
                    AvailableGameMode::Local => {
                        if state.clock_cursor > TIME_CONTROL_CUSTOM_INDEX {
                            state.clock_cursor = 3;
                        }
                    }
                    AvailableGameMode::PGNLoader => {
                        state.form_active = false;
                        app.game_mode_open_pgn_loader();
                    }
                    _ => {
                        app.game_mode_set_hosting(None);
                        app.game_mode_state_mut().selected_color = None;
                    }
                }
            }
            KeyCode::Char('r' | 'R') => {
                if app.game_mode_has_resume(kind) {
                    app.game_mode_resume(kind);
                }
            }
            KeyCode::Esc | KeyCode::Char('b') => {
                let state = app.game_mode_state_mut();
                state.selection = None;
                state.form_cursor = 0;
                state.form_active = false;
                state.clock_cursor = 3;
                state.custom_time_minutes = 10;
                app.go_home();
            }
            KeyCode::Char('?') => app.open_help(),
            KeyCode::Char('q') => app.quit(),
            _ => {}
        }
        app.game_mode_set_menu_cursor(self.cursor);
    }
}

impl Default for GameModeMenuPage {
    fn default() -> Self {
        Self::new()
    }
}

impl Page for GameModeMenuPage {
    fn handle_key(&mut self, key: KeyEvent, app: &mut App) -> Nav {
        if self.cursor >= AvailableGameMode::COUNT {
            self.cursor = 0;
        }
        let Some(kind) = self.current_mode() else {
            return Nav::None;
        };
        if app.game_mode_state().form_active {
            self.handle_form(key, app, kind);
        } else {
            self.handle_menu(key, app, kind);
        }
        Nav::None
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, app: &mut App) {
        render::render_game_mode_menu(frame, area, app, self.cursor);
    }
}

const _: usize = BOT_DIFFICULTY_COUNT;
