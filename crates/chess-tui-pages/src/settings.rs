//! Settings page.

use chess_tui_app::{App, Nav, Page};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

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
}

impl SettingsPage {
    pub fn new() -> Self {
        Self { cursor: 0 }
    }

    fn items(app: &App) -> Vec<SettingsItem> {
        let mut items = vec![SettingsItem::Skin];
        if app.settings_has_sound() {
            items.push(SettingsItem::Sound);
        }
        items.push(SettingsItem::Animations);
        items.push(SettingsItem::EnginePath);
        items.push(SettingsItem::BotDepth);
        items.push(SettingsItem::BotDifficulty);
        if app.settings_kitty_available() {
            items.push(SettingsItem::KittyGraphics);
        }
        items
    }

    fn current_item(&self, app: &App) -> Option<SettingsItem> {
        let items = Self::items(app);
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
}

impl Default for SettingsPage {
    fn default() -> Self {
        Self::new()
    }
}

impl Page for SettingsPage {
    fn handle_key(&mut self, key: KeyEvent, app: &mut App) -> Nav {
        let items = Self::items(app);
        let len = items.len();

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.move_up(len),
            KeyCode::Down | KeyCode::Char('j') => self.move_down(len),
            KeyCode::Left | KeyCode::Char('h') => {
                if let Some(item) = self.current_item(app) {
                    match item {
                        SettingsItem::Skin => app.settings_cycle_skin(false),
                        SettingsItem::BotDepth => app.settings_bot_depth_down(),
                        SettingsItem::BotDifficulty => app.settings_cycle_difficulty(false),
                        _ => {}
                    }
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if let Some(item) = self.current_item(app) {
                    match item {
                        SettingsItem::Skin => app.settings_cycle_skin(true),
                        SettingsItem::BotDepth => app.settings_bot_depth_up(),
                        SettingsItem::BotDifficulty => app.settings_cycle_difficulty(true),
                        _ => {}
                    }
                }
            }
            KeyCode::Char('s' | 'S') => app.settings_cycle_skin(true),
            KeyCode::Char(' ') | KeyCode::Enter => {
                if let Some(item) = self.current_item(app) {
                    match item {
                        SettingsItem::Skin => app.settings_cycle_skin(true),
                        SettingsItem::Sound => app.settings_toggle_sound(),
                        SettingsItem::Animations => app.settings_toggle_animations(),
                        SettingsItem::EnginePath => app.settings_open_engine_path(),
                        SettingsItem::BotDepth => {}
                        SettingsItem::BotDifficulty => app.settings_cycle_difficulty(true),
                        SettingsItem::KittyGraphics => app.settings_toggle_kitty(),
                    }
                }
            }
            KeyCode::Esc | KeyCode::Char('b') => app.go_home(),
            KeyCode::Char('?') => app.open_help(),
            KeyCode::Char('q') => app.quit(),
            _ => {}
        }

        Nav::None
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, app: &mut App) {
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

        let items = Self::items(app);
        let mut menu_items: Vec<(String, &str)> = Vec::new();
        for item in &items {
            match item {
                SettingsItem::Skin => {
                    menu_items.push((app.settings_skin_label(), "Change display theme"))
                }
                SettingsItem::Sound => {
                    menu_items.push((app.settings_sound_label(), "Toggle sound effects"))
                }
                SettingsItem::Animations => {
                    menu_items.push((app.settings_animations_label(), "Toggle animation effects"))
                }
                SettingsItem::EnginePath => menu_items.push((
                    "Chess Engine Path".to_string(),
                    "Set engine path with command-line arguments",
                )),
                SettingsItem::BotDepth => menu_items.push((
                    app.settings_bot_depth_label(),
                    "Set bot thinking depth for chess engine",
                )),
                SettingsItem::BotDifficulty => menu_items.push((
                    app.settings_bot_difficulty_label(),
                    "Set bot difficulty for chess engine",
                )),
                SettingsItem::KittyGraphics => menu_items.push((
                    app.settings_kitty_label(),
                    "Draw pieces as images in Kitty (Off uses skin pieces)",
                )),
            }
        }

        let mut menu_lines = vec![Line::from("")];
        for (idx, (option, description)) in menu_items.iter().enumerate() {
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
                Span::styled(option.clone(), style),
            ]));
            menu_lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(*description, Style::default().fg(Color::Gray)),
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
    }
}
