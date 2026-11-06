//! Single mutable root passed to every handler; owns all game logic, state structs, and global settings.

use crate::animations::AnimationState;
use crate::constants::{Pages, Popups};
use crate::game_logic::game::Game;
use crate::state::bot_state::BotState;
use crate::state::lichess_state::LichessState;
use crate::state::multiplayer_state::MultiplayerState;
use crate::state::resume::ResumeMode;
use crate::state::theme_state::ThemeState;
use crate::state::ui_state::UIState;
use chess_tui_app::GameModeState;
use chess_tui_app::{
    AppContext, GameModeContext, GamePageContext, GamePageKind, LichessMenuContext,
    OngoingGamesContext, Page, PgnViewerContext, Router, SettingsContext,
};
use chess_tui_core::GameOutcome;
use chess_tui_core::{Clock, ClockState};
use chess_tui_game::GameKind;
use chess_tui_pages::{
    CreditsPage, GameModeMenuPage, GamePage, HomePage, LichessMenuPage, OngoingGamesPage,
    PgnViewerPage, SettingsPage,
};
use chess_tui_theme::DisplayMode;
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

impl AppContext for App {
    fn open_game_mode_menu(&mut self) {
        self.ui_state.menu_cursor = 0;
        self.game_mode_state.selection = Some(chess_tui_app::AvailableGameMode::Local);
        self.game_mode_state.form_cursor = 0;
        self.game_mode_state.form_active = false;
        self.ui_state.current_page = Pages::GameModeMenu;
    }

    fn open_lichess_menu(&mut self) {
        if !self.lichess_has_token() {
            self.lichess_open_token_prompt();
            return;
        }
        self.ui_state.menu_cursor = 0;
        self.ui_state.current_page = Pages::LichessMenu;
        if self.lichess_state.user_profile.is_none() {
            self.ui_state.popup_message = Some("Loading your lichess account ...".to_string());
            self.ui_state.show_popup(Popups::Loading);
        }
        self.fetch_lichess_user_profile();
    }

    fn open_settings(&mut self) {
        self.ui_state.menu_cursor = 0;
        self.ui_state.current_page = Pages::SettingsMenu;
    }

    fn open_help(&mut self) {
        self.ui_state.toggle_help_popup();
    }

    fn open_credits(&mut self) {
        self.ui_state.current_page = Pages::Credit;
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
            Pages::SettingsMenu => Box::new(SettingsPage::new()),
            Pages::GameModeMenu => Box::new(GameModeMenuPage::new()),
            Pages::LichessMenu => Box::new(LichessMenuPage::new(self.game_mode_state.clock_cursor)),
            Pages::OngoingGames => Box::new(OngoingGamesPage::new()),
            Pages::PgnViewer => Box::new(PgnViewerPage::new(
                self.pgn_viewer_state.take().unwrap_or_default(),
                self.pgn_viewer_game_idx,
            )),
            Pages::Solo => Box::new(GamePage::new(GamePageKind::Solo)),
            Pages::Bot => Box::new(GamePage::new(GamePageKind::Bot)),
            Pages::Multiplayer => Box::new(GamePage::new(GamePageKind::Multiplayer)),
            Pages::Lichess => Box::new(GamePage::new(GamePageKind::Lichess)),
        })
    }
}

impl SettingsContext for App {
    fn settings_has_sound(&self) -> bool {
        cfg!(feature = "sound")
    }

    fn settings_kitty_available(&self) -> bool {
        self.kitty_pieces.is_some()
    }

    fn settings_skin_label(&self) -> String {
        let skin_name = match self.game.ui.display_mode {
            DisplayMode::DEFAULT => "Default",
            DisplayMode::ASCII => "ASCII",
            DisplayMode::CUSTOM => self.game.ui.skin.name.as_str(),
        };
        format!("Skin: {skin_name}")
    }

    fn settings_sound_label(&self) -> String {
        let status = if self.sound_enabled {
            "On 🔊"
        } else {
            "Off 🔇"
        };
        format!("Sound: {status}")
    }

    fn settings_animations_label(&self) -> String {
        let status = if self.animations_enabled { "On" } else { "Off" };
        format!("Animations: {status}")
    }

    fn settings_bot_depth_label(&self) -> String {
        format!("Bot Depth: -  {}  +", self.bot_state.bot_depth)
    }

    fn settings_bot_difficulty_label(&self) -> String {
        let difficulty = match self.bot_state.bot_difficulty {
            Some(0) => "Easy",
            Some(1) => "Medium",
            Some(2) => "Hard",
            Some(3) => "Magnus",
            _ => "Off",
        };
        format!("Bot Difficulty: {difficulty}")
    }

    fn settings_kitty_label(&self) -> String {
        let status = if self.kitty_graphics_enabled {
            "On"
        } else {
            "Off"
        };
        format!("Kitty Graphics: {status}")
    }

    fn settings_cycle_skin(&mut self, next: bool) {
        self.cycle_skin(next);
        self.update_config_from_app();
    }

    fn settings_toggle_sound(&mut self) {
        #[cfg(feature = "sound")]
        {
            self.sound_enabled = !self.sound_enabled;
            crate::sound::set_sound_enabled(self.sound_enabled);
            self.update_config_from_app();
        }
    }

    fn settings_toggle_animations(&mut self) {
        self.animations_enabled = !self.animations_enabled;
        self.update_config_from_app();
    }

    fn settings_open_engine_path(&mut self) {
        self.ui_state.current_popup = Some(Popups::EnterEnginePath);
        self.game.ui.prompt.input = self.bot_state.chess_engine_path.clone().unwrap_or_default();
        self.game.ui.prompt.character_index = self.game.ui.prompt.input.chars().count();
    }

    fn settings_bot_depth_down(&mut self) {
        if self.bot_state.bot_depth > 1 {
            self.bot_state.bot_depth -= 1;
            self.update_config_from_app();
        }
    }

    fn settings_bot_depth_up(&mut self) {
        if self.bot_state.bot_depth < 20 {
            self.bot_state.bot_depth += 1;
            self.update_config_from_app();
        }
    }

    fn settings_cycle_difficulty(&mut self, next: bool) {
        self.cycle_bot_difficulty(next);
    }

    fn settings_toggle_kitty(&mut self) {
        self.kitty_graphics_enabled = !self.kitty_graphics_enabled;
        self.update_config_from_app();
    }
}

impl GameModeContext for App {
    fn game_mode_menu_cursor(&self) -> u8 {
        self.ui_state.menu_cursor
    }

    fn game_mode_set_menu_cursor(&mut self, cursor: u8) {
        self.ui_state.menu_cursor = cursor;
    }

    fn game_mode_state(&self) -> &chess_tui_app::GameModeState {
        &self.game_mode_state
    }

    fn game_mode_state_mut(&mut self) -> &mut chess_tui_app::GameModeState {
        &mut self.game_mode_state
    }

    fn game_mode_bot_depth(&self) -> u8 {
        self.bot_state.bot_depth
    }

    fn game_mode_set_bot_depth(&mut self, depth: u8) {
        self.bot_state.bot_depth = depth;
        self.update_config_from_app();
    }

    fn game_mode_bot_difficulty(&self) -> Option<u8> {
        self.bot_state.bot_difficulty
    }

    fn game_mode_cycle_difficulty(&mut self, next: bool) {
        self.cycle_bot_difficulty(next);
    }

    fn game_mode_engine_path(&self) -> Option<String> {
        self.bot_state.chess_engine_path.clone()
    }

    fn game_mode_hosting(&self) -> Option<bool> {
        self.multiplayer_state.hosting
    }

    fn game_mode_set_hosting(&mut self, hosting: Option<bool>) {
        self.multiplayer_state.hosting = hosting;
    }

    fn game_mode_has_resume(&self, kind: chess_tui_app::AvailableGameMode) -> bool {
        match kind {
            chess_tui_app::AvailableGameMode::Local => self.has_resume_save(ResumeMode::Local),
            chess_tui_app::AvailableGameMode::Bot => self.has_resume_save(ResumeMode::Bot),
            _ => false,
        }
    }

    fn game_mode_resume(&mut self, kind: chess_tui_app::AvailableGameMode) {
        match kind {
            chess_tui_app::AvailableGameMode::Local => {
                self.resume_from_saved(ResumeMode::Local);
            }
            chess_tui_app::AvailableGameMode::Bot => {
                self.resume_from_saved(ResumeMode::Bot);
            }
            _ => {}
        }
    }

    fn game_mode_submit(&mut self, kind: chess_tui_app::AvailableGameMode) {
        match kind {
            chess_tui_app::AvailableGameMode::Local => {
                self.game
                    .runtime
                    .set_clock(Clock::new(self.game_mode_state.get_time_control()));
                crate::handlers::game_mode_menu::finish_form(self, Pages::Solo);
            }
            chess_tui_app::AvailableGameMode::Bot => {
                crate::handlers::game_mode_menu::finish_form(self, Pages::Bot);
            }
            chess_tui_app::AvailableGameMode::Multiplayer => {
                crate::handlers::game_mode_menu::finish_form(self, Pages::Multiplayer);
            }
            chess_tui_app::AvailableGameMode::PGNLoader => {}
        }
    }

    fn game_mode_open_pgn_loader(&mut self) {
        self.ui_state.current_popup = Some(Popups::LoadPgnPath);
        self.game.ui.prompt.reset();
    }
}

impl LichessMenuContext for App {
    fn lichess_has_token(&self) -> bool {
        self.lichess_state
            .token
            .as_ref()
            .map(|token| !token.is_empty())
            .unwrap_or(false)
    }

    fn lichess_open_token_prompt(&mut self) {
        self.ui_state.current_popup = Some(Popups::EnterLichessToken);
        self.game.ui.prompt.reset();
        self.game.ui.prompt.message = "Enter your Lichess API token:".to_string();
    }

    fn lichess_profile(&self) -> Option<&chess_tui_lichess::UserProfile> {
        self.lichess_state.user_profile.as_ref()
    }

    fn lichess_rating_history(&self) -> Option<&[chess_tui_lichess::RatingHistoryEntry]> {
        self.lichess_state.rating_history.as_deref()
    }

    fn lichess_time_control(&self) -> u8 {
        self.game_mode_state.clock_cursor
    }

    fn lichess_set_time_control(&mut self, cursor: u8) {
        self.game_mode_state.clock_cursor = cursor;
    }

    fn lichess_seek(&mut self, cursor: u8) {
        self.ui_state.menu_cursor = 0;
        self.ui_state.current_page = Pages::Lichess;
        self.create_lichess_opponent(cursor.clamp(3, 4));
    }

    fn lichess_start_puzzle(&mut self) {
        self.start_puzzle_mode();
    }

    fn lichess_open_ongoing_games(&mut self) {
        if self.lichess_state.ongoing_games.is_empty() {
            self.ui_state.show_message_popup(
                "Loading your ongoing games ...".to_string(),
                Popups::Loading,
            );
        }

        self.fetch_ongoing_games();
        self.ui_state.current_page = Pages::OngoingGames;
        self.ui_state.menu_cursor = 0;
    }

    fn lichess_open_join_code(&mut self) {
        self.ui_state.current_popup = Some(Popups::EnterGameCode);
        self.game.ui.prompt.reset();
    }

    fn lichess_disconnect(&mut self) {
        self.disconnect_lichess();
    }
}

impl OngoingGamesContext for App {
    fn ongoing_games(&self) -> &[chess_tui_lichess::OngoingGame] {
        &self.lichess_state.ongoing_games
    }

    fn ongoing_game_open(&mut self, index: usize) {
        self.ui_state.menu_cursor = index as u8;
        self.select_ongoing_game();
    }

    fn ongoing_game_request_resign(&mut self, index: usize) {
        self.ui_state.menu_cursor = index as u8;
        self.show_resign_confirmation();
    }

    fn ongoing_games_back(&mut self) {
        self.ui_state.menu_cursor = 0;
        self.ui_state.current_page = Pages::LichessMenu;
    }
}

impl PgnViewerContext for App {
    fn pgn_mirror(&mut self, games: &[chess_tui_core::PgnViewer], game_idx: usize) {
        self.pgn_viewer_state = Some(games.to_vec());
        self.pgn_viewer_game_idx = game_idx;
    }

    fn pgn_sync_board(&mut self, games: &[chess_tui_core::PgnViewer], game_idx: usize) {
        crate::ui::pgn_viewer_ui::sync_viewer_to_board(self, games, game_idx);
    }

    fn pgn_render_board(&mut self, frame: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect) {
        crate::ui::game_ui::render_game_ui(frame, self, area);
    }

    fn pgn_exit(&mut self) {
        self.pgn_viewer_state = None;
        self.pgn_viewer_game_idx = 0;
        self.reset_home();
    }
}

impl GamePageContext for App {
    fn game_take(&mut self) -> chess_tui_app::game::Game {
        std::mem::take(&mut self.game)
    }

    fn game_put(&mut self, game: chess_tui_app::game::Game) {
        self.game = game;
    }

    fn game_is_puzzle(&self) -> bool {
        self.lichess_state.puzzle_game.is_some()
    }

    fn game_selected_color(&self) -> Option<shakmaty::Color> {
        self.game_mode_state.selected_color
    }

    fn game_is_lichess(&self) -> bool {
        self.game.runtime.kind() == GameKind::Lichess
    }

    fn game_validate_promotion(
        &mut self,
        game: &mut chess_tui_app::game::Game,
        move_uci: String,
    ) -> bool {
        let placeholder = std::mem::take(&mut self.game);
        self.game = std::mem::take(game);
        let result = self.validate_puzzle_move_uci(move_uci);
        *game = std::mem::take(&mut self.game);
        self.game = placeholder;
        result
    }

    fn game_validate_move(
        &mut self,
        game: &mut chess_tui_app::game::Game,
        from: shakmaty::Square,
        to: shakmaty::Square,
    ) {
        let placeholder = std::mem::take(&mut self.game);
        self.game = std::mem::take(game);
        self.validate_puzzle_move_after_execution(from, to);
        *game = std::mem::take(&mut self.game);
        self.game = placeholder;
    }

    fn game_check_end(&mut self, game: &mut chess_tui_app::game::Game) {
        let placeholder = std::mem::take(&mut self.game);
        self.game = std::mem::take(game);
        self.check_and_show_game_end();
        *game = std::mem::take(&mut self.game);
        self.game = placeholder;
    }

    fn game_puzzle_hint(&mut self, game: &mut chess_tui_app::game::Game) {
        let placeholder = std::mem::take(&mut self.game);
        self.game = std::mem::take(game);
        if let Some(puzzle_game) = &self.lichess_state.puzzle_game {
            puzzle_game.show_hint(&mut self.game);
        }
        *game = std::mem::take(&mut self.game);
        self.game = placeholder;
    }

    fn game_toggle_end_screen(&mut self) {
        if self.ui_state.current_popup == Some(Popups::EndScreen) {
            self.ui_state.close_popup();
            self.ui_state.end_screen_dismissed = true;
        } else if self.ui_state.end_screen_dismissed {
            self.ui_state.end_screen_dismissed = false;
            self.show_end_screen();
        } else {
            self.show_end_screen();
        }
    }

    fn game_request_restart(&mut self) {
        self.request_restart();
    }

    fn game_reset_home(&mut self) {
        self.reset_home();
    }

    fn game_open_move_input(&mut self) {
        self.game.ui.prompt.reset();
        self.ui_state.current_popup = Some(Popups::MoveInputSelection);
    }

    fn game_cycle_skin(&mut self, next: bool) {
        self.cycle_skin(next);
    }

    fn game_prepare(&mut self, kind: GamePageKind) -> bool {
        match kind {
            GamePageKind::Bot => self.prepare_bot_page(),
            GamePageKind::Multiplayer => self.prepare_multiplayer_page(),
            GamePageKind::Solo | GamePageKind::Lichess => true,
        }
    }

    fn game_render(&mut self, frame: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect) {
        crate::ui::game_ui::render_game_ui(frame, self, area);
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

    pub fn dispatch_mouse(&mut self, mouse: ratatui::crossterm::event::MouseEvent) {
        self.sync_page();
        if let Some(mut router) = self.router.take() {
            router.handle_mouse(mouse, self);
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

        let is_lichess = self.game.runtime.kind() == GameKind::Lichess;

        let clock = &self.game.runtime.clock();

        // Detect time running out for Lichess games (the server stops sending
        // events once a player flags, so we finish the countdown locally).
        if is_lichess
            && let ClockState::Running { active_color, .. } = self.game.runtime.clock().state()
            && clock.get_time(active_color, Instant::now()).is_zero()
        {
            self.game.runtime.force_time_up(active_color);
        }

        // Check clock for time up (for local games and bot games with clock)
        if let ClockState::TimeUp { loser_color } = self.game.runtime.clock().state() {
            let winner = loser_color.other();
            if !self.game.runtime.state().is_finished() {
                self.game.runtime.finish(GameOutcome::Timeout { winner });
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
