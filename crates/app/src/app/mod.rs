//! Top-level application state and services.

use crate::animations::AnimationState;
use crate::game_mode::{AvailableGameMode, GameModeState};
use session::{Game, GameHandle, game_handle};
use crate::state::bot_state::BotState;
use crate::state::lichess_state::LichessState;
use crate::state::multiplayer_state::MultiplayerState;
use crate::state::resume::ResumeMode;
use crate::state::theme_state::ThemeState;
use crate::{
    GameAction, GameModeAction, GamePageKind, GameView, GameModeView, LichessAction,
    LichessView, Modal, ModalResult, OngoingAction, Page, PageAction, PageContext, PageFactory,
    PgnAction, PgnViewerAccess, Router, SettingsAction, SettingsView,
};
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
    pub game: GameHandle,
    pub log_level: LevelFilter,
    pub sound_enabled: bool,
    pub animations_enabled: bool,
    pub kitty_graphics_enabled: bool,
    pub theme_state: ThemeState,
    pub bot_state: BotState,
    pub multiplayer_state: MultiplayerState,
    pub game_mode_state: GameModeState,
    pub lichess_state: LichessState,
    pub animations: AnimationState,
    pub last_saved_move_count: Option<usize>,
    pub last_resume_flush_at: Option<Instant>,
    pub router: Option<Router>,
    pub pages: Box<dyn PageFactory>,
    pending_page: Option<Box<dyn Page>>,
    modal: Option<Box<dyn Modal>>,
}

impl App {
    pub fn new(pages: Box<dyn PageFactory>) -> Self {
        let router = Router::new(pages.home());
        Self {
            kitty_pieces: None,
            running: true,
            game: game_handle(Game::default()),
            log_level: LevelFilter::Off,
            theme_state: ThemeState::default(),
            bot_state: BotState::default(),
            multiplayer_state: MultiplayerState::default(),
            game_mode_state: GameModeState::default(),
            lichess_state: LichessState::default(),
            sound_enabled: true,
            animations_enabled: false,
            kitty_graphics_enabled: true,
            animations: AnimationState::default(),
            last_saved_move_count: None,
            last_resume_flush_at: None,
            router: Some(router),
            pages,
            pending_page: None,
            modal: None,
        }
    }

    pub fn show_solo(&mut self) {
        let page = self.pages.solo(self.game.clone());
        self.set_page(page);
    }

    pub fn show_bot(&mut self) {
        let page = self.pages.bot(self.game.clone());
        self.set_page(page);
    }

    pub fn show_multiplayer(&mut self) {
        let page = self.pages.multiplayer(self.game.clone());
        self.set_page(page);
    }

    pub fn show_pgn_viewer(&mut self, games: Vec<model::PgnViewer>, game_idx: usize) {
        let page = self.pages.pgn_viewer(games, game_idx);
        self.set_page(page);
    }

    pub fn show_lichess(&mut self) {
        let page = self.pages.lichess(self.game.clone());
        self.set_page(page);
    }

    pub fn open_modal(&mut self, modal: Box<dyn Modal>) {
        self.modal = Some(modal);
    }

    pub fn close_modal(&mut self) {
        self.modal = None;
    }

    pub fn has_any_modal(&self) -> bool {
        self.modal.is_some()
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
        self.game_mut().ui.prompt.reset();
        self.open_modal(Box::new(crate::modal::PromptModal::enter_host_ip()));
    }

    pub fn show_enter_game_code(&mut self) {
        self.game_mut().ui.prompt.reset();
        self.open_modal(Box::new(crate::modal::PromptModal::enter_game_code()));
    }

    pub fn show_enter_lichess_token(&mut self) {
        self.game_mut().ui.prompt.reset();
        self.game_mut().ui.prompt.message = "Enter your Lichess API token:".to_string();
        self.open_modal(Box::new(crate::modal::PromptModal::enter_lichess_token()));
    }

    pub fn show_load_pgn_path(&mut self) {
        self.game_mut().ui.prompt.reset();
        self.open_modal(Box::new(crate::modal::PromptModal::load_pgn_path()));
    }

    pub fn show_enter_engine_path(&mut self) {
        {
            let mut game = self.game_mut();
            game.ui.prompt.input =
                self.bot_state.chess_engine_path.clone().unwrap_or_default();
            game.ui.prompt.character_index = game.ui.prompt.input.chars().count();
        }
        self.open_modal(Box::new(crate::modal::PromptModal::enter_engine_path()));
    }

    pub fn show_move_input(&mut self) {
        self.game_mut().ui.prompt.reset();
        self.open_modal(Box::new(crate::modal::PromptModal::move_input()));
    }

    pub fn show_seeking_lichess_game(&mut self) {
        self.open_modal(Box::new(crate::modal::SeekingLichessModal));
    }

    pub fn show_waiting_for_opponent(&mut self) {
        self.open_modal(Box::new(crate::modal::WaitingForOpponentModal));
    }

    pub fn show_resign_confirmation(&mut self, index: usize) {
        self.open_modal(Box::new(crate::modal::ResignConfirmationModal::new(index)));
    }

    pub fn show_new_game_confirmation(&mut self, pending: crate::game_mode_menu::NewGame) {
        self.open_modal(Box::new(crate::modal::NewGameConfirmationModal::new(
            pending,
        )));
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

    pub fn game(&self) -> std::cell::Ref<'_, Game> {
        self.game.borrow()
    }

    pub fn game_mut(&self) -> std::cell::RefMut<'_, Game> {
        self.game.borrow_mut()
    }

    pub fn replace_game(&mut self, game: Game) {
        *self.game.borrow_mut() = game;
    }

    fn page_context(&mut self, include_kitty: bool) -> PageContext<'_> {
        let has_local_resume = self.has_resume_save(ResumeMode::Local);
        let has_bot_resume = self.has_resume_save(ResumeMode::Bot);
        let skin_name = {
            let game = self.game();
            match game.ui.display_mode {
                DisplayMode::DEFAULT => "Default".to_string(),
                DisplayMode::ASCII => "ASCII".to_string(),
                DisplayMode::CUSTOM => game.ui.skin.name.clone(),
            }
        };
        let is_lichess = {
            let game = self.game();
            game.runtime.kind() == GameKind::Lichess
        };
        let settings = SettingsView {
            has_sound: cfg!(feature = "sound"),
            kitty_available: self.kitty_pieces.is_some(),
            skin_name,
            sound_enabled: self.sound_enabled,
            animations_enabled: self.animations_enabled,
            bot_depth: self.bot_state.bot_depth,
            bot_difficulty: self.bot_state.bot_difficulty,
            kitty_graphics_enabled: self.kitty_graphics_enabled,
        };
        let game_mode = GameModeView {
            bot_depth: self.bot_state.bot_depth,
            bot_difficulty: self.bot_state.bot_difficulty,
            engine_path: self.bot_state.chess_engine_path.clone(),
            hosting: self.multiplayer_state.hosting,
            has_local_resume,
            has_bot_resume,
        };
        let game = GameView {
            is_puzzle: self.lichess_state.puzzle_game.is_some(),
            selected_color: self.game_mode_state.selected_color,
            is_lichess,
        };
        let has_token = self.lichess_has_token();
        let kitty = if include_kitty {
            self.kitty_pieces.as_mut()
        } else {
            None
        };
        let lichess = LichessView {
            has_token,
            profile: self.lichess_state.user_profile.as_ref(),
            rating_history: self.lichess_state.rating_history.as_deref(),
            ongoing_games: &self.lichess_state.ongoing_games,
        };
        PageContext {
            settings,
            game_mode,
            game,
            lichess,
            kitty,
            kitty_enabled: self.kitty_graphics_enabled,
        }
    }

    fn apply_page_action(&mut self, action: PageAction) {
        match action {
            PageAction::None => {}
            PageAction::Quit => self.quit(),
            PageAction::OpenHome => self.show_home(),
            PageAction::OpenGameModeMenu => self.open_game_mode_menu(),
            PageAction::OpenLichessMenu => self.open_lichess_menu(),
            PageAction::OpenSettings => self.open_settings(),
            PageAction::OpenCredits => self.open_credits(),
            PageAction::OpenHelp => self.show_help(),
            PageAction::Settings(action) => self.apply_settings_action(action),
            PageAction::GameMode(action) => self.apply_game_mode_action(action),
            PageAction::Lichess(action) => self.apply_lichess_action(action),
            PageAction::Ongoing(action) => self.apply_ongoing_action(action),
            PageAction::Pgn(PgnAction::Exit) => self.reset_home(),
            PageAction::Game(action) => self.apply_game_action(action),
        }
    }

    fn apply_settings_action(&mut self, action: SettingsAction) {
        match action {
            SettingsAction::CycleSkin(next) => self.settings_cycle_skin(next),
            SettingsAction::ToggleSound => self.settings_toggle_sound(),
            SettingsAction::ToggleAnimations => self.settings_toggle_animations(),
            SettingsAction::OpenEnginePath => self.settings_open_engine_path(),
            SettingsAction::BotDepthDown => self.settings_bot_depth_down(),
            SettingsAction::BotDepthUp => self.settings_bot_depth_up(),
            SettingsAction::CycleDifficulty(next) => self.settings_cycle_difficulty(next),
            SettingsAction::ToggleKitty => self.settings_toggle_kitty(),
        }
    }

    fn apply_game_mode_action(&mut self, action: GameModeAction) {
        match action {
            GameModeAction::Start {
                mode,
                time_control,
                color,
                random_color,
                hosting,
            } => {
                self.game_mode_state.selected_color = color;
                self.game_mode_state.is_random_color = random_color;
                self.multiplayer_state.hosting = hosting;
                self.game_mut().runtime.set_clock(Clock::new(time_control));
                match mode {
                    AvailableGameMode::Local => crate::game_mode_menu::finish_form(
                        self,
                        GamePageKind::Solo,
                    ),
                    AvailableGameMode::Bot => {
                        crate::game_mode_menu::finish_form(self, GamePageKind::Bot)
                    }
                    AvailableGameMode::Multiplayer => {
                        crate::game_mode_menu::finish_form(self, GamePageKind::Multiplayer)
                    }
                    AvailableGameMode::PGNLoader => self.show_load_pgn_path(),
                }
            }
            GameModeAction::Resume(mode) => match mode {
                AvailableGameMode::Local => {
                    self.resume_from_saved(ResumeMode::Local);
                }
                AvailableGameMode::Bot => {
                    self.resume_from_saved(ResumeMode::Bot);
                }
                _ => {}
            },
            GameModeAction::OpenPgnLoader => self.show_load_pgn_path(),
        }
    }

    fn apply_lichess_action(&mut self, action: LichessAction) {
        match action {
            LichessAction::Seek { time_control } => self.lichess_seek(time_control),
            LichessAction::Puzzle => self.lichess_start_puzzle(),
            LichessAction::OpenOngoingGames => self.lichess_open_ongoing_games(),
            LichessAction::JoinByCode => self.lichess_open_join_code(),
            LichessAction::Disconnect => self.lichess_disconnect(),
            LichessAction::PromptToken => self.lichess_open_token_prompt(),
        }
    }

    fn apply_ongoing_action(&mut self, action: OngoingAction) {
        match action {
            OngoingAction::Open(index) => self.ongoing_game_open(index),
            OngoingAction::RequestResign(index) => self.ongoing_game_request_resign(index),
            OngoingAction::Back => self.ongoing_games_back(),
        }
    }

    fn apply_game_action(&mut self, action: GameAction) {
        match action {
            GameAction::RequestRestart => self.request_restart(),
            GameAction::GoHome => self.reset_home(),
            GameAction::OpenMoveInput => self.show_move_input(),
            GameAction::OpenHelp => self.show_help(),
            GameAction::Quit => self.quit(),
            GameAction::PuzzleHint => {
                if let Some(puzzle_game) = &self.lichess_state.puzzle_game {
                    let mut game = self.game_mut();
                    puzzle_game.show_hint(&mut game);
                }
            }
            GameAction::ValidatePromotion { move_uci } => {
                self.validate_puzzle_move_uci(move_uci);
            }
            GameAction::ValidateMove { from, to } => {
                self.validate_puzzle_move_after_execution(from, to);
            }
            GameAction::CheckEnd => {
                self.check_and_show_game_end();
            }
        }
    }

    pub fn set_page(&mut self, page: Box<dyn Page>) {
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
        self.router.as_ref().and_then(|router| router.page().game_kind())
    }

    pub fn current_page_type_id(&self) -> Option<std::any::TypeId> {
        self.router
            .as_ref()
            .map(|router| router.page().as_any().type_id())
    }

    pub fn menu_cursor(&self) -> Option<u8> {
        self.router.as_ref()?.page().menu_cursor()
    }

    pub fn form_active(&self) -> Option<bool> {
        self.router.as_ref()?.page().form_active()
    }

    pub fn is_home(&self) -> bool {
        self.router
            .as_ref()
            .is_some_and(|router| router.page().is_home())
    }

    pub fn pgn_viewer(&self) -> Option<&dyn PgnViewerAccess> {
        self.router.as_ref()?.page().pgn_viewer()
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
            let action = {
                let ctx = self.page_context(false);
                router.handle_key(key, &ctx)
            };
            self.router = Some(router);
            self.apply_page_action(action);
        }
    }

    pub fn dispatch_mouse(&mut self, mouse: ratatui::crossterm::event::MouseEvent) {
        if self.modal.is_some() {
            return;
        }
        self.sync_page();
        if let Some(mut router) = self.router.take() {
            let action = {
                let ctx = self.page_context(false);
                router.handle_mouse(mouse, &ctx)
            };
            self.router = Some(router);
            self.apply_page_action(action);
        }
    }

    pub fn dispatch_tick(&mut self) {
        self.sync_page();
        if let Some(mut router) = self.router.take() {
            let action = {
                let ctx = self.page_context(false);
                router.tick(&ctx)
            };
            self.router = Some(router);
            self.apply_page_action(action);
        }
    }

    pub fn render_page(&mut self, frame: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect) {
        self.sync_page();
        if let Some(mut router) = self.router.take() {
            let mut ctx = self.page_context(true);
            router.render(frame, area, &mut ctx);
            self.router = Some(router);
        }
    }

    pub fn open_game_mode_menu(&mut self) {
        let page = self.pages.game_mode_menu();
        self.set_page(page);
    }

    pub fn show_home(&mut self) {
        let page = self.pages.home();
        self.set_page(page);
    }

    pub fn show_lichess_menu(&mut self) {
        let page = self.pages.lichess_menu();
        self.set_page(page);
    }

    pub fn show_ongoing_games(&mut self) {
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

    pub fn lichess_has_token(&self) -> bool {
        self.lichess_state
            .token
            .as_ref()
            .map(|token| !token.is_empty())
            .unwrap_or(false)
    }

    pub fn lichess_open_token_prompt(&mut self) {
        self.show_enter_lichess_token();
        self.game_mut().ui.prompt.reset();
        self.game_mut().ui.prompt.message = "Enter your Lichess API token:".to_string();
    }

    pub fn lichess_seek(&mut self, cursor: u8) {
        let page = self.pages.lichess(self.game.clone());
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
        let page = self.pages.ongoing_games();
        self.set_page(page);
    }

    pub fn lichess_open_join_code(&mut self) {
        self.show_enter_game_code();
        self.game_mut().ui.prompt.reset();
    }

    pub fn lichess_disconnect(&mut self) {
        self.disconnect_lichess();
    }

    pub fn ongoing_game_open(&mut self, index: usize) {
        self.select_ongoing_game(index);
    }

    pub fn ongoing_game_request_resign(&mut self, index: usize) {
        self.show_resign_confirmation(index);
    }

    pub fn ongoing_games_back(&mut self) {
        let page = self.pages.lichess_menu();
        self.set_page(page);
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
        self.game_mut().ui.update_cursor_blink();

        if let Some(mut puzzle_game) = self.lichess_state.puzzle_game.take() {
            puzzle_game.check_elo_update();
            let token = self.lichess_state.token.clone();
            let pending = {
                let mut game = self.game_mut();
                puzzle_game.check_pending_move(&mut game, token)
            };
            if let Some(success_message) = pending {
                self.show_puzzle_end(success_message);
            }
            self.lichess_state.puzzle_game = Some(puzzle_game);
        }

        let (is_lichess, clock, clock_state) = {
            let game = self.game();
            (
                game.runtime.kind() == GameKind::Lichess,
                game.runtime.clock().clone(),
                game.runtime.clock().state(),
            )
        };
        if is_lichess
            && let ClockState::Running { active_color, .. } = clock_state
            && clock.get_time(active_color, Instant::now()).is_zero()
        {
            self.game_mut().runtime.force_time_up(active_color);
        }

        let clock_state = self.game().runtime.clock().state();
        if let ClockState::TimeUp { loser_color } = clock_state {
            let winner = loser_color.other();
            let finished = self.game().runtime.state().is_finished();
            if !finished {
                self.game_mut().runtime.finish(GameOutcome::Timeout { winner });
            }
            self.check_and_show_game_end();
        }
    }
}

