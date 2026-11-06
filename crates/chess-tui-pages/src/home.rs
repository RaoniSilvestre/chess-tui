//! Home page: title banner, main menu, and navigation.

use chess_tui_app::{App, Nav, Page};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

pub const TITLE: &str = r"
 ██████╗██╗  ██╗███████╗███████╗███████╗   ████████╗██╗   ██╗██╗
██╔════╝██║  ██║██╔════╝██╔════╝██╔════╝   ╚══██╔══╝██║   ██║██║
██║     ███████║█████╗  ███████╗███████╗█████╗██║   ██║   ██║██║
██║     ██╔══██║██╔══╝  ╚════██║╚════██║╚════╝██║   ██║   ██║██║
╚██████╗██║  ██║███████╗███████║███████║      ██║   ╚██████╔╝██║
 ╚═════╝╚═╝  ╚═╝╚══════╝╚══════╝╚══════╝      ╚═╝    ╚═════╝ ╚═╝
";

pub struct HomePage {
    cursor: u8,
}

impl HomePage {
    pub fn new() -> Self {
        Self { cursor: 0 }
    }
}

impl Default for HomePage {
    fn default() -> Self {
        Self::new()
    }
}

impl Page for HomePage {
    fn handle_key(&mut self, key: KeyEvent, app: &mut App) -> Nav {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.cursor = if self.cursor == 0 { 4 } else { self.cursor - 1 };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.cursor = (self.cursor + 1) % 5;
            }
            KeyCode::Char(' ') | KeyCode::Enter => match self.cursor {
                0 => app.open_game_mode_menu(),
                1 => app.open_lichess_menu(),
                2 => app.open_settings(),
                3 => app.open_help(),
                4 => app.open_credits(),
                _ => {}
            },
            KeyCode::Char('?') => app.open_help(),
            KeyCode::Char('q') => app.quit(),
            _ => {}
        }
        Nav::None
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, _app: &mut App) {
        render_home(frame, area, self.cursor);
    }
}

fn render_home(frame: &mut Frame, main_area: Rect, cursor: u8) {
    let menu_items: Vec<(&str, &str)> = vec![
        ("Play Game", "Local, Multiplayer, or Bot game"),
        ("Play on Lichess", "Play on Lichess.org"),
        ("Settings", "View and change settings"),
        ("Help", "View keyboard shortcuts and controls"),
        ("About", "Project information and credits"),
    ];

    let menu_height = menu_items.len() as u16 * 3 + 4;

    let main_layout_horizontal = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Ratio(1, 5),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(menu_height),
            Constraint::Min(0),
            Constraint::Ratio(1, 10),
        ])
        .split(main_area);

    frame.render_widget(
        Paragraph::new(TITLE)
            .alignment(Alignment::Center)
            .block(Block::default()),
        main_layout_horizontal[0],
    );
    frame.render_widget(
        Paragraph::new("A chess game made in 🦀")
            .alignment(Alignment::Center)
            .block(Block::default()),
        main_layout_horizontal[1],
    );

    let mut menu_lines: Vec<Line<'_>> = vec![];
    for (i, (item, description)) in menu_items.iter().enumerate() {
        let is_selected = cursor == i as u8;
        let item_style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightBlue)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let indicator = if is_selected { "▶ " } else { "  " };
        menu_lines.push(Line::from(vec![Span::styled(
            format!("{}{}", indicator, item),
            item_style,
        )]));
        if is_selected {
            menu_lines.push(Line::from(vec![Span::styled(
                format!("   {}", description),
                Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::ITALIC),
            )]));
        } else {
            menu_lines.push(Line::from(""));
        }
        menu_lines.push(Line::from(""));
    }

    frame.render_widget(
        Paragraph::new(menu_lines)
            .alignment(Alignment::Center)
            .block(Block::default()),
        main_layout_horizontal[3],
    );

    let version = env!("CARGO_PKG_VERSION");
    let footer_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Navigation: ", Style::default().fg(Color::Gray)),
            Span::styled("↑/k ", Style::default().fg(Color::Yellow)),
            Span::styled("↓/j ", Style::default().fg(Color::Yellow)),
            Span::styled("| Select: ", Style::default().fg(Color::Gray)),
            Span::styled("Enter/Space", Style::default().fg(Color::Yellow)),
            Span::styled(" | Help: ", Style::default().fg(Color::Gray)),
            Span::styled("?", Style::default().fg(Color::Yellow)),
        ]),
        Line::from(format!("Version: {}", version))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray)),
    ];

    frame.render_widget(
        Paragraph::new(footer_text)
            .alignment(Alignment::Center)
            .block(Block::default()),
        main_layout_horizontal[5],
    );
}
