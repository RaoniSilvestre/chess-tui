//! PGN viewer page: playback, game switching, footer, and result banner.

use crate::globals::Globals;
use crate::page::{Page, PageAction, PageKind, PageRequest};
use model::PgnViewer;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap},
};
use session::Game;
use ui::components::centered_rect::centered_rect;

/// PGN viewer page. Owns the loaded games, the selected game, and playback
/// state (auto-play and speed live inside each [`PgnViewer`]).
pub struct PgnViewerPage {
    games: Vec<PgnViewer>,
    game_idx: usize,
    game: Game,
}

impl PgnViewerPage {
    pub fn new(games: Vec<PgnViewer>, game_idx: usize) -> Self {
        Self {
            games,
            game_idx,
            game: Game::default(),
        }
    }

    fn current_mut(&mut self) -> Option<&mut PgnViewer> {
        self.games.get_mut(self.game_idx)
    }

    fn sync_board(&mut self) {
        let Some(viewer) = self.games.get(self.game_idx) else {
            return;
        };
        let (positions, moves) = viewer.current_snapshot();
        let current_ply = viewer.current_ply();
        let taken_pieces = self.game.runtime.taken_pieces().to_vec();
        let halfmove_clock = self.game.runtime.halfmove_clock();
        if let Err(e) = self.game.runtime.load_history(
            positions.to_vec(),
            moves.to_vec(),
            taken_pieces,
            halfmove_clock,
        ) {
            log::error!("Failed to sync PGN viewer to board: {e}");
            return;
        }
        self.game.ui.selected_square = None;
        self.game.runtime.start_playing();
        self.game.ui.hide_cursor = true;
        self.game.runtime.set_turn(if current_ply % 2 == 0 {
            shakmaty::Color::White
        } else {
            shakmaty::Color::Black
        });
    }
}

impl Page for PgnViewerPage {
    fn kind(&self) -> PageKind {
        PageKind::PgnViewer
    }

    fn on_enter(&mut self, globals: &mut Globals) -> PageAction {
        globals.theme.apply_to_game(&mut self.game);
        PageAction::None
    }

    fn handle_key(&mut self, key: KeyEvent, globals: &mut Globals) -> PageAction {
        let game_count = self.games.len();

        match key.code {
            // Quit viewer - full reset so the injected board state does not leak
            // into the next game (position history, hidden cursor, etc.).
            KeyCode::Esc | KeyCode::Char('q') => {
                self.games.clear();
                self.game_idx = 0;
                return PageAction::Navigate(PageRequest::Home);
            }

            // Next move
            KeyCode::Right | KeyCode::Char('l' | 'n' | 'N') => {
                if let Some(viewer) = self.current_mut() {
                    viewer.next();
                }
            }

            // Previous move - or dismiss end-of-game banner when it's visible
            KeyCode::Left | KeyCode::Char('h' | 'p' | 'P') => {
                if let Some(viewer) = self.current_mut() {
                    let is_h = matches!(key.code, KeyCode::Char('h' | 'H'));
                    if is_h && viewer.is_at_end() && !viewer.end_banner_dismissed() {
                        viewer.dismiss_end_banner();
                    } else {
                        viewer.prev();
                    }
                }
            }

            // Help popup
            KeyCode::Char('?') => globals.show_help(),

            // Go to start
            KeyCode::Char('g') => {
                if let Some(viewer) = self.current_mut() {
                    viewer.goto_start();
                }
            }

            // Go to end
            KeyCode::Char('G') => {
                if let Some(viewer) = self.current_mut() {
                    viewer.goto_end();
                }
            }

            // Toggle auto-play
            KeyCode::Char(' ') => {
                if let Some(viewer) = self.current_mut() {
                    viewer.toggle_auto_play();
                }
            }

            // Speed up
            KeyCode::Char('+') | KeyCode::Char('=') => {
                if let Some(viewer) = self.current_mut() {
                    viewer.speed_up();
                }
            }

            // Slow down
            KeyCode::Char('-') => {
                if let Some(viewer) = self.current_mut() {
                    viewer.speed_down();
                }
            }

            // Next game (Tab)
            KeyCode::Tab if game_count > 1 => {
                self.game_idx = (self.game_idx + 1) % game_count;
            }

            // Previous game (BackTab / Shift+Tab)
            KeyCode::BackTab if game_count > 1 => {
                if self.game_idx == 0 {
                    self.game_idx = game_count - 1;
                } else {
                    self.game_idx -= 1;
                }
            }

            _ => {}
        }

        PageAction::None
    }

    fn tick(&mut self, _globals: &mut Globals) -> PageAction {
        if let Some(viewer) = self.current_mut() {
            viewer.tick();
        }
        PageAction::None
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, globals: &mut Globals) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // board + history (reuses render_game_ui)
                Constraint::Length(4), // footer with controls
            ])
            .split(area);

        self.sync_board();
        ui::game::render_game_ui(
            frame,
            layout[0],
            &self.game.runtime,
            &mut self.game.ui,
            globals.kitty_pieces.as_mut(),
            globals.kitty_graphics_enabled,
            false,
            false,
        );
        render_footer(frame, &self.games, self.game_idx, layout[1]);
        render_end_banner(frame, &self.games, self.game_idx, layout[0]);
    }
}

fn render_footer(frame: &mut Frame, games: &[PgnViewer], game_idx: usize, area: Rect) {
    let Some(viewer) = games.get(game_idx) else {
        return;
    };

    let (current_ply, total_plies, auto_play, speed_label, title, white, black, result, game_count) = (
        viewer.current_ply(),
        viewer.total_plies(),
        viewer.auto_play(),
        viewer.speed_label(),
        viewer.title(),
        viewer.white(),
        viewer.black(),
        viewer.result(),
        games.len().max(1),
    );

    let game_number = game_idx + 1;

    let ply_str = if current_ply == 0 {
        "Start".to_string()
    } else {
        let move_n = (current_ply + 1).div_ceil(2);
        let side = if current_ply % 2 == 1 { "W" } else { "B" };
        format!("Move {}.{}", move_n, side)
    };

    let play_label = if auto_play { "Pause" } else { "Play" };

    let mut nav_spans = vec![
        Span::styled("←/→", Style::default().fg(Color::Cyan)),
        Span::raw(" or "),
        Span::styled("P/N", Style::default().fg(Color::Cyan)),
        Span::raw(" Prev/Next  "),
        Span::styled("Space", Style::default().fg(Color::Cyan)),
        Span::raw(format!(" {}  ", play_label)),
        Span::styled("+/-", Style::default().fg(Color::Cyan)),
        Span::raw(format!(" Speed:{speed_label}  ")),
        Span::styled("g/G", Style::default().fg(Color::Cyan)),
        Span::raw(" Start/End  "),
    ];
    if game_count > 1 {
        nav_spans.push(Span::styled(
            format!("Tab {}/{}  ", game_number, game_count),
            Style::default().fg(Color::Cyan),
        ));
    }
    nav_spans.extend([
        Span::styled("?", Style::default().fg(Color::Cyan)),
        Span::raw(" Help  "),
        Span::styled("Esc", Style::default().fg(Color::Cyan)),
        Span::raw(" Back"),
    ]);

    let progress = if let Some(pct) = (current_ply * 20).checked_div(total_plies) {
        format!(
            "[{}{}] {}/{}",
            "█".repeat(pct),
            "░".repeat(20 - pct),
            current_ply,
            total_plies
        )
    } else {
        "[────────────────────] 0/0".to_string()
    };

    let header_str = format!(
        "{} │ {} vs {} │ {} │  {}",
        title, white, black, result, ply_str
    );

    let footer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = footer_block.inner(area);
    frame.render_widget(footer_block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    let row1_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(30)])
        .split(rows[0]);

    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            header_str,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]))
        .alignment(Alignment::Left),
        row1_layout[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            progress,
            Style::default().fg(Color::Green),
        )]))
        .alignment(Alignment::Right),
        row1_layout[1],
    );

    frame.render_widget(
        Paragraph::new(Line::from(nav_spans)).alignment(Alignment::Center),
        rows[1],
    );
}

/// When the viewer sits on the final position, overlay a small banner with
/// the final result and termination reason. Non-modal - navigation keys
/// continue to work and the banner disappears as soon as the user steps back.
fn render_end_banner(frame: &mut Frame, games: &[PgnViewer], game_idx: usize, board_area: Rect) {
    let Some(viewer) = games.get(game_idx) else {
        return;
    };

    if !viewer.is_at_end() || viewer.end_banner_dismissed() {
        return;
    }

    let area = centered_rect(45, 28, board_area);

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Game Over",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            viewer.result_summary(),
            Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        Line::from(""),
        Line::from("Press `H` to hide this screen")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::LightBlue)),
    ];
    if !viewer.termination().is_empty() {
        lines.push(Line::from(""));
        lines.push(
            Line::from(Span::styled(
                viewer.termination().to_string(),
                Style::default().fg(Color::Gray),
            ))
            .alignment(Alignment::Center),
        );
    }
    lines.extend([
        Line::from(""),
        Line::from(Span::styled(
            "h: hide  ·  ← / P: step back",
            Style::default().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Center),
    ]);

    let block = Block::default()
        .title(" Result ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Yellow))
        .padding(Padding::horizontal(1))
        .style(Style::default().bg(Color::DarkGray));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    frame.render_widget(Clear, area);
    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::PageRequest;
    use crate::test_support::{send, test_globals};
    use model::PgnViewer;
    use ratatui::crossterm::event::KeyCode;

    fn load_page() -> (PgnViewerPage, Globals) {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sample.pgn");
        let games = PgnViewer::from_file(path.to_str().expect("utf8 path")).expect("sample pgn");
        let mut globals = test_globals();
        let mut page = PgnViewerPage::new(games, 0);
        page.on_enter(&mut globals);
        (page, globals)
    }

    #[test]
    fn help_popup_opens_in_pgn_mode() {
        let (mut page, mut globals) = load_page();
        send(&mut page, &mut globals, &[KeyCode::Char('?')]);
        assert!(matches!(
            globals.overlay,
            Some(crate::overlay::Overlay::Help)
        ));
        let _ = globals.handle_overlay_key(crate::test_support::key(KeyCode::Char('?')));
        assert!(globals.overlay.is_none());
    }

    #[test]
    fn p_and_n_navigate_prev_next() {
        let (mut page, mut globals) = load_page();
        send(
            &mut page,
            &mut globals,
            &[KeyCode::Char('N'), KeyCode::Char('N')],
        );
        assert_eq!(page.games[0].current_ply(), 2);
        send(&mut page, &mut globals, &[KeyCode::Char('P')]);
        assert_eq!(page.games[0].current_ply(), 1);
    }

    #[test]
    fn g_and_capital_g_jump_to_start_and_end() {
        let (mut page, mut globals) = load_page();
        send(&mut page, &mut globals, &[KeyCode::Char('G')]);
        assert!(page.games[0].is_at_end());
        let total = page.games[0].total_plies();

        send(&mut page, &mut globals, &[KeyCode::Char('g')]);
        assert_eq!(page.games[0].current_ply(), 0);
        assert!(total > 0);
    }

    #[test]
    fn h_dismisses_end_banner_and_step_back_restores_it() {
        let (mut page, mut globals) = load_page();
        send(&mut page, &mut globals, &[KeyCode::Char('G')]);
        assert!(page.games[0].is_at_end());

        let end_ply = page.games[0].current_ply();
        send(&mut page, &mut globals, &[KeyCode::Char('h')]);
        assert_eq!(page.games[0].current_ply(), end_ply);
        assert!(page.games[0].end_banner_dismissed());

        send(&mut page, &mut globals, &[KeyCode::Char('h')]);
        assert_eq!(page.games[0].current_ply(), end_ply - 1);
        assert!(!page.games[0].end_banner_dismissed());

        send(&mut page, &mut globals, &[KeyCode::Char('N')]);
        assert!(page.games[0].is_at_end());
        assert!(!page.games[0].end_banner_dismissed());
    }

    #[test]
    fn speed_steps_through_multipliers() {
        let (mut page, mut globals) = load_page();
        assert_eq!(page.games[0].speed_label(), "1x");

        for expected in ["1.5x", "2x", "2.5x", "3x", "4x", "4x"] {
            send(&mut page, &mut globals, &[KeyCode::Char('+')]);
            assert_eq!(page.games[0].speed_label(), expected);
        }

        send(&mut page, &mut globals, &[KeyCode::Char('-')]);
        assert_eq!(page.games[0].speed_label(), "3x");
    }

    #[test]
    fn esc_returns_home_and_clears_games() {
        let (mut page, mut globals) = load_page();
        let action = page.handle_key(crate::test_support::key(KeyCode::Esc), &mut globals);
        assert!(matches!(action, PageAction::Navigate(PageRequest::Home)));
        assert!(page.games.is_empty());
    }
}
