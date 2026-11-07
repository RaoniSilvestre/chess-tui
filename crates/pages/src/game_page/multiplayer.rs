//! TCP multiplayer game page. Owns the connection state and board.

use game::TcpMode;
use model::GameState;
use net::{NETWORK_PORT, get_host_ip, setup_game_server};
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Position, Rect},
    style::Style,
    text::Line,
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap},
};
use shakmaty::Color;
use ui::prompt::Prompt;

use crate::game_mode::MatchConfig;
use crate::globals::Globals;
use crate::page::{Page, PageAction, PageKind, PageRequest};
use ui::prompt::PromptResult;

use super::board::{BoardAction, GameBoard};

pub struct MultiplayerGamePage {
    board: GameBoard,
    config: MatchConfig,
    hosting: bool,
    host_ip: Option<String>,
    waiting_for_opponent: bool,
    prompt: Option<Prompt>,
    error: Option<String>,
}

impl MultiplayerGamePage {
    pub fn new(config: MatchConfig, _globals: &mut Globals) -> Self {
        Self {
            board: GameBoard::new(),
            config,
            hosting: config.hosting.unwrap_or(false),
            host_ip: None,
            waiting_for_opponent: false,
            prompt: None,
            error: None,
        }
    }

    pub fn game(&self) -> &session::Game {
        self.board.game()
    }

    pub fn game_mut(&mut self) -> &mut session::Game {
        self.board.game_mut()
    }

    fn connect(&mut self, globals: &mut Globals) {
        let other_player_color = if self.hosting {
            self.config.color.map(|c| c.other())
        } else {
            None
        };

        if self.hosting {
            self.waiting_for_opponent = true;
            if let Some(ip) = get_host_ip() {
                self.host_ip = Some(format!("{}:{}", ip, NETWORK_PORT));
            } else {
                log::error!("Could not get local IP, defaulting to 127.0.0.1");
                self.host_ip = Some(format!("127.0.0.1:{}", NETWORK_PORT));
            }
        }

        let addr = self
            .host_ip
            .clone()
            .unwrap_or_else(|| format!("127.0.0.1:{}", NETWORK_PORT));

        match TcpMode::new(&addr, other_player_color) {
            Ok(mode) => {
                if !self.hosting {
                    self.config.color = Some(mode.my_color());
                    self.config.random_color = false;
                }
                self.board.game.runtime.set_mode(Box::new(mode));
                self.waiting_for_opponent = false;
            }
            Err(e) => {
                log::error!("Failed to create TCP peer: {}", e);
                self.host_ip = None;
                self.error = Some(format!("Connection failed: {}", e));
                globals.show_error_then(format!("Connection failed: {}", e), PageRequest::Home);
                return;
            }
        }

        if self.config.color.unwrap_or(Color::White) == Color::Black {
            self.board.game.runtime.flip();
        }
        globals.theme.apply_to_game(&mut self.board.game);
    }

    fn handle_board_action(&mut self, action: BoardAction) -> PageAction {
        match action {
            BoardAction::None => PageAction::None,
            BoardAction::Page(action) => action,
            BoardAction::Restart => PageAction::None,
            BoardAction::OpenMoveInput => {
                let mut prompt = Prompt::new();
                prompt.message = "Enter a move (e.g. e4, Nf3, O-O):".to_string();
                self.prompt = Some(prompt);
                PageAction::None
            }
            BoardAction::PuzzleMove { .. } => PageAction::None,
        }
    }

    fn handle_prompt(&mut self, key: KeyEvent, globals: &mut Globals) -> PageAction {
        let Some(prompt) = self.prompt.as_mut() else {
            return PageAction::None;
        };
        match ui::prompt::handle_key(prompt, key) {
            PromptResult::Cancel => {
                self.prompt = None;
                PageAction::Navigate(PageRequest::Home)
            }
            PromptResult::Submit => {
                let message = prompt.input.trim().to_string();
                self.prompt = None;
                if message.is_empty() {
                    return PageAction::Navigate(PageRequest::Home);
                }
                self.host_ip = Some(message);
                self.connect(globals);
                PageAction::None
            }
            PromptResult::None => PageAction::None,
        }
    }
}

impl Page for MultiplayerGamePage {
    fn kind(&self) -> PageKind {
        PageKind::MultiplayerGame
    }
    fn is_modal_active(&self) -> bool {
        self.prompt.is_some() || self.waiting_for_opponent
    }

    fn on_enter(&mut self, globals: &mut Globals) -> PageAction {
        self.config.resolve_color();
        if self.hosting {
            if let Some(color) = self.config.color {
                setup_game_server(color);
            }
            self.connect(globals);
        } else {
            let mut prompt = Prompt::new();
            prompt.message = "Enter the host IP address:".to_string();
            self.prompt = Some(prompt);
        }
        PageAction::None
    }

    fn on_exit(&mut self, _globals: &mut Globals) {
        self.board.game.runtime.disconnect();
        self.waiting_for_opponent = false;
    }

    fn handle_key(&mut self, key: KeyEvent, globals: &mut Globals) -> PageAction {
        if self.prompt.is_some() {
            return self.handle_prompt(key, globals);
        }
        if self.waiting_for_opponent {
            if matches!(key.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ')) {
                return PageAction::Navigate(PageRequest::Home);
            }
            return PageAction::None;
        }
        if self.board.end_screen_visible {
            let action = self.board.handle_end_screen_key(key);
            return self.handle_board_action(action);
        }
        if key.code == KeyCode::Char('b') {
            return PageAction::Navigate(PageRequest::Home);
        }
        let action = self.board.handle_common_key(key, globals);
        self.handle_board_action(action)
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, _globals: &mut Globals) -> PageAction {
        let action = self.board.handle_mouse(mouse);
        self.handle_board_action(action)
    }

    fn tick(&mut self, _globals: &mut Globals) -> PageAction {
        self.board.tick();
        PageAction::None
    }

    fn update(&mut self, _globals: &mut Globals) -> PageAction {
        if self.board.game.runtime.state() == GameState::Playing {
            self.board.update();
        }
        PageAction::None
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, globals: &mut Globals) {
        self.board.render(frame, area, globals);
        if self.waiting_for_opponent {
            render_wait_for_other_player(frame, get_host_ip());
        }
        if let Some(prompt) = &self.prompt {
            render_enter_multiplayer_ip(frame, prompt);
        }
    }
}

fn render_enter_multiplayer_ip(frame: &mut Frame<'_>, prompt: &Prompt) {
    let block = Block::default()
        .title("Join a game")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .padding(Padding::horizontal(1))
        .border_style(Style::default().fg(ui::constants::WHITE));
    let area = ui::components::centered_rect::centered_rect(40, 40, frame.area());

    let text = vec![
        Line::from("Enter the ip address and port of the host:").alignment(Alignment::Center),
        Line::from(""),
        Line::from(prompt.input.as_str()),
        Line::from(""),
        Line::from(""),
        Line::from(""),
        Line::from(""),
        Line::from(format!("Example: 10.111.6.50:{};", NETWORK_PORT)),
        Line::from(format!(
            "Documentation: {}/Multiplayer/Online%20multiplayer/",
            crate::constants::DOCS_URL
        )),
        Line::from(""),
        Line::from(""),
        Line::from("Press `Esc` to close the popup.").alignment(Alignment::Center),
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

fn render_wait_for_other_player(frame: &mut Frame<'_>, ip: Option<std::net::IpAddr>) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .padding(Padding::horizontal(1))
        .border_style(Style::default().fg(ui::constants::WHITE));
    let area = ui::components::centered_rect::centered_rect(40, 40, frame.area());
    let inner_area = block.inner(area);

    let ip_str = ip
        .map(|i| i.to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let text = vec![
        Line::from(""),
        Line::from(""),
        Line::from("").alignment(Alignment::Center),
        Line::from(format!(
            "Host IP address and port: {}:{}",
            ip_str, NETWORK_PORT
        ))
        .alignment(Alignment::Center),
    ];

    let paragraph = Paragraph::new(text)
        .block(block.clone())
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    let content_width = "Waiting for other player ...".len() as u16 + 3;

    let throbber_row = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .split(inner_area)[1];

    let throbber_col = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(content_width),
            Constraint::Fill(1),
        ])
        .split(throbber_row)[1];

    let full = throbber_widgets_tui::Throbber::default()
        .label("Waiting for other player ...")
        .throbber_style(
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )
        .throbber_set(ui::constants::CHESS_SET)
        .use_type(throbber_widgets_tui::WhichUse::Spin);

    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    frame.render_widget(paragraph, area);
    frame.render_widget(full, throbber_col);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_globals;

    #[test]
    fn guest_flow_prompts_for_host_ip() {
        let mut globals = test_globals();
        let config = MatchConfig {
            hosting: Some(false),
            ..MatchConfig::default()
        };
        let mut page = MultiplayerGamePage::new(config, &mut globals);

        assert!(matches!(page.on_enter(&mut globals), PageAction::None));
        assert!(page.prompt.is_some());
    }
}
