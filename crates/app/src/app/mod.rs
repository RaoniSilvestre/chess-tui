//! Top-level application state and services.

use crate::animations::AnimationState;
use crate::game_mode::{AvailableGameMode, GameModeState};
use crate::game_mode_menu::finish_form;
use crate::game_state::Game;
use crate::state::bot_state::BotState;
use crate::state::lichess_state::LichessState;
use crate::state::multiplayer_state::MultiplayerState;
use crate::state::resume::ResumeMode;
use crate::state::theme_state::ThemeState;
use crate::state::ui_state::UIState;
use crate::{GamePageKind, Modal, ModalResult, NoopPageFactory, Page, PageFactory, Router};
use ::game::GameKind;
use log::LevelFilter;
use model::GameOutcome;
use model::{Clock, ClockState};
use std::error;
use std::time::Instant;
use theme::DisplayMode;
use ui::graphics::KittyPieces;

pub mod bot;
pub mod config;
pub mod game;
pub mod input;
pub mod lichess;
pub mod menu;
pub mod multiplayer;
pub mod resume;

/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

/// Top-level application context that owns all runtime state.
pub struct App {
    pub kitty_pieces: Option<KittyPieces>,
    pub running: bool,
    pub game: Game,
    pub log_level: LevelFilter,
    pub sound_enabled: bool,
    pub animations_enabled: bool,
    pub kitty_graphics_enabled: bool,
    pub theme_state: ThemeState,
    pub bot_state: BotState,
    pub ui_state: UIState,
    pub multiplayer_state: MultiplayerState,
    pub game_mode_state: GameModeState,
    pub lichess_state: LichessState,
    pub pgn_viewer_state: Option<Vec<model::PgnViewer>>,
    pub pgn_viewer_game_idx: usize,
    pub animations: AnimationState,
    pub last_saved_move_count: Option<usize>,
    pub last_resume_flush_at: Option<Instant>,
    pub router: Option<Router>,
    pub pages: Box<dyn PageFactory>,
    pending_page: Option<Box<dyn Page>>,
    active_game_kind: Option<GamePageKind>,
    modal: Option<Box<dyn Modal>>,
}

impl Default for App {
    fn default() -> Self {
        Self::new(Box::new(NoopPageFactory))
    }
}

impl App {
    pub fn new(pages: Box<dyn PageFactory>) -> Self {
        let router = Router::new(pages.home());
        Self {
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
            router: Some(router),
            pages,
            pending_page: None,
            active_game_kind: None,
            modal: None,
        }
    }

    pub fn show_solo(&mut self) {
        let page = self.pages.solo();
        self.set_page(page);
    }

    pub fn show_bot(&mut self) {
        let page = self.pages.bot();
        self.set_page(page);
    }

    pub fn show_multiplayer(&mut self) {
        let page = self.pages.multiplayer();
        self.set_page(page);
    }

    pub fn show_pgn_viewer(&mut self, games: Vec<model::PgnViewer>, game_idx: usize) {
        self.pgn_viewer_state = Some(games.clone());
        self.pgn_viewer_game_idx = game_idx;
        let page = self.pages.pgn_viewer(games, game_idx);
        self.set_page(page);
    }

    pub fn show_lichess(&mut self) {
        let page = self.pages.lichess();
        self.set_page(page);
    }

    pub fn open_modal(&mut self, modal: Box<dyn Modal>) {
        self.modal = Some(modal);
    }

    pub fn close_modal(&mut self) {
        self.modal = None;
        self.ui_state.end_screen_visible = false;
    }

    pub fn has_any_modal(&self) -> bool {
        self.modal.is_some() || self.ui_state.end_screen_visible
    }

    pub fn has_modal<T: Modal + 'static>(&self) -> bool {
        self.modal
            .as_ref()
            .is_some_and(|modal| modal.as_any().type_id() == std::any::TypeId::of::<T>())
    }

    pub fn show_help(&mut self) {
        self.open_modal(Box::new(crate::modal::HelpModal));
    }

    pub fn toggle_help(&mut self) {
        if self.has_modal::<crate::modal::HelpModal>() {
            self.close_modal();
        } else {
            self.show_help();
        }
    }

    pub fn show_error(&mut self, message: String) {
        self.open_modal(Box::new(crate::modal::MessageModal::error(message)));
    }

    pub fn show_success(&mut self, message: String) {
        self.open_modal(Box::new(crate::modal::MessageModal::success(message)));
    }

    pub fn show_loading(&mut self, message: String) {
        self.open_modal(Box::new(crate::modal::MessageModal::loading(message)));
    }

    pub fn show_enter_host_ip(&mut self) {
        self.game.ui.prompt.reset();
        self.open_modal(Box::new(crate::modal::PromptModal::enter_host_ip()));
    }

    pub fn show_enter_game_code(&mut self) {
        self.game.ui.prompt.reset();
        self.open_modal(Box::new(crate::modal::PromptModal::enter_game_code()));
    }

    pub fn show_enter_lichess_token(&mut self) {
        self.game.ui.prompt.reset();
        self.game.ui.prompt.message = "Enter your Lichess API token:".to_string();
        self.open_modal(Box::new(crate::modal::PromptModal::enter_lichess_token()));
    }

    pub fn show_load_pgn_path(&mut self) {
        self.game.ui.prompt.reset();
        self.open_modal(Box::new(crate::modal::PromptModal::load_pgn_path()));
    }

    pub fn show_enter_engine_path(&mut self) {
        self.game.ui.prompt.input = self.bot_state.chess_engine_path.clone().unwrap_or_default();
        self.game.ui.prompt.character_index = self.game.ui.prompt.input.chars().count();
        self.open_modal(Box::new(crate::modal::PromptModal::enter_engine_path()));
    }

    pub fn show_move_input(&mut self) {
        self.game.ui.prompt.reset();
        self.open_modal(Box::new(crate::modal::PromptModal::move_input()));
    }

    pub fn show_seeking_lichess_game(&mut self) {
        self.open_modal(Box::new(crate::modal::SeekingLichessModal));
    }

    pub fn show_waiting_for_opponent(&mut self) {
        self.open_modal(Box::new(crate::modal::WaitingForOpponentModal));
    }

    pub fn show_resign_confirmation(&mut self) {
        self.open_modal(Box::new(crate::modal::ResignConfirmationModal));
    }

    pub fn show_new_game_confirmation(&mut self) {
        self.open_modal(Box::new(crate::modal::NewGameConfirmationModal));
    }

    pub fn show_puzzle_end(&mut self, message: String) {
        self.open_modal(Box::new(crate::modal::PuzzleEndModal::new(message)));
    }

    pub fn after_error_popup(&mut self) {
        if self.current_game_kind() == Some(GamePageKind::Lichess)
            || self
                .router
                .as_ref()
                .is_some_and(|router| router.page().is_ongoing_games())
        {
            self.show_lichess_menu();
        } else if matches!(
            self.current_game_kind(),
            Some(GamePageKind::Multiplayer | GamePageKind::Bot)
        ) {
            self.show_home();
        }
    }

    pub fn after_success_popup(&mut self) {
        if self.current_game_kind() == Some(GamePageKind::Lichess) {
            self.show_lichess_menu();
        } else if self
            .router
            .as_ref()
            .is_some_and(|router| router.page().is_ongoing_games())
        {
            self.fetch_ongoing_games();
            self.show_ongoing_games();
        } else if matches!(
            self.current_game_kind(),
            Some(GamePageKind::Multiplayer | GamePageKind::Bot)
        ) {
            self.show_home();
        }
    }

    pub fn render_modal(&mut self, frame: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect) {
        if let Some(mut modal) = self.modal.take() {
            modal.render(frame, area, self);
            self.modal = Some(modal);
        }
    }

    pub fn handle_modal_key(&mut self, key: ratatui::crossterm::event::KeyEvent) -> bool {
        let Some(mut modal) = self.modal.take() else {
            return false;
        };
        let result = modal.handle_key(key, self);
        match result {
            ModalResult::None => self.modal = Some(modal),
            ModalResult::Close => {}
            ModalResult::Replace(next) => self.modal = Some(next),
        }
        true
    }

    pub fn set_page(&mut self, page: Box<dyn Page>) {
        self.active_game_kind = page.game_kind();
        if self.router.is_some() {
            self.router = Some(Router::new(page));
        } else {
            self.pending_page = Some(page);
        }
    }

    pub fn take_pending_page(&mut self) -> Option<Box<dyn Page>> {
        self.pending_page.take()
    }

    pub fn sync_page(&mut self) {
        if let Some(page) = self.pending_page.take() {
            self.router = Some(Router::new(page));
        }
    }

    pub fn is_page<T: Page + 'static>(&self) -> bool {
        self.router
            .as_ref()
            .is_some_and(|router| router.page().as_any().is::<T>())
    }

    pub fn current_game_kind(&self) -> Option<GamePageKind> {
        self.router
            .as_ref()
            .and_then(|router| router.page().game_kind())
            .or(self.active_game_kind)
    }

    pub fn current_page_type_id(&self) -> Option<std::any::TypeId> {
        self.router
            .as_ref()
            .map(|router| router.page().as_any().type_id())
    }

    pub fn is_home(&self) -> bool {
        self.router
            .as_ref()
            .is_some_and(|router| router.page().is_home())
    }

    pub fn is_pgn_viewer(&self) -> bool {
        self.router
            .as_ref()
            .is_some_and(|router| router.page().is_pgn_viewer())
    }

    pub fn dispatch_key(&mut self, key: ratatui::crossterm::event::KeyEvent) {
        if self.handle_modal_key(key) {
            return;
        }
        self.sync_page();
        if let Some(mut router) = self.router.take() {
            router.handle_key(key, self);
            self.router = Some(router);
        }
    }

    pub fn dispatch_mouse(&mut self, mouse: ratatui::crossterm::event::MouseEvent) {
        if self.modal.is_some() || self.ui_state.end_screen_visible {
            return;
        }
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

    pub fn open_game_mode_menu(&mut self) {
        self.ui_state.menu_cursor = 0;
        self.game_mode_state.selection = Some(AvailableGameMode::Local);
        self.game_mode_state.form_cursor = 0;
        self.game_mode_state.form_active = false;
        let page = self.pages.game_mode_menu();
        self.set_page(page);
    }

    pub fn show_home(&mut self) {
        self.ui_state.menu_cursor = 0;
        let page = self.pages.home();
        self.set_page(page);
    }

    pub fn show_lichess_menu(&mut self) {
        self.ui_state.menu_cursor = 0;
        let page = self.pages.lichess_menu(self.game_mode_state.clock_cursor);
        self.set_page(page);
    }

    pub fn show_ongoing_games(&mut self) {
        self.ui_state.menu_cursor = 0;
        let page = self.pages.ongoing_games();
        self.set_page(page);
    }

    pub fn open_lichess_menu(&mut self) {
        if !self.lichess_has_token() {
            self.lichess_open_token_prompt();
            return;
        }
        self.show_lichess_menu();
        if self.lichess_state.user_profile.is_none() {
            self.show_loading("Loading your lichess account ...".to_string());
        }
        self.fetch_lichess_user_profile();
    }

    pub fn open_settings(&mut self) {
        self.ui_state.menu_cursor = 0;
        let page = self.pages.settings();
        self.set_page(page);
    }

    pub fn open_help(&mut self) {
        self.toggle_help();
    }

    pub fn open_credits(&mut self) {
        let page = self.pages.credits();
        self.set_page(page);
    }

    pub fn go_home(&mut self) {
        self.show_home();
    }

    pub fn settings_has_sound(&self) -> bool {
        cfg!(feature = "sound")
    }

    pub fn settings_kitty_available(&self) -> bool {
        self.kitty_pieces.is_some()
    }

    pub fn settings_skin_label(&self) -> String {
        let skin_name = match self.game.ui.display_mode {
            DisplayMode::DEFAULT => "Default",
            DisplayMode::ASCII => "ASCII",
            DisplayMode::CUSTOM => self.game.ui.skin.name.as_str(),
        };
        format!("Skin: {skin_name}")
    }

    pub fn settings_sound_label(&self) -> String {
        let status = if self.sound_enabled {
            "On 🔊"
        } else {
            "Off 🔇"
        };
        format!("Sound: {status}")
    }

    pub fn settings_animations_label(&self) -> String {
        let status = if self.animations_enabled { "On" } else { "Off" };
        format!("Animations: {status}")
    }

    pub fn settings_bot_depth_label(&self) -> String {
        format!("Bot Depth: -  {}  +", self.bot_state.bot_depth)
    }

    pub fn settings_bot_difficulty_label(&self) -> String {
        let difficulty = match self.bot_state.bot_difficulty {
            Some(0) => "Easy",
            Some(1) => "Medium",
            Some(2) => "Hard",
            Some(3) => "Magnus",
            _ => "Off",
        };
        format!("Bot Difficulty: {difficulty}")
    }

    pub fn settings_kitty_label(&self) -> String {
        let status = if self.kitty_graphics_enabled {
            "On"
        } else {
            "Off"
        };
        format!("Kitty Graphics: {status}")
    }

    pub fn settings_cycle_skin(&mut self, next: bool) {
        self.cycle_skin(next);
        self.update_config_from_app();
    }

    pub fn settings_toggle_sound(&mut self) {
        #[cfg(feature = "sound")]
        {
            self.sound_enabled = !self.sound_enabled;
            ui::sound::set_sound_enabled(self.sound_enabled);
            self.update_config_from_app();
        }
    }

    pub fn settings_toggle_animations(&mut self) {
        self.animations_enabled = !self.animations_enabled;
        self.update_config_from_app();
    }

    pub fn settings_open_engine_path(&mut self) {
        self.show_enter_engine_path();
        self.game.ui.prompt.input = self.bot_state.chess_engine_path.clone().unwrap_or_default();
        self.game.ui.prompt.character_index = self.game.ui.prompt.input.chars().count();
    }

    pub fn settings_bot_depth_down(&mut self) {
        if self.bot_state.bot_depth > 1 {
            self.bot_state.bot_depth -= 1;
            self.update_config_from_app();
        }
    }

    pub fn settings_bot_depth_up(&mut self) {
        if self.bot_state.bot_depth < 20 {
            self.bot_state.bot_depth += 1;
            self.update_config_from_app();
        }
    }

    pub fn settings_cycle_difficulty(&mut self, next: bool) {
        self.cycle_bot_difficulty(next);
    }

    pub fn settings_toggle_kitty(&mut self) {
        self.kitty_graphics_enabled = !self.kitty_graphics_enabled;
        self.update_config_from_app();
    }

    pub fn game_mode_menu_cursor(&self) -> u8 {
        self.ui_state.menu_cursor
    }

    pub fn game_mode_set_menu_cursor(&mut self, cursor: u8) {
        self.ui_state.menu_cursor = cursor;
    }

    pub fn game_mode_state(&self) -> &GameModeState {
        &self.game_mode_state
    }

    pub fn game_mode_state_mut(&mut self) -> &mut GameModeState {
        &mut self.game_mode_state
    }

    pub fn game_mode_bot_depth(&self) -> u8 {
        self.bot_state.bot_depth
    }

    pub fn game_mode_set_bot_depth(&mut self, depth: u8) {
        self.bot_state.bot_depth = depth;
        self.update_config_from_app();
    }

    pub fn game_mode_bot_difficulty(&self) -> Option<u8> {
        self.bot_state.bot_difficulty
    }

    pub fn game_mode_cycle_difficulty(&mut self, next: bool) {
        self.cycle_bot_difficulty(next);
    }

    pub fn game_mode_engine_path(&self) -> Option<String> {
        self.bot_state.chess_engine_path.clone()
    }

    pub fn game_mode_hosting(&self) -> Option<bool> {
        self.multiplayer_state.hosting
    }

    pub fn game_mode_set_hosting(&mut self, hosting: Option<bool>) {
        self.multiplayer_state.hosting = hosting;
    }

    pub fn game_mode_has_resume(&self, kind: AvailableGameMode) -> bool {
        match kind {
            AvailableGameMode::Local => self.has_resume_save(ResumeMode::Local),
            AvailableGameMode::Bot => self.has_resume_save(ResumeMode::Bot),
            _ => false,
        }
    }

    pub fn game_mode_resume(&mut self, kind: AvailableGameMode) {
        match kind {
            AvailableGameMode::Local => {
                self.resume_from_saved(ResumeMode::Local);
            }
            AvailableGameMode::Bot => {
                self.resume_from_saved(ResumeMode::Bot);
            }
            _ => {}
        }
    }

    pub fn game_mode_submit(&mut self, kind: AvailableGameMode) {
        match kind {
            AvailableGameMode::Local => {
                self.game
                    .runtime
                    .set_clock(Clock::new(self.game_mode_state.get_time_control()));
                finish_form(self, GamePageKind::Solo);
            }
            AvailableGameMode::Bot => finish_form(self, GamePageKind::Bot),
            AvailableGameMode::Multiplayer => finish_form(self, GamePageKind::Multiplayer),
            AvailableGameMode::PGNLoader => {}
        }
    }

    pub fn game_mode_open_pgn_loader(&mut self) {
        self.show_load_pgn_path();
        self.game.ui.prompt.reset();
    }

    pub fn lichess_has_token(&self) -> bool {
        self.lichess_state
            .token
            .as_ref()
            .map(|token| !token.is_empty())
            .unwrap_or(false)
    }

    pub fn lichess_open_token_prompt(&mut self) {
        self.show_enter_lichess_token();
        self.game.ui.prompt.reset();
        self.game.ui.prompt.message = "Enter your Lichess API token:".to_string();
    }

    pub fn lichess_profile(&self) -> Option<&::lichess::UserProfile> {
        self.lichess_state.user_profile.as_ref()
    }

    pub fn lichess_rating_history(&self) -> Option<&[::lichess::RatingHistoryEntry]> {
        self.lichess_state.rating_history.as_deref()
    }

    pub fn lichess_time_control(&self) -> u8 {
        self.game_mode_state.clock_cursor
    }

    pub fn lichess_set_time_control(&mut self, cursor: u8) {
        self.game_mode_state.clock_cursor = cursor;
    }

    pub fn lichess_seek(&mut self, cursor: u8) {
        self.ui_state.menu_cursor = 0;
        let page = self.pages.lichess();
        self.set_page(page);
        self.create_lichess_opponent(cursor.clamp(3, 4));
    }

    pub fn lichess_start_puzzle(&mut self) {
        self.start_puzzle_mode();
    }

    pub fn lichess_open_ongoing_games(&mut self) {
        if self.lichess_state.ongoing_games.is_empty() {
            self.show_loading("Loading your ongoing games ...".to_string());
        }
        self.fetch_ongoing_games();
        self.ui_state.menu_cursor = 0;
        let page = self.pages.ongoing_games();
        self.set_page(page);
    }

    pub fn lichess_open_join_code(&mut self) {
        self.show_enter_game_code();
        self.game.ui.prompt.reset();
    }

    pub fn lichess_disconnect(&mut self) {
        self.disconnect_lichess();
    }

    pub fn ongoing_games(&self) -> &[::lichess::OngoingGame] {
        &self.lichess_state.ongoing_games
    }

    pub fn ongoing_game_open(&mut self, index: usize) {
        self.ui_state.menu_cursor = index as u8;
        self.select_ongoing_game();
    }

    pub fn ongoing_game_request_resign(&mut self, index: usize) {
        self.ui_state.menu_cursor = index as u8;
        self.show_resign_confirmation();
    }

    pub fn ongoing_games_back(&mut self) {
        self.ui_state.menu_cursor = 0;
        let page = self.pages.lichess_menu(self.game_mode_state.clock_cursor);
        self.set_page(page);
    }

    pub fn pgn_mirror(&mut self, games: &[model::PgnViewer], game_idx: usize) {
        self.pgn_viewer_state = Some(games.to_vec());
        self.pgn_viewer_game_idx = game_idx;
    }

    pub fn pgn_sync_board(&mut self, games: &[model::PgnViewer], game_idx: usize) {
        crate::app::pgn_sync::sync_viewer_to_board(self, games, game_idx);
    }

    pub fn pgn_exit(&mut self) {
        self.pgn_viewer_state = None;
        self.pgn_viewer_game_idx = 0;
        self.reset_home();
    }

    pub fn game_take(&mut self) -> Game {
        std::mem::take(&mut self.game)
    }

    pub fn game_put(&mut self, game: Game) {
        self.game = game;
    }

    pub fn game_is_puzzle(&self) -> bool {
        self.lichess_state.puzzle_game.is_some()
    }

    pub fn game_selected_color(&self) -> Option<shakmaty::Color> {
        self.game_mode_state.selected_color
    }

    pub fn game_is_lichess(&self) -> bool {
        self.game.runtime.kind() == GameKind::Lichess
    }

    pub fn game_validate_promotion(&mut self, game: &mut Game, move_uci: String) -> bool {
        let placeholder = std::mem::take(&mut self.game);
        self.game = std::mem::take(game);
        let result = self.validate_puzzle_move_uci(move_uci);
        *game = std::mem::take(&mut self.game);
        self.game = placeholder;
        result
    }

    pub fn game_validate_move(
        &mut self,
        game: &mut Game,
        from: shakmaty::Square,
        to: shakmaty::Square,
    ) {
        let placeholder = std::mem::take(&mut self.game);
        self.game = std::mem::take(game);
        self.validate_puzzle_move_after_execution(from, to);
        *game = std::mem::take(&mut self.game);
        self.game = placeholder;
    }

    pub fn game_check_end(&mut self, game: &mut Game) {
        let placeholder = std::mem::take(&mut self.game);
        self.game = std::mem::take(game);
        self.check_and_show_game_end();
        *game = std::mem::take(&mut self.game);
        self.game = placeholder;
    }

    pub fn game_puzzle_hint(&mut self, game: &mut Game) {
        let placeholder = std::mem::take(&mut self.game);
        self.game = std::mem::take(game);
        if let Some(puzzle_game) = &self.lichess_state.puzzle_game {
            puzzle_game.show_hint(&mut self.game);
        }
        *game = std::mem::take(&mut self.game);
        self.game = placeholder;
    }

    pub fn game_toggle_end_screen(&mut self) {
        if self.ui_state.end_screen_visible {
            self.close_modal();
            self.ui_state.end_screen_dismissed = true;
        } else if self.ui_state.end_screen_dismissed {
            self.ui_state.end_screen_dismissed = false;
            self.show_end_screen();
        } else {
            self.show_end_screen();
        }
    }

    pub fn game_request_restart(&mut self) {
        self.request_restart();
    }

    pub fn game_reset_home(&mut self) {
        self.reset_home();
    }

    pub fn game_open_move_input(&mut self) {
        self.game.ui.prompt.reset();
        self.show_move_input();
    }

    pub fn game_cycle_skin(&mut self, next: bool) {
        self.cycle_skin(next);
    }

    pub fn game_prepare(&mut self, kind: GamePageKind) -> bool {
        match kind {
            GamePageKind::Bot => self.prepare_bot_page(),
            GamePageKind::Multiplayer => self.prepare_multiplayer_page(),
            GamePageKind::Solo | GamePageKind::Lichess => true,
        }
    }

    pub fn quit(&mut self) {
        self.flush_resume_state();
        if let Some(cancellation_token) = &self.lichess_state.cancellation_token {
            cancellation_token.store(true, std::sync::atomic::Ordering::Relaxed);
            log::info!("Cancelling Lichess seek before quit");
        }
        self.running = false;
    }

    pub fn tick(&mut self) {
        self.dispatch_tick();
        self.game.ui.update_cursor_blink();

        if let Some(mut puzzle_game) = self.lichess_state.puzzle_game.take() {
            puzzle_game.check_elo_update();
            if let Some(success_message) =
                puzzle_game.check_pending_move(&mut self.game, self.lichess_state.token.clone())
            {
                self.show_puzzle_end(success_message);
            }
            self.lichess_state.puzzle_game = Some(puzzle_game);
        }

        let is_lichess = self.game.runtime.kind() == GameKind::Lichess;
        let clock = &self.game.runtime.clock();
        if is_lichess
            && let ClockState::Running { active_color, .. } = self.game.runtime.clock().state()
            && clock.get_time(active_color, Instant::now()).is_zero()
        {
            self.game.runtime.force_time_up(active_color);
        }

        if let ClockState::TimeUp { loser_color } = self.game.runtime.clock().state() {
            let winner = loser_color.other();
            if !self.game.runtime.state().is_finished() {
                self.game.runtime.finish(GameOutcome::Timeout { winner });
            }
            self.check_and_show_game_end();
        }
    }
}

pub mod pgn_sync;
