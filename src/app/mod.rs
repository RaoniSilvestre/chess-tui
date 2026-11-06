//! Single mutable root passed to every handler; owns all game logic, state structs, and global settings.

use crate::animations::AnimationState;
use crate::constants::{Pages, Popups};
use crate::game_logic::game::Game;
use crate::handlers::handler::{handle_mouse_events, handle_page_input};
use crate::state::bot_state::BotState;
use crate::state::game_mode_state::GameModeState;
use crate::state::lichess_state::LichessState;
use crate::state::multiplayer_state::MultiplayerState;
use crate::state::theme_state::ThemeState;
use crate::state::ui_state::UIState;
use chess_tui_app::{AppContext, Page, Router};
use chess_tui_core::ClockState;
use chess_tui_core::GameOutcome;
use chess_tui_game::GameKind;
use chess_tui_pages::{CreditsPage, HomePage};
use chess_tui_ui::graphics::KittyPieces;
use log::LevelFilter;
use std::error;
use std::time::Instant;

pub mod bot;
pub mod config;
pub mod game;
pub mod input;
pub mod lichess;
pub mod menu;
pub mod multiplayer;
pub mod resume;
pub mod run;

/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

/// Top-level application context that owns all runtime state.
pub struct App {
    /// High-resolution pieces when Kitty's graphics protocol is available.
    pub kitty_pieces: Option<KittyPieces>,
    /// Is the application running?
    pub running: bool,
    /// Game
    pub game: Game,
    /// The log level of the app
    pub log_level: LevelFilter,
    /// Whether sound effects are enabled
    pub sound_enabled: bool,
    /// Whether animations are enabled
    pub animations_enabled: bool,
    /// Whether to draw pieces as images when Kitty graphics are available
    pub kitty_graphics_enabled: bool,
    /// Everything related to the skin handling through the app
    pub theme_state: ThemeState,
    /// Bot engine state (path, depth, difficulty, move channel)
    pub bot_state: BotState,
    /// UI navigation state (current page, popup, menu cursor)
    pub ui_state: UIState,
    /// Everything related to multiplayer networking
    pub multiplayer_state: MultiplayerState,
    /// Everything related to game mode setup
    pub game_mode_state: GameModeState,
    /// Everything related to Lichess
    pub lichess_state: LichessState,
    /// PGN viewer: list of games loaded from a PGN file
    pub pgn_viewer_state: Option<Vec<chess_tui_core::PgnViewer>>,
    /// PGN viewer: which game is currently shown
    pub pgn_viewer_game_idx: usize,
    pub animations: AnimationState,
    /// Tracks the move count last persisted to disk for resume support, so the
    /// autosave hook only writes when the position has actually advanced.
    pub last_saved_move_count: Option<usize>,
    /// Wall-clock time of the last resume-state flush. Used to throttle the
    /// per-tick clock-capture write so we don't hammer the disk.
    pub last_resume_flush_at: Option<std::time::Instant>,
    /// Active page router. `None` only while a dispatch/render borrows it.
    pub router: Option<Router<App>>,
    /// Last page used to build a router page.
    pub last_page: Pages,
}

impl Default for App {
    fn default() -> Self {
        let mut app = Self {
            kitty_pieces: None,
            running: true,
            game: Game::default(),
            log_level: LevelFilter::Off,
            theme_state: ThemeState::default(),
            bot_state: BotState::default(),
            multiplayer_state: MultiplayerState::default(),
            game_mode_state: GameModeState::default(),
            lichess_state: LichessState::default(),
            ui_state: UIState::default(),
            sound_enabled: true,
            animations_enabled: false,
            kitty_graphics_enabled: true,
            pgn_viewer_state: None,
            pgn_viewer_game_idx: 0,
            animations: AnimationState::default(),
            last_saved_move_count: None,
            last_resume_flush_at: None,
            router: None,
            last_page: Pages::Home,
        };
        app.router = Some(Router::new(Box::new(HomePage::new())));
        app
    }
}

/// Compatibility page for screens that still use the old handlers/renderers.
struct LegacyPage;

impl Page<App> for LegacyPage {
    fn handle_key(
        &mut self,
        key: ratatui::crossterm::event::KeyEvent,
        app: &mut App,
    ) -> chess_tui_app::Nav<App> {
        handle_page_input(app, key);
        chess_tui_app::Nav::None
    }

    fn handle_mouse(
        &mut self,
        mouse: ratatui::crossterm::event::MouseEvent,
        app: &mut App,
    ) -> chess_tui_app::Nav<App> {
        let _ = handle_mouse_events(mouse, app);
        chess_tui_app::Nav::None
    }

    fn render(
        &mut self,
        frame: &mut ratatui::Frame<'_>,
        area: ratatui::layout::Rect,
        app: &mut App,
    ) {
        crate::ui::main_ui::render_page(frame, app, area);
    }
}

impl AppContext for App {
    fn home_cursor(&self) -> u8 {
        self.ui_state.menu_cursor
    }

    fn home_cursor_up(&mut self) {
        self.ui_state.menu_cursor_up(5);
    }

    fn home_cursor_down(&mut self) {
        self.ui_state.menu_cursor_down(5);
    }

    fn open_game_mode_menu(&mut self) {
        self.ui_state.menu_cursor = 0;
        self.menu_select();
    }

    fn open_lichess_menu(&mut self) {
        self.ui_state.menu_cursor = 1;
        self.menu_select();
    }

    fn open_settings(&mut self) {
        self.ui_state.menu_cursor = 2;
        self.menu_select();
    }

    fn open_help(&mut self) {
        self.ui_state.menu_cursor = 3;
        self.menu_select();
    }

    fn open_credits(&mut self) {
        self.ui_state.menu_cursor = 4;
        self.menu_select();
    }

    fn go_home(&mut self) {
        self.ui_state.navigate_to_homepage();
    }

    fn quit(&mut self) {
        self.quit();
    }

    fn take_page(&mut self) -> Option<Box<dyn Page<Self>>> {
        let current = self.ui_state.current_page;
        if current == self.last_page {
            return None;
        }
        self.last_page = current;
        Some(match current {
            Pages::Home => Box::new(HomePage::new()),
            Pages::Credit => Box::new(CreditsPage::new()),
            _ => Box::new(LegacyPage),
        })
    }
}

impl App {
    pub fn sync_page(&mut self) {
        if let Some(page) = self.take_page() {
            self.router = Some(Router::new(page));
        }
    }

    pub fn dispatch_key(&mut self, key: ratatui::crossterm::event::KeyEvent) {
        self.sync_page();
        if let Some(mut router) = self.router.take() {
            router.handle_key(key, self);
            self.router = Some(router);
        }
    }

    pub fn dispatch_tick(&mut self) {
        self.sync_page();
        if let Some(mut router) = self.router.take() {
            router.tick(self);
            self.router = Some(router);
        }
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&mut self) {
        self.dispatch_tick();
        // Advance PGN viewer auto-play
        if let Some(ref mut games) = self.pgn_viewer_state
            && let Some(viewer) = games.get_mut(self.pgn_viewer_game_idx)
        {
            viewer.tick();
        }

        // Update cursor blink state (used to flicker the cursor cell when a piece is selected)
        self.game.ui.update_cursor_blink();

        // Handle puzzle logic
        if let Some(mut puzzle_game) = self.lichess_state.puzzle_game.take() {
            puzzle_game.check_elo_update();

            if let Some(success_message) =
                puzzle_game.check_pending_move(&mut self.game, self.lichess_state.token.clone())
            {
                self.ui_state
                    .show_message_popup(success_message, Popups::PuzzleEndScreen);
            }

            self.lichess_state.puzzle_game = Some(puzzle_game);
        }

        let is_lichess = self.game.logic.kind() == GameKind::Lichess;

        let clock = &self.game.logic.clock();

        // Detect time running out for Lichess games (the server stops sending
        // events once a player flags, so we finish the countdown locally).
        if is_lichess
            && let ClockState::Running { active_color, .. } = self.game.logic.clock().state()
            && clock.get_time(active_color, Instant::now()).is_zero()
        {
            self.game.logic.force_time_up(active_color);
        }

        // Check clock for time up (for local games and bot games with clock)
        if let ClockState::TimeUp { loser_color } = self.game.logic.clock().state() {
            let winner = loser_color.other();
            if !self.game.logic.state().is_finished() {
                self.game.logic.finish(GameOutcome::Timeout { winner });
            }
            self.check_and_show_game_end();
        }
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        // Capture the clock's current value (which keeps ticking between
        // moves) so a resumed game reflects real elapsed time rather than
        // freezing at the last move's timestamp.
        self.flush_resume_state();

        // Cancel any active Lichess seek before quitting
        if let Some(cancellation_token) = &self.lichess_state.cancellation_token {
            cancellation_token.store(true, std::sync::atomic::Ordering::Relaxed);
            log::info!("Cancelling Lichess seek before quit");
        }
        self.running = false;
    }
}
