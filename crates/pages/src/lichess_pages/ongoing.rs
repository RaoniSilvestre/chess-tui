//! Ongoing Lichess games list.

use std::sync::mpsc::{Receiver, channel};

use lichess::{LichessClient, OngoingGame};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap},
};
use shakmaty::Color as ChessColor;

use super::{LichessGameSpec, LichessUpdate, client_from_globals};
use crate::globals::Globals;
use crate::page::{Page, PageAction, PageKind, PageRequest};

/// Renders the ongoing Lichess games list with navigation hints.
fn render_ongoing_games(frame: &mut Frame, area: Rect, games: &[OngoingGame], cursor: u8) {
    // Create layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(10),   // Games list
            Constraint::Length(3), // Footer
        ])
        .split(area);

    // Title
    let title = Paragraph::new("My Ongoing Lichess Games")
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
    frame.render_widget(title, chunks[0]);

    // Games list
    let mut game_lines = vec![Line::from("")];

    if games.is_empty() {
        game_lines
            .push(Line::from("No ongoing games found.").style(Style::default().fg(Color::Gray)));
        game_lines.push(Line::from(""));
        game_lines.push(Line::from("Use 'Seek Game' to start a new game."));
    } else {
        for (idx, game) in games.iter().enumerate() {
            let is_selected = cursor == idx as u8;

            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let prefix = if is_selected { "► " } else { "  " };

            let opponent_name = &game.opponent.username;
            let rating = game
                .opponent
                .rating
                .map(|r| format!(" ({})", r))
                .unwrap_or_default();
            let turn_indicator = if game.is_my_turn {
                " ⏰ Your turn"
            } else {
                ""
            };

            game_lines.push(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(
                    format!("vs {} {}{}", opponent_name, rating, turn_indicator),
                    style,
                ),
            ]));

            game_lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    format!("Game ID: {} | Color: {}", game.game_id, game.color),
                    Style::default().fg(Color::Gray),
                ),
            ]));

            game_lines.push(Line::from(""));
        }
    }

    let games_widget = Paragraph::new(game_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(format!("{} game(s)", games.len())),
        )
        .alignment(Alignment::Left);
    frame.render_widget(games_widget, chunks[1]);

    // Footer
    let footer = Paragraph::new(vec![Line::from(vec![
        Span::styled("↑/↓", Style::default().fg(Color::Cyan)),
        Span::raw(" Navigate  "),
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::raw(" Join  "),
        Span::styled("R", Style::default().fg(Color::Cyan)),
        Span::raw(" Resign  "),
        Span::styled("Esc", Style::default().fg(Color::Cyan)),
        Span::raw(" Back"),
    ])])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded),
    );
    frame.render_widget(footer, chunks[2]);
}

/// Ongoing Lichess games list page. Owns the fetched list and resign prompt.
pub struct OngoingGamesPage {
    cursor: u8,
    client: Option<LichessClient>,
    games: Vec<OngoingGame>,
    receiver: Option<Receiver<LichessUpdate>>,
    resign_index: Option<usize>,
}

impl OngoingGamesPage {
    pub fn new(globals: &mut Globals) -> Self {
        Self {
            cursor: 0,
            client: client_from_globals(globals),
            games: Vec::new(),
            receiver: None,
            resign_index: None,
        }
    }

    fn fetch(&mut self, globals: &mut Globals) {
        let Some(client) = self.client.clone() else {
            globals.show_error_then(
                "Lichess client not initialized. Configure a Lichess token first.",
                PageRequest::LichessMenu,
            );
            return;
        };
        let (tx, rx) = channel();
        self.receiver = Some(rx);
        if self.games.is_empty() {
            globals.show_loading("Loading your ongoing games ...");
        }
        std::thread::spawn(move || match client.get_ongoing_games() {
            Ok(games) => {
                let _ = tx.send(LichessUpdate::OngoingGamesResult(Ok(games)));
            }
            Err(e) => {
                let _ = tx.send(LichessUpdate::OngoingGamesResult(Err(e.to_string())));
            }
        });
    }

    fn open_selected(&self) -> PageAction {
        let Some(game) = self.games.get(self.cursor as usize) else {
            return PageAction::None;
        };
        let color = if game.color == "white" {
            ChessColor::White
        } else {
            ChessColor::Black
        };
        PageAction::Navigate(PageRequest::LichessGame {
            spec: LichessGameSpec {
                game_id: game.game_id.clone(),
                color,
                fen: Some(game.fen.clone()),
            },
        })
    }

    fn resign_selected(&mut self, globals: &mut Globals) {
        let Some(index) = self.resign_index.take() else {
            return;
        };
        let Some(game) = self.games.get(index) else {
            return;
        };
        let game_id = game.game_id.clone();
        let opponent = game.opponent.username.clone();
        let Some(client) = self.client.clone() else {
            return;
        };
        std::thread::spawn(move || {
            if let Err(e) = client.resign_game(&game_id) {
                log::error!("Failed to resign game {}: {}", game_id, e);
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
            let _ = client.get_ongoing_games();
        });
        globals.show_success(format!(
            "Game resigned successfully!\n\nYou have resigned the game vs {}.\n\nThe game list will be updated shortly.",
            opponent
        ));
    }

    fn poll(&mut self, globals: &mut Globals) -> PageAction {
        let update = match &self.receiver {
            Some(rx) => rx.try_recv().ok(),
            None => None,
        };
        let Some(update) = update else {
            return PageAction::None;
        };
        self.receiver = None;
        globals.close_overlay();
        if let LichessUpdate::OngoingGamesResult(result) = update {
            match result {
                Ok(games) => {
                    self.games = games;
                    if self.cursor as usize >= self.games.len() {
                        self.cursor = self.games.len().saturating_sub(1) as u8;
                    }
                }
                Err(e) => {
                    log::error!("Failed to fetch ongoing games: {e}");
                    globals.show_error(format!("Failed to fetch ongoing games: {e}"));
                }
            }
        }
        PageAction::None
    }
}

impl Page for OngoingGamesPage {
    fn kind(&self) -> PageKind {
        PageKind::OngoingGames
    }
    fn is_modal_active(&self) -> bool {
        self.resign_index.is_some()
    }

    fn on_enter(&mut self, globals: &mut Globals) -> PageAction {
        self.fetch(globals);
        PageAction::None
    }

    fn handle_key(&mut self, key: KeyEvent, globals: &mut Globals) -> PageAction {
        if let Some(index) = self.resign_index {
            match key.code {
                KeyCode::Char('y' | 'Y') | KeyCode::Enter => self.resign_selected(globals),
                KeyCode::Char('n' | 'N') | KeyCode::Esc => self.resign_index = None,
                _ => {}
            }
            let _ = index;
            return PageAction::None;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.cursor = self.cursor.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if (self.cursor as usize) < self.games.len().saturating_sub(1) {
                    self.cursor += 1;
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => return self.open_selected(),
            KeyCode::Char('r' | 'R') => {
                if !self.games.is_empty() {
                    self.resign_index = Some(self.cursor as usize);
                }
            }
            KeyCode::Esc | KeyCode::Char('b') => {
                return PageAction::Navigate(PageRequest::LichessMenu);
            }
            KeyCode::Char('?') => globals.show_help(),
            KeyCode::Char('q') => return PageAction::Quit,
            _ => {}
        }
        PageAction::None
    }

    fn update(&mut self, globals: &mut Globals) -> PageAction {
        self.poll(globals)
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, _globals: &mut Globals) {
        render_ongoing_games(frame, area, &self.games, self.cursor);
        if let Some(index) = self.resign_index {
            let opponent = self
                .games
                .get(index)
                .map(|game| game.opponent.username.as_str());
            render_resign_confirmation_popup(frame, opponent);
        }
    }
}
fn render_resign_confirmation_popup(frame: &mut Frame<'_>, opponent_name: Option<&str>) {
    let block = Block::default()
        .title("Resign Game")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .padding(Padding::horizontal(1))
        .border_style(Style::default().fg(ui::color::board_white()));
    let area = ui::components::centered_rect::centered_rect(50, 30, frame.area());

    let opponent_name = opponent_name
        .map(|name| format!("vs {name}"))
        .unwrap_or_else(|| "this game".to_string());

    let text = vec![
        Line::from(""),
        Line::from(format!(
            "Are you sure you want to resign {}?",
            opponent_name
        ))
        .alignment(Alignment::Center),
        Line::from(""),
        Line::from("This action cannot be undone.").alignment(Alignment::Center),
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Y",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("es / "),
            Span::styled(
                "N",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw("o"),
        ])
        .alignment(Alignment::Center),
    ];

    let paragraph = Paragraph::new(text)
        .block(block.clone())
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    frame.render_widget(paragraph, area);
}
