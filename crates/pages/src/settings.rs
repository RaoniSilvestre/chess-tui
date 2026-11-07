//! Settings page. Owns its cursor and engine-path prompt; edits global config.

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap},
};
use ui::prompt::Prompt;
use ui::sound::play_menu_nav_sound;

use crate::game_mode::BOT_DIFFICULTY_COUNT;
use crate::globals::Globals;
use crate::page::{Page, PageAction, PageKind, PageRequest};
use ui::prompt::PromptResult;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SettingsItem {
    Skin,
    Sound,
    Animations,
    EnginePath,
    BotDepth,
    BotDifficulty,
    KittyGraphics,
}

pub struct SettingsPage {
    cursor: usize,
    engine_prompt: Option<Prompt>,
}

impl SettingsPage {
    pub fn new() -> Self {
        Self {
            cursor: 0,
            engine_prompt: None,
        }
    }

    fn items(globals: &Globals) -> Vec<SettingsItem> {
        let mut items = vec![SettingsItem::Skin];
        if cfg!(feature = "sound") {
            items.push(SettingsItem::Sound);
        }
        items.push(SettingsItem::Animations);
        items.push(SettingsItem::EnginePath);
        items.push(SettingsItem::BotDepth);
        items.push(SettingsItem::BotDifficulty);
        if globals.kitty_pieces.is_some() {
            items.push(SettingsItem::KittyGraphics);
        }
        items
    }

    fn current_item(&self, globals: &Globals) -> Option<SettingsItem> {
        let items = Self::items(globals);
        if items.is_empty() {
            None
        } else {
            Some(items[self.cursor.min(items.len() - 1)])
        }
    }

    fn move_up(&mut self, len: usize) {
        if len == 0 {
            self.cursor = 0;
        } else if self.cursor == 0 {
            self.cursor = len - 1;
        } else {
            self.cursor -= 1;
        }
    }

    fn move_down(&mut self, len: usize) {
        if len == 0 {
            self.cursor = 0;
        } else {
            self.cursor = (self.cursor + 1) % len;
        }
    }

    fn label(item: SettingsItem, globals: &Globals) -> String {
        match item {
            SettingsItem::Skin => format!("Skin: {}", globals.theme.selected_skin_name),
            SettingsItem::Sound => format!(
                "Sound: {}",
                if globals.sound_enabled {
                    "On 🔊"
                } else {
                    "Off 🔇"
                }
            ),
            SettingsItem::Animations => format!(
                "Animations: {}",
                if globals.animations_enabled {
                    "On"
                } else {
                    "Off"
                }
            ),
            SettingsItem::EnginePath => "Chess Engine Path".to_string(),
            SettingsItem::BotDepth => {
                format!("Bot Depth: -  {}  +", globals.bot_settings().depth)
            }
            SettingsItem::BotDifficulty => {
                let difficulty = match globals.bot_settings().difficulty {
                    Some(0) => "Easy",
                    Some(1) => "Medium",
                    Some(2) => "Hard",
                    Some(3) => "Magnus",
                    _ => "Off",
                };
                format!("Bot Difficulty: {difficulty}")
            }
            SettingsItem::KittyGraphics => format!(
                "Kitty Graphics: {}",
                if globals.kitty_graphics_enabled {
                    "On"
                } else {
                    "Off"
                }
            ),
        }
    }

    fn description(item: SettingsItem) -> &'static str {
        match item {
            SettingsItem::Skin => "Change display theme",
            SettingsItem::Sound => "Toggle sound effects",
            SettingsItem::Animations => "Toggle animation effects",
            SettingsItem::EnginePath => "Set engine path with command-line arguments",
            SettingsItem::BotDepth => "Set bot thinking depth for chess engine",
            SettingsItem::BotDifficulty => "Set bot difficulty for chess engine",
            SettingsItem::KittyGraphics => "Draw pieces as images in Kitty (Off uses skin pieces)",
        }
    }

    fn cycle_skin(&self, globals: &mut Globals, next: bool) {
        globals.theme.cycle(next);
        globals.persist_ui_config();
        play_menu_nav_sound();
    }

    fn toggle_sound(&self, globals: &mut Globals) {
        #[cfg(feature = "sound")]
        {
            globals.sound_enabled = !globals.sound_enabled;
            ui::sound::set_sound_enabled(globals.sound_enabled);
            globals.persist_ui_config();
        }
        #[cfg(not(feature = "sound"))]
        let _ = globals;
    }

    fn toggle_animations(&self, globals: &mut Globals) {
        globals.animations_enabled = !globals.animations_enabled;
        globals.persist_ui_config();
    }

    fn toggle_kitty(&self, globals: &mut Globals) {
        globals.kitty_graphics_enabled = !globals.kitty_graphics_enabled;
        globals.persist_ui_config();
    }

    fn adjust_depth(&self, globals: &mut Globals, delta: i8) {
        let mut settings = globals.bot_settings();
        if delta < 0 && settings.depth > 1 {
            settings.depth -= 1;
        } else if delta > 0 && settings.depth < 20 {
            settings.depth += 1;
        }
        globals.set_bot_settings(&settings);
        play_menu_nav_sound();
    }

    fn cycle_difficulty(&self, globals: &mut Globals, next: bool) {
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
        play_menu_nav_sound();
    }

    fn open_engine_prompt(&mut self, globals: &Globals) {
        let mut prompt = Prompt::new();
        prompt.input = globals.bot_settings().engine_path;
        prompt.character_index = prompt.input.chars().count();
        prompt.message = "Enter the path to your chess engine:".to_string();
        self.engine_prompt = Some(prompt);
    }

    fn handle_engine_prompt(&mut self, key: KeyEvent, globals: &mut Globals) -> PageAction {
        let Some(prompt) = self.engine_prompt.as_mut() else {
            return PageAction::None;
        };
        match ui::prompt::handle_key(prompt, key) {
            PromptResult::Cancel => {
                self.engine_prompt = None;
                PageAction::None
            }
            PromptResult::Submit => {
                let path = prompt
                    .input
                    .trim()
                    .trim_matches(&['\'', '"'][..])
                    .to_string();
                let mut settings = globals.bot_settings();
                settings.engine_path = path;
                globals.set_bot_settings(&settings);
                self.engine_prompt = None;
                PageAction::None
            }
            PromptResult::None => PageAction::None,
        }
    }
}

impl Default for SettingsPage {
    fn default() -> Self {
        Self::new()
    }
}

impl Page for SettingsPage {
    fn kind(&self) -> PageKind {
        PageKind::Settings
    }
    fn is_modal_active(&self) -> bool {
        self.engine_prompt.is_some()
    }

    fn handle_key(&mut self, key: KeyEvent, globals: &mut Globals) -> PageAction {
        if self.engine_prompt.is_some() {
            return self.handle_engine_prompt(key, globals);
        }
        let items = Self::items(globals);
        let len = items.len();

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.move_up(len),
            KeyCode::Down | KeyCode::Char('j') => self.move_down(len),
            KeyCode::Left | KeyCode::Char('h') => {
                if let Some(item) = self.current_item(globals) {
                    match item {
                        SettingsItem::Skin => self.cycle_skin(globals, false),
                        SettingsItem::BotDepth => self.adjust_depth(globals, -1),
                        SettingsItem::BotDifficulty => self.cycle_difficulty(globals, false),
                        _ => {}
                    }
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if let Some(item) = self.current_item(globals) {
                    match item {
                        SettingsItem::Skin => self.cycle_skin(globals, true),
                        SettingsItem::BotDepth => self.adjust_depth(globals, 1),
                        SettingsItem::BotDifficulty => self.cycle_difficulty(globals, true),
                        _ => {}
                    }
                }
            }
            KeyCode::Char('s' | 'S') => {
                self.cycle_skin(globals, true);
                return PageAction::Handled;
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                if let Some(item) = self.current_item(globals) {
                    match item {
                        SettingsItem::Skin => self.cycle_skin(globals, true),
                        SettingsItem::Sound => self.toggle_sound(globals),
                        SettingsItem::Animations => self.toggle_animations(globals),
                        SettingsItem::EnginePath => self.open_engine_prompt(globals),
                        SettingsItem::BotDepth => {}
                        SettingsItem::BotDifficulty => self.cycle_difficulty(globals, true),
                        SettingsItem::KittyGraphics => self.toggle_kitty(globals),
                    }
                }
            }
            KeyCode::Esc | KeyCode::Char('b') => {
                return PageAction::Navigate(PageRequest::Home);
            }
            KeyCode::Char('?') => globals.show_help(),
            KeyCode::Char('q') => return PageAction::Quit,
            _ => {}
        }

        PageAction::None
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, globals: &mut Globals) {
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(area);

        let title = Paragraph::new("Settings")
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            );
        frame.render_widget(title, main_chunks[0]);

        let items = Self::items(globals);
        let mut menu_lines = vec![Line::from("")];
        for (idx, item) in items.iter().enumerate() {
            let is_selected = self.cursor == idx;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if is_selected { "► " } else { "  " };
            menu_lines.push(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(Self::label(*item, globals), style),
            ]));
            menu_lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(Self::description(*item), Style::default().fg(Color::Gray)),
            ]));
            menu_lines.push(Line::from(""));
        }

        let menu = Paragraph::new(menu_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(""),
            )
            .alignment(Alignment::Center);
        frame.render_widget(menu, main_chunks[1]);

        let footer = Paragraph::new(vec![Line::from(vec![
            Span::styled("↑/↓", Style::default().fg(Color::Cyan)),
            Span::raw(" Navigate  "),
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::raw(" Select  "),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::raw(" Back to Home"),
        ])])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        );
        frame.render_widget(footer, main_chunks[2]);

        if let Some(prompt) = &self.engine_prompt {
            render_engine_path_prompt(frame, prompt);
        }
    }
}

fn render_engine_path_prompt(frame: &mut Frame<'_>, prompt: &Prompt) {
    let block = Block::default()
        .title("Enter Chess Engine Path")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .padding(Padding::horizontal(1))
        .border_style(Style::default().fg(ui::color::board_white()));
    let area = ui::components::centered_rect::centered_rect(70, 20, frame.area());

    let text = vec![
        Line::from("Enter the absolute path of chess engine:").alignment(Alignment::Center),
        Line::from(""),
        Line::from(prompt.input.as_str()),
        Line::from(""),
        Line::from(""),
        Line::from("Press `Enter` to save, `Esc` to cancel.").alignment(Alignment::Center),
    ];

    let paragraph = Paragraph::new(text)
        .block(block.clone())
        .alignment(Alignment::Left)
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
    use crate::test_support::{key, send, test_globals};
    use ratatui::crossterm::event::KeyCode;

    #[test]
    fn esc_returns_home() {
        let mut globals = test_globals();
        let mut page = SettingsPage::new();
        let action = page.handle_key(key(KeyCode::Esc), &mut globals);
        assert!(matches!(action, PageAction::Navigate(PageRequest::Home)));
    }

    #[test]
    #[cfg(feature = "sound")]
    fn toggle_sound() {
        let mut globals = test_globals();
        let mut page = SettingsPage::new();
        let original = globals.sound_enabled;

        send(&mut page, &mut globals, &[KeyCode::Down, KeyCode::Enter]);
        assert_ne!(globals.sound_enabled, original);

        send(&mut page, &mut globals, &[KeyCode::Enter]);
        assert_eq!(globals.sound_enabled, original);
    }

    #[test]
    fn toggle_animations() {
        let mut globals = test_globals();
        let mut page = SettingsPage::new();
        let original = globals.animations_enabled;

        send(
            &mut page,
            &mut globals,
            &[
                KeyCode::Down,
                #[cfg(feature = "sound")]
                KeyCode::Down,
                KeyCode::Enter,
            ],
        );
        assert_ne!(globals.animations_enabled, original);

        send(&mut page, &mut globals, &[KeyCode::Enter]);
        assert_eq!(globals.animations_enabled, original);
    }

    #[test]
    fn open_close_chess_engine_path_popup() {
        let mut globals = test_globals();
        let mut page = SettingsPage::new();

        send(
            &mut page,
            &mut globals,
            &[
                KeyCode::Down,
                KeyCode::Down,
                #[cfg(feature = "sound")]
                KeyCode::Down,
                KeyCode::Enter,
            ],
        );
        assert!(page.engine_prompt.is_some());

        send(&mut page, &mut globals, &[KeyCode::Esc]);
        assert!(page.engine_prompt.is_none());
    }

    #[test]
    fn change_bot_difficulty() {
        let mut globals = test_globals();
        let mut page = SettingsPage::new();
        let original = globals.bot_settings().difficulty;

        send(
            &mut page,
            &mut globals,
            &[
                KeyCode::Down,
                KeyCode::Down,
                KeyCode::Down,
                #[cfg(feature = "sound")]
                KeyCode::Down,
                KeyCode::Down,
                KeyCode::Right,
            ],
        );
        let next = globals.bot_settings().difficulty;
        assert_ne!(next, original);

        send(&mut page, &mut globals, &[KeyCode::Left]);
        assert_eq!(globals.bot_settings().difficulty, original);

        send(&mut page, &mut globals, &[KeyCode::Left]);
        let prev = globals.bot_settings().difficulty;
        assert_ne!(prev, original);
        assert_ne!(prev, next);

        send(&mut page, &mut globals, &[KeyCode::Right]);
        assert_eq!(globals.bot_settings().difficulty, original);
    }

    #[test]
    fn kitty_graphics_hidden_outside_kitty() {
        let mut globals = test_globals();
        assert!(globals.kitty_pieces.is_none());
        let mut page = SettingsPage::new();

        // Up from the first item wraps to the last visible item, which must not be
        // Kitty Graphics when no Kitty terminal was detected.
        send(&mut page, &mut globals, &[KeyCode::Up, KeyCode::Enter]);
        assert!(globals.kitty_graphics_enabled);
    }
}
