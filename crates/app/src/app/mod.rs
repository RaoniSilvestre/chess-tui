//! Top-level application state and services.

use crate::services::Services;
use crate::{Page, PageFactory, Router};
use ::game::GameKind;
use model::{ClockState, GameOutcome};
use session::Game;
use std::error;
use std::time::Instant;

pub mod actions;
pub mod bot;
pub mod config;
pub mod game;
pub mod lichess;
pub mod menu;
pub mod modals;
pub mod navigation;
pub mod multiplayer;
pub mod resume;

/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

/// Top-level application context that owns all runtime state.
pub struct App {
    pub running: bool,
    pub router: Option<Router>,
    pub pages: Box<dyn PageFactory>,
    pub services: Services,
    pending_page: Option<Box<dyn Page>>,
}

impl App {
    pub fn new(pages: Box<dyn PageFactory>) -> Self {
        let router = Router::new(pages.home());
        Self {
            running: true,
            router: Some(router),
            pages,
            services: Services::default(),
            pending_page: None,
        }
    }
    pub fn game(&self) -> std::cell::Ref<'_, Game> {
        self.services.game.borrow()
    }

    pub fn game_mut(&self) -> std::cell::RefMut<'_, Game> {
        self.services.game.borrow_mut()
    }

    pub fn replace_game(&mut self, game: Game) {
        *self.services.game.borrow_mut() = game;
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
        if self.services.modal.is_some() {
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



















    pub fn settings_cycle_skin(&mut self, next: bool) {
        self.cycle_skin(next);
        self.update_config_from_app();
    }

    pub fn settings_toggle_sound(&mut self) {
        #[cfg(feature = "sound")]
        {
            self.services.sound_enabled = !self.services.sound_enabled;
            ui::sound::set_sound_enabled(self.services.sound_enabled);
            self.update_config_from_app();
        }
    }

    pub fn settings_toggle_animations(&mut self) {
        self.services.animations_enabled = !self.services.animations_enabled;
        self.update_config_from_app();
    }

    pub fn settings_open_engine_path(&mut self) {
        self.show_enter_engine_path();
    }

    pub fn settings_bot_depth_down(&mut self) {
        if self.services.bot_state.bot_depth > 1 {
            self.services.bot_state.bot_depth -= 1;
            self.update_config_from_app();
        }
    }

    pub fn settings_bot_depth_up(&mut self) {
        if self.services.bot_state.bot_depth < 20 {
            self.services.bot_state.bot_depth += 1;
            self.update_config_from_app();
        }
    }

    pub fn settings_cycle_difficulty(&mut self, next: bool) {
        self.cycle_bot_difficulty(next);
    }

    pub fn settings_toggle_kitty(&mut self) {
        self.services.kitty_graphics_enabled = !self.services.kitty_graphics_enabled;
        self.update_config_from_app();
    }

    pub fn lichess_has_token(&self) -> bool {
        self.services.lichess_state
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
        let page = self.pages.lichess(self.services.game.clone());
        self.set_page(page);
        self.create_lichess_opponent(cursor.clamp(3, 4));
    }

    pub fn lichess_start_puzzle(&mut self) {
        self.start_puzzle_mode();
    }

    pub fn lichess_open_ongoing_games(&mut self) {
        if self.services.lichess_state.ongoing_games.is_empty() {
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
        if let Some(cancellation_token) = &self.services.lichess_state.cancellation_token {
            cancellation_token.store(true, std::sync::atomic::Ordering::Relaxed);
            log::info!("Cancelling Lichess seek before quit");
        }
        self.running = false;
    }

    pub fn tick(&mut self) {
        self.dispatch_tick();
        self.game_mut().ui.update_cursor_blink();

        if let Some(mut puzzle_game) = self.services.lichess_state.puzzle_game.take() {
            puzzle_game.check_elo_update();
            let token = self.services.lichess_state.token.clone();
            let pending = {
                let mut game = self.game_mut();
                puzzle_game.check_pending_move(&mut game, token)
            };
            if let Some(success_message) = pending {
                self.show_puzzle_end(success_message);
            }
            self.services.lichess_state.puzzle_game = Some(puzzle_game);
        }

        let (is_lichess, clock, clock_state) = {
            let game = self.services.game();
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

        let clock_state = self.services.game().runtime.clock().state();
        if let ClockState::TimeUp { loser_color } = clock_state {
            let winner = loser_color.other();
            let finished = self.services.game().runtime.state().is_finished();
            if !finished {
                self.game_mut().runtime.finish(GameOutcome::Timeout { winner });
            }
            self.check_and_show_game_end();
        }
    }
}

