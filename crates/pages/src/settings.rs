//! Settings page.

use page::{Page, PageAction, PageContext, SettingsAction};
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

    fn items(ctx: &PageContext<'_>) -> Vec<SettingsItem> {
        let mut items = vec![SettingsItem::Skin];
        if ctx.settings.has_sound {
            items.push(SettingsItem::Sound);
        }
        items.push(SettingsItem::Animations);
        items.push(SettingsItem::EnginePath);
        items.push(SettingsItem::BotDepth);
        items.push(SettingsItem::BotDifficulty);
        if ctx.settings.kitty_available {
            items.push(SettingsItem::KittyGraphics);
        }
        items
    }

    fn current_item(&self, ctx: &PageContext<'_>) -> Option<SettingsItem> {
        let items = Self::items(ctx);
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

    fn label(item: SettingsItem, ctx: &PageContext<'_>) -> String {
        match item {
            SettingsItem::Skin => format!("Skin: {}", ctx.settings.skin_name),
            SettingsItem::Sound => format!(
                "Sound: {}",
                if ctx.settings.sound_enabled {
                    "On 🔊"
                } else {
                    "Off 🔇"
                }
            ),
            SettingsItem::Animations => format!(
                "Animations: {}",
                if ctx.settings.animations_enabled {
                    "On"
                } else {
                    "Off"
                }
            ),
            SettingsItem::EnginePath => "Chess Engine Path".to_string(),
            SettingsItem::BotDepth => format!("Bot Depth: -  {}  +", ctx.settings.bot_depth),
            SettingsItem::BotDifficulty => {
                let difficulty = match ctx.settings.bot_difficulty {
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
                if ctx.settings.kitty_graphics_enabled {
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
}

impl Default for SettingsPage {
    fn default() -> Self {
        Self::new()
    }
}

impl Page for SettingsPage {
    fn handle_key(&mut self, key: KeyEvent, ctx: &PageContext<'_>) -> PageAction {
        let items = Self::items(ctx);
        let len = items.len();

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.move_up(len),
            KeyCode::Down | KeyCode::Char('j') => self.move_down(len),
            KeyCode::Left | KeyCode::Char('h') => {
                if let Some(item) = self.current_item(ctx) {
                    return match item {
                        SettingsItem::Skin => PageAction::Settings(SettingsAction::CycleSkin(false)),
                        SettingsItem::BotDepth => {
                            PageAction::Settings(SettingsAction::BotDepthDown)
                        }
                        SettingsItem::BotDifficulty => {
                            PageAction::Settings(SettingsAction::CycleDifficulty(false))
                        }
                        _ => PageAction::None,
                    };
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if let Some(item) = self.current_item(ctx) {
                    return match item {
                        SettingsItem::Skin => PageAction::Settings(SettingsAction::CycleSkin(true)),
                        SettingsItem::BotDepth => PageAction::Settings(SettingsAction::BotDepthUp),
                        SettingsItem::BotDifficulty => {
                            PageAction::Settings(SettingsAction::CycleDifficulty(true))
                        }
                        _ => PageAction::None,
                    };
                }
            }
            KeyCode::Char('s' | 'S') => {
                return PageAction::Settings(SettingsAction::CycleSkin(true));
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                if let Some(item) = self.current_item(ctx) {
                    return match item {
                        SettingsItem::Skin => PageAction::Settings(SettingsAction::CycleSkin(true)),
                        SettingsItem::Sound => PageAction::Settings(SettingsAction::ToggleSound),
                        SettingsItem::Animations => {
                            PageAction::Settings(SettingsAction::ToggleAnimations)
                        }
                        SettingsItem::EnginePath => {
                            PageAction::Settings(SettingsAction::OpenEnginePath)
                        }
                        SettingsItem::BotDepth => PageAction::None,
                        SettingsItem::BotDifficulty => {
                            PageAction::Settings(SettingsAction::CycleDifficulty(true))
                        }
                        SettingsItem::KittyGraphics => {
                            PageAction::Settings(SettingsAction::ToggleKitty)
                        }
                    };
                }
            }
            KeyCode::Esc | KeyCode::Char('b') => return PageAction::OpenHome,
            KeyCode::Char('?') => return PageAction::OpenHelp,
            KeyCode::Char('q') => return PageAction::Quit,
            _ => {}
        }

        PageAction::None
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, ctx: &mut PageContext<'_>) {
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

        let items = Self::items(ctx);
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
                Span::styled(Self::label(*item, ctx), style),
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
    }
}
