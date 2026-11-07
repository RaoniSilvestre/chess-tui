//! Renders the user profile, rating sparkline, and menu options for the Lichess landing page.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};

use lichess::{LICHESS_TIME_CONTROLS, LichessClient, RatingHistoryEntry, UserProfile};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap},
};
use ui::prompt::Prompt;

use super::rating_chart::render_rating_history_chart;
use super::{LichessGameSpec, LichessUpdate};
use crate::globals::Globals;
use crate::page::{Page, PageAction, PageKind, PageRequest};
use ui::prompt::PromptResult;

struct LichessLayout {
    title: Rect,
    menu: Rect,
    time_control: Rect,
    profile: Rect,
    chart: Rect,
    footer: Rect,
}

impl LichessLayout {
    fn new(area: Rect) -> Self {
        // Create main layout: title, content (menu + stats), footer
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(10),   // Content (menu + stats)
                Constraint::Length(3), // Footer
            ])
            .split(area);

        let content_stats = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40), // Menu + time control
                Constraint::Percentage(60), // Stats
            ])
            .split(main_chunks[1]);

        let menu_time_control = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(90), // Menu
                Constraint::Length(3),      // Time control
            ])
            .split(content_stats[0]);

        let profile_and_chart = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(2),     // Stats text
                Constraint::Length(24), // Graph
            ])
            .split(content_stats[1]);

        LichessLayout {
            title: main_chunks[0],
            menu: menu_time_control[0],
            time_control: menu_time_control[1],
            profile: profile_and_chart[0],
            chart: profile_and_chart[1],
            footer: main_chunks[2],
        }
    }
}

struct RenderState<'c> {
    profile: Option<&'c UserProfile>,
    history: Option<&'c [RatingHistoryEntry]>,
}

impl RenderState<'_> {
    fn lichess_profile(&self) -> Option<&lichess::UserProfile> {
        self.profile
    }

    fn lichess_rating_history(&self) -> Option<&[lichess::RatingHistoryEntry]> {
        self.history
    }
}

fn render_lichess_menu(
    frame: &mut Frame,
    area: Rect,
    profile: Option<&UserProfile>,
    history: Option<&[RatingHistoryEntry]>,
    page: &LichessMenuPage,
) {
    let state = RenderState { profile, history };
    let cursor = page.cursor;
    let stats_scroll = page.stats_scroll;
    let time_control = page.time_control;
    let layout = LichessLayout::new(area);

    // Title
    let title = Paragraph::new("Lichess Menu")
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
    frame.render_widget(title, layout.title);

    render_menu_panel(frame, layout.menu, cursor);
    render_time_control(frame, layout.time_control, time_control);
    render_user_stats_panel(frame, &state, layout.profile, stats_scroll);
    render_chart_panel(frame, &state, layout.chart);

    // Footer with controls
    let footer = Paragraph::new(vec![Line::from(vec![
        Span::styled("↑/↓", Style::default().fg(Color::Cyan)),
        Span::raw(" Menu  "),
        Span::styled("←/→", Style::default().fg(Color::Cyan)),
        Span::raw(" Time  "),
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
    frame.render_widget(footer, layout.footer);
}

/// Renders the time control selector (Rapid/Classical) using standard chess
/// notation, e.g. `10+0 Rapid`. Only the selected text gets a highlighted
/// background, sized to the text itself.
fn render_time_control(frame: &mut Frame, area: Rect, time_control: u8) {
    let menu = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("Time Control");
    let inner_area = menu.inner(area);
    frame.render_widget(menu, area);

    // Lichess only allows the Rapid (3) and Classical (4) presets.
    let selected = usize::from(time_control >= 4);

    let labels: Vec<String> = LICHESS_TIME_CONTROLS.iter().map(|tc| tc.label()).collect();

    let widths: Vec<Constraint> = labels
        .iter()
        .map(|label| Constraint::Length(label.len() as u16 + 2))
        .collect();
    let button_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(widths)
        .split(inner_area);

    for (idx, label) in labels.iter().enumerate() {
        let style = if idx == selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        // Size the render area to the text so the highlight hugs the label.
        let text_len = label.len() as u16;
        let text_rect = Rect {
            x: button_area[idx].x + button_area[idx].width.saturating_sub(text_len) / 2,
            y: button_area[idx].y,
            width: text_len.min(button_area[idx].width),
            height: button_area[idx].height,
        };
        frame.render_widget(Paragraph::new(label.as_str()).style(style), text_rect);
    }
}

fn render_menu_panel(frame: &mut Frame, area: Rect, cursor: u8) {
    let menu_items = [
        ("Seek Game", "Find a random opponent"),
        ("Puzzle", "Play a puzzle"),
        ("My Ongoing Games", "View and join your current games"),
        ("Join by Code", "Enter a game code to join"),
        ("Disconnect", "Remove Lichess token and logout"),
    ];

    let mut menu_lines = vec![Line::from("")];

    for (idx, (option, description)) in menu_items.iter().enumerate() {
        let is_selected = cursor == idx as u8;
        let is_disconnect = idx == 4;

        let style = if is_selected {
            if is_disconnect {
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Red)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            }
        } else if is_disconnect {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::White)
        };

        let prefix = if is_selected { "► " } else { "  " };
        menu_lines.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(*option, style),
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
                .title("Select an option"),
        )
        .alignment(Alignment::Left);
    frame.render_widget(menu, area);
}

fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.lines() {
        if max_width == 0 || paragraph.len() <= max_width {
            lines.push(paragraph.to_string());
        } else {
            let mut current = String::new();
            for word in paragraph.split_whitespace() {
                if current.is_empty() {
                    current.push_str(word);
                } else if current.len() + 1 + word.len() <= max_width {
                    current.push(' ');
                    current.push_str(word);
                } else {
                    lines.push(current.clone());
                    current = word.to_string();
                }
            }
            if !current.is_empty() {
                lines.push(current);
            }
        }
    }
    lines
}

fn rating_line(label: &str, rating: u16, prog: Option<i16>) -> Line<'static> {
    let text = match prog {
        Some(p) if p > 0 => format!("{} (+{})", rating, p),
        Some(p) if p < 0 => format!("{} ({})", rating, p),
        _ => format!("{}", rating),
    };
    Line::from(vec![
        Span::raw(format!("  {}: ", label)),
        Span::styled(text, Style::default().fg(Color::Yellow)),
    ])
}

fn render_user_stats_panel(
    frame: &mut Frame,
    state: &RenderState<'_>,
    area: Rect,
    stats_scroll: u16,
) {
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("User Stats (Scroll with Page Up/Down)");
    let inner_area = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(58),
            Constraint::Percentage(4),
            Constraint::Percentage(38),
        ])
        .split(inner_area);

    let scroll_offset = stats_scroll as usize;
    let max_visible = inner_area.height as usize;

    if let Some(profile) = state.lichess_profile() {
        // --- LEFT: username, online, bio, location, country, created at ---
        let mut left: Vec<Line> = vec![Line::from("")];

        let username_display = if let Some(title) = &profile.title {
            format!("{} {}", title, profile.username)
        } else {
            profile.username.clone()
        };
        left.push(Line::from(vec![Span::styled(
            "Username:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]));
        left.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(username_display, Style::default().fg(Color::White)),
        ]));
        if let Some(online) = profile.online {
            left.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    if online { "● Online" } else { "○ Offline" },
                    Style::default().fg(if online { Color::Green } else { Color::Gray }),
                ),
            ]));
        }
        left.push(Line::from(""));

        if let Some(profile_info) = &profile.profile {
            if let Some(bio) = &profile_info.bio {
                left.push(Line::from(vec![Span::styled(
                    "Bio:",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )]));
                let max_width = columns[0].width.saturating_sub(2) as usize;
                for line in &wrap_text(bio, max_width) {
                    left.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(line.clone(), Style::default().fg(Color::White)),
                    ]));
                }
                left.push(Line::from(""));
            }
            if let Some(location) = &profile_info.location {
                left.push(Line::from(vec![Span::styled(
                    "Location:",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )]));
                left.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(location.clone(), Style::default().fg(Color::White)),
                ]));
                left.push(Line::from(""));
            }
            if let Some(country) = &profile_info.country {
                left.push(Line::from(vec![Span::styled(
                    "Country:",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )]));
                left.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(country.clone(), Style::default().fg(Color::White)),
                ]));
                left.push(Line::from(""));
            }
        }

        if let Some(created_at_secs) = profile.created_at_secs() {
            use chrono::{DateTime, Utc};
            let date_str = DateTime::<Utc>::from_timestamp(created_at_secs as i64, 0)
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| String::from("Invalid date"));
            left.push(Line::from(vec![Span::styled(
                "Created:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]));
            left.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(date_str, Style::default().fg(Color::White)),
            ]));
            left.push(Line::from(""));
        }

        // --- RIGHT: ratings, games, playtime ---
        let mut right: Vec<Line> = vec![Line::from("")];

        if let Some(perfs) = &profile.perfs {
            right.push(Line::from(vec![Span::styled(
                "Ratings:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]));
            if let Some(p) = &perfs.bullet {
                right.push(rating_line("Bullet", p.rating, p.prog));
            }
            if let Some(p) = &perfs.blitz {
                right.push(rating_line("Blitz", p.rating, p.prog));
            }
            if let Some(p) = &perfs.rapid {
                right.push(rating_line("Rapid", p.rating, p.prog));
            }
            if let Some(p) = &perfs.classical {
                right.push(rating_line("Classical", p.rating, p.prog));
            }
            if let Some(p) = &perfs.puzzle {
                right.push(rating_line("Puzzle", p.rating, p.prog));
            }
            right.push(Line::from(""));
        }

        if let Some(counts) = &profile.count {
            right.push(Line::from(vec![Span::styled(
                "Games:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]));
            if let Some(v) = counts.all {
                right.push(Line::from(vec![
                    Span::raw("  Total: "),
                    Span::styled(format!("{}", v), Style::default().fg(Color::White)),
                ]));
            }
            if let Some(v) = counts.win {
                right.push(Line::from(vec![
                    Span::raw("  Wins: "),
                    Span::styled(format!("{}", v), Style::default().fg(Color::Green)),
                ]));
            }
            if let Some(v) = counts.loss {
                right.push(Line::from(vec![
                    Span::raw("  Losses: "),
                    Span::styled(format!("{}", v), Style::default().fg(Color::Red)),
                ]));
            }
            if let Some(v) = counts.draw {
                right.push(Line::from(vec![
                    Span::raw("  Draws: "),
                    Span::styled(format!("{}", v), Style::default().fg(Color::Yellow)),
                ]));
            }
            if let Some(v) = counts.playing
                && v > 0
            {
                right.push(Line::from(vec![
                    Span::raw("  Playing: "),
                    Span::styled(format!("{}", v), Style::default().fg(Color::Magenta)),
                ]));
            }
            right.push(Line::from(""));
        }

        if let Some(playtime) = &profile.playtime
            && let Some(total_seconds) = playtime.total
        {
            let hours = total_seconds / 3600;
            let minutes = (total_seconds % 3600) / 60;
            let playtime_str = if hours > 0 {
                format!("{}h {}m", hours, minutes)
            } else {
                format!("{}m", minutes)
            };
            right.push(Line::from(vec![Span::styled(
                "Playtime:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]));
            right.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(playtime_str, Style::default().fg(Color::White)),
            ]));
            right.push(Line::from(""));
        }

        if let Some(streamer) = &profile.streamer
            && (streamer.youtube.is_some() || streamer.twitch.is_some())
        {
            right.push(Line::from(vec![Span::styled(
                "Verified Streamer:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]));
            if let Some(youtube) = &streamer.youtube {
                right.push(Line::from(vec![
                    Span::raw("  YouTube: "),
                    Span::styled(youtube.clone(), Style::default().fg(Color::Red)),
                ]));
            }
            if let Some(twitch) = &streamer.twitch {
                right.push(Line::from(vec![
                    Span::raw("  Twitch: "),
                    Span::styled(twitch.clone(), Style::default().fg(Color::Magenta)),
                ]));
            }
            right.push(Line::from(""));
        }

        let left_start = scroll_offset.min(left.len().saturating_sub(1));
        let left_end = (left_start + max_visible).min(left.len());
        let visible_left = left[left_start..left_end].to_vec();

        let right_start = scroll_offset.min(right.len().saturating_sub(1));
        let right_end = (right_start + max_visible).min(right.len());
        let visible_right = right[right_start..right_end].to_vec();

        frame.render_widget(
            Paragraph::new(visible_left).alignment(Alignment::Left),
            columns[0],
        );
        frame.render_widget(
            Paragraph::new(visible_right).alignment(Alignment::Left),
            columns[2],
        );
    } else {
        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                "Loading...",
                Style::default().fg(Color::Gray),
            )]))
            .alignment(Alignment::Left),
            inner_area,
        );
    }
}

fn render_chart_panel(frame: &mut Frame, state: &RenderState<'_>, area: Rect) {
    if let Some(history) = state.lichess_rating_history() {
        render_rating_history_chart(frame, history, area);
    } else {
        let msg = if state.lichess_profile().is_some() {
            "Loading rating history..."
        } else {
            "Loading..."
        };
        let loading_graph = Paragraph::new(msg)
            .style(Style::default().fg(Color::Gray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("Rating History"),
            )
            .alignment(Alignment::Center);
        frame.render_widget(loading_graph, area);
    }
}

enum PromptKind {
    Token,
    GameCode,
}

struct PromptState {
    kind: PromptKind,
    prompt: Prompt,
}

/// Lichess landing menu page. Owns the client, profile, seek channel and
/// page-local prompts.
pub struct LichessMenuPage {
    cursor: u8,
    stats_scroll: u16,
    time_control: u8,
    client: Option<LichessClient>,
    token: Option<String>,
    profile: Option<UserProfile>,
    rating_history: Option<Vec<RatingHistoryEntry>>,
    receiver: Option<Receiver<LichessUpdate>>,
    cancellation: Option<Arc<AtomicBool>>,
    prompt: Option<PromptState>,
    seeking: bool,
}

impl LichessMenuPage {
    pub fn new(globals: &mut Globals) -> Self {
        let token = globals.lichess_token();
        let client = token.clone().map(LichessClient::new);
        Self {
            cursor: 0,
            stats_scroll: 0,
            time_control: 3,
            client,
            token,
            profile: None,
            rating_history: None,
            receiver: None,
            cancellation: None,
            prompt: None,
            seeking: false,
        }
    }

    fn open_prompt(&mut self, kind: PromptKind, message: &str) {
        let mut prompt = Prompt::new();
        prompt.message = message.to_string();
        self.prompt = Some(PromptState { kind, prompt });
    }

    fn fetch_profile(&mut self) {
        let Some(client) = self.client.clone() else {
            return;
        };
        let (tx, rx) = channel();
        self.receiver = Some(rx);
        std::thread::spawn(move || {
            const MAX_ATTEMPTS: u32 = 3;
            const RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(2);
            let mut last_err = String::new();
            for attempt in 1..=MAX_ATTEMPTS {
                match client.get_user_profile() {
                    Ok(profile) => {
                        let username = profile.username.clone();
                        let history = client.get_rating_history(&username).unwrap_or_default();
                        let _ = tx.send(LichessUpdate::ProfileResult(Ok(Box::new((
                            profile, history,
                        )))));
                        return;
                    }
                    Err(e) => {
                        last_err = e.to_string();
                        log::warn!(
                            "Profile fetch attempt {}/{} failed: {}",
                            attempt,
                            MAX_ATTEMPTS,
                            last_err
                        );
                        if attempt < MAX_ATTEMPTS {
                            std::thread::sleep(RETRY_DELAY);
                        }
                    }
                }
            }
            let _ = tx.send(LichessUpdate::ProfileResult(Err(last_err)));
        });
    }

    fn start_seek(&mut self, globals: &mut Globals) {
        let Some(client) = self.client.clone() else {
            self.open_prompt(PromptKind::Token, "Enter your Lichess API token:");
            return;
        };
        let (tx, rx) = channel();
        self.receiver = Some(rx);
        let cancellation = Arc::new(AtomicBool::new(false));
        self.cancellation = Some(cancellation.clone());
        let idx = usize::from(self.time_control >= 4);
        let tc = LICHESS_TIME_CONTROLS[idx];
        self.seeking = true;
        let _ = globals;
        std::thread::spawn(move || {
            match client.seek_game(tc.minutes, tc.increment, cancellation) {
                Ok((game_id, color)) => {
                    let _ = tx.send(LichessUpdate::SeekResult(Ok((game_id, color))));
                }
                Err(e) => {
                    let _ = tx.send(LichessUpdate::SeekResult(Err(e.to_string())));
                }
            }
        });
    }

    fn start_join(&mut self, game_code: String) {
        let Some(client) = self.client.clone() else {
            return;
        };
        let (tx, rx) = channel();
        self.receiver = Some(rx);
        self.seeking = true;
        std::thread::spawn(move || {
            let game_id = if game_code.contains("lichess.org/") {
                game_code
                    .split('/')
                    .next_back()
                    .unwrap_or(&game_code)
                    .to_string()
            } else {
                game_code
            };
            let my_id = match client.get_user_profile() {
                Ok(profile) => profile.id,
                Err(e) => {
                    let _ = tx.send(LichessUpdate::SeekResult(Err(format!(
                        "Failed to fetch profile: {}",
                        e
                    ))));
                    return;
                }
            };
            match client.join_game(&game_id, my_id) {
                Ok((game_id, color)) => {
                    let _ = tx.send(LichessUpdate::SeekResult(Ok((game_id, color))));
                }
                Err(e) => {
                    let _ = tx.send(LichessUpdate::SeekResult(Err(e.to_string())));
                }
            }
        });
    }

    fn start_puzzle(&mut self, globals: &mut Globals) -> PageAction {
        let Some(client) = &self.client else {
            self.open_prompt(PromptKind::Token, "Enter your Lichess API token:");
            return PageAction::None;
        };
        match super::puzzle::PuzzleGame::fetch(client) {
            Ok(puzzle) => PageAction::Navigate(PageRequest::PuzzleGame { puzzle }),
            Err(e) => {
                globals.show_error(format!("Failed to load puzzle: {e}"));
                PageAction::None
            }
        }
    }

    fn disconnect(&mut self, globals: &mut Globals) -> PageAction {
        self.token = None;
        self.client = None;
        self.profile = None;
        self.rating_history = None;
        self.receiver = None;
        self.cancellation = None;
        globals.set_lichess_token(None);
        globals.show_success(
            "Disconnected from Lichess successfully!\n\n Your token has been removed.\n\n You can reconnect anytime from the Lichess menu.",
        );
        PageAction::Navigate(PageRequest::Home)
    }

    fn handle_prompt(&mut self, key: KeyEvent, globals: &mut Globals) -> PageAction {
        let Some(state) = self.prompt.as_mut() else {
            return PageAction::None;
        };
        match ui::prompt::handle_key(&mut state.prompt, key) {
            PromptResult::Cancel => {
                self.prompt = None;
                PageAction::None
            }
            PromptResult::Submit => {
                let value = state.prompt.input.trim().to_string();
                match state.kind {
                    PromptKind::Token => {
                        if value.is_empty() {
                            state.prompt.error = Some("Token cannot be empty".into());
                            return PageAction::None;
                        }
                        match LichessClient::new(value.clone()).get_user_profile() {
                            Ok(profile) => {
                                self.token = Some(value.clone());
                                self.client = Some(LichessClient::new(value.clone()));
                                self.profile = Some(profile);
                                globals.set_lichess_token(Some(value));
                                self.prompt = None;
                                self.fetch_profile();
                            }
                            Err(e) => {
                                state.prompt.error = Some(format!("Invalid token: {e}"));
                            }
                        }
                    }
                    PromptKind::GameCode => {
                        self.prompt = None;
                        if !value.is_empty() {
                            self.start_join(value);
                        }
                    }
                }
                PageAction::None
            }
            PromptResult::None => PageAction::None,
        }
    }

    fn poll_updates(&mut self, globals: &mut Globals) -> PageAction {
        let update = match &self.receiver {
            Some(rx) => rx.try_recv().ok(),
            None => None,
        };
        let Some(update) = update else {
            return PageAction::None;
        };
        self.receiver = None;
        match update {
            LichessUpdate::ProfileResult(result) => match result {
                Ok(data) => {
                    let (profile, history) = *data;
                    self.profile = Some(profile);
                    self.rating_history = Some(history);
                }
                Err(e) => {
                    log::error!("Failed to fetch user profile: {e}");
                    globals
                        .show_error("An error occured when trying to fetch your lichess profile");
                }
            },
            LichessUpdate::SeekResult(result) => {
                self.seeking = false;
                if let Some(token) = self.cancellation.take() {
                    token.store(true, Ordering::Relaxed);
                }
                match result {
                    Ok((game_id, color)) => {
                        return PageAction::Navigate(PageRequest::LichessGame {
                            spec: LichessGameSpec {
                                game_id,
                                color,
                                fen: None,
                            },
                        });
                    }
                    Err(e) => {
                        log::error!("Failed to seek Lichess game: {e}");
                        globals.show_error_then(
                            format!("Failed to seek game: {e}"),
                            PageRequest::LichessMenu,
                        );
                    }
                }
            }
            LichessUpdate::OngoingGamesResult(_) => {}
        }
        PageAction::None
    }
}

impl Page for LichessMenuPage {
    fn kind(&self) -> PageKind {
        PageKind::LichessMenu
    }
    fn is_modal_active(&self) -> bool {
        self.prompt.is_some() || self.seeking
    }

    fn on_enter(&mut self, globals: &mut Globals) -> PageAction {
        if self.token.is_none() {
            self.open_prompt(PromptKind::Token, "Enter your Lichess API token:");
        } else if self.profile.is_none() {
            self.fetch_profile();
        }
        let _ = globals;
        PageAction::None
    }

    fn on_exit(&mut self, _globals: &mut Globals) {
        if let Some(token) = self.cancellation.take() {
            token.store(true, Ordering::Relaxed);
        }
        self.receiver = None;
    }

    fn handle_key(&mut self, key: KeyEvent, globals: &mut Globals) -> PageAction {
        if self.prompt.is_some() {
            return self.handle_prompt(key, globals);
        }
        if self.seeking {
            if key.code == KeyCode::Esc {
                if let Some(token) = self.cancellation.take() {
                    token.store(true, Ordering::Relaxed);
                }
                self.receiver = None;
                self.seeking = false;
                return PageAction::Navigate(PageRequest::Home);
            }
            return PageAction::None;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.cursor = if self.cursor == 0 { 4 } else { self.cursor - 1 };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.cursor = (self.cursor + 1) % 5;
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.time_control = if self.time_control >= 4 { 3 } else { 4 };
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.time_control = if self.time_control <= 3 { 4 } else { 3 };
            }
            KeyCode::PageUp => {
                self.stats_scroll = self.stats_scroll.saturating_sub(3);
            }
            KeyCode::PageDown => {
                self.stats_scroll = self.stats_scroll.saturating_add(3);
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                if self.cursor == 4 {
                    return self.disconnect(globals);
                }
                if self.client.is_none() {
                    self.open_prompt(PromptKind::Token, "Enter your Lichess API token:");
                    return PageAction::None;
                }
                return match self.cursor {
                    0 => {
                        self.start_seek(globals);
                        PageAction::None
                    }
                    1 => self.start_puzzle(globals),
                    2 => PageAction::Navigate(PageRequest::OngoingGames),
                    3 => {
                        self.open_prompt(PromptKind::GameCode, "Enter a Lichess game code or URL:");
                        PageAction::None
                    }
                    _ => PageAction::None,
                };
            }
            KeyCode::Esc | KeyCode::Char('b') => return PageAction::Navigate(PageRequest::Home),
            KeyCode::Char('?') => {
                globals.show_help();
            }
            KeyCode::Char('q') => return PageAction::Quit,
            _ => {}
        }
        PageAction::None
    }

    fn update(&mut self, globals: &mut Globals) -> PageAction {
        self.poll_updates(globals)
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, _globals: &mut Globals) {
        render_lichess_menu(
            frame,
            area,
            self.profile.as_ref(),
            self.rating_history.as_deref(),
            self,
        );
        if let Some(state) = &self.prompt {
            match state.kind {
                PromptKind::Token => render_enter_lichess_token_popup(frame, &state.prompt),
                PromptKind::GameCode => render_enter_game_code_popup(frame, &state.prompt),
            }
        }
        if self.seeking {
            render_seeking_popup(frame, area);
        }
    }
}
fn render_enter_game_code_popup(frame: &mut Frame<'_>, prompt: &Prompt) {
    let block = Block::default()
        .title("Join Lichess Game")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .padding(Padding::horizontal(1))
        .border_style(Style::default().fg(ui::constants::WHITE));
    let area = ui::components::centered_rect::centered_rect(50, 30, frame.area());

    let text = vec![
        Line::from("Enter game ID or URL:").alignment(Alignment::Center),
        Line::from(""),
        Line::from(prompt.input.as_str()),
        Line::from(""),
        Line::from(""),
        Line::from("You can paste the full URL or just the game ID."),
        Line::from("Note: You must be a participant in the game."),
        Line::from(""),
        Line::from("Press `Esc` to cancel.").alignment(Alignment::Center),
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

fn render_enter_lichess_token_popup(frame: &mut Frame<'_>, prompt: &Prompt) {
    let block = Block::default()
        .title("Enter Lichess API Token")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .padding(Padding::horizontal(1))
        .border_style(Style::default().fg(ui::constants::WHITE));
    let area = ui::components::centered_rect::centered_rect(70, 40, frame.area());

    let current_input = prompt.input.as_str();
    let masked_input = if current_input.len() > 4 {
        format!(
            "{}...{}",
            "*".repeat(current_input.len().saturating_sub(4)),
            &current_input[current_input.len().saturating_sub(4)..]
        )
    } else if !current_input.is_empty() {
        "*".repeat(current_input.len())
    } else {
        String::new()
    };

    let text = vec![
        Line::from("Enter your Lichess API token:").alignment(Alignment::Center),
        Line::from(""),
        Line::from(masked_input),
        Line::from(""),
        Line::from(""),
        Line::from("To get a token, follow the documentation:"),
        Line::from(format!(
            "Documentation: {}/Lichess/setup",
            crate::constants::DOCS_URL
        )),
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

fn render_seeking_popup(frame: &mut Frame<'_>, area: Rect) {
    let popup_area = ui::components::centered_rect::centered_rect(60, 20, area);
    let block = Block::default()
        .title("Lichess")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray));
    let paragraph = Paragraph::new("Seeking a game on Lichess...\n(Press 'Esc' to cancel)")
        .block(block)
        .alignment(Alignment::Center);
    frame.render_widget(paragraph, popup_area);
}
