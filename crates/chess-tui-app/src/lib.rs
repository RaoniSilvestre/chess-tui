//! Page and modal framework for the application.

pub mod game_mode;

use ratatui::Frame;
use ratatui::crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::Rect;

/// Navigation result returned by a page or modal.
pub enum Nav<C: AppContext> {
    None,
    Replace(Box<dyn Page<C>>),
    Quit,
}

/// Shared application context exposed to pages.
///
/// Pages own their local state and call these methods for navigation and
/// shared services. The concrete application implements this trait.
pub trait AppContext {
    fn home_cursor(&self) -> u8;
    fn home_cursor_up(&mut self);
    fn home_cursor_down(&mut self);

    fn open_game_mode_menu(&mut self);
    fn open_lichess_menu(&mut self);
    fn open_settings(&mut self);
    fn open_help(&mut self);
    fn open_credits(&mut self);
    fn go_home(&mut self);
    fn quit(&mut self);

    /// Build the page for the current navigation state, if it changed.
    fn take_page(&mut self) -> Option<Box<dyn Page<Self>>>
    where
        Self: Sized;
}

/// A full-screen page. Pages own their local state and compose UI widgets.
pub trait Page<C: AppContext> {
    fn handle_key(&mut self, key: KeyEvent, ctx: &mut C) -> Nav<C>;
    fn handle_mouse(&mut self, _mouse: MouseEvent, _ctx: &mut C) -> Nav<C> {
        Nav::None
    }
    fn tick(&mut self, _ctx: &mut C) -> Nav<C> {
        Nav::None
    }
    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, ctx: &mut C);
}

/// Owns the active page and applies navigation results.
pub struct Router<C: AppContext> {
    page: Box<dyn Page<C>>,
}

impl<C: AppContext> Router<C> {
    pub fn new(page: Box<dyn Page<C>>) -> Self {
        Self { page }
    }

    pub fn page(&self) -> &dyn Page<C> {
        self.page.as_ref()
    }

    pub fn page_mut(&mut self) -> &mut dyn Page<C> {
        self.page.as_mut()
    }

    pub fn handle_key(&mut self, key: KeyEvent, ctx: &mut C) {
        let nav = self.page.handle_key(key, ctx);
        self.apply(nav, ctx);
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent, ctx: &mut C) {
        let nav = self.page.handle_mouse(mouse, ctx);
        self.apply(nav, ctx);
    }

    pub fn tick(&mut self, ctx: &mut C) {
        let nav = self.page.tick(ctx);
        self.apply(nav, ctx);
    }

    pub fn render(&mut self, frame: &mut Frame<'_>, area: Rect, ctx: &mut C) {
        self.page.render(frame, area, ctx);
    }

    fn apply(&mut self, nav: Nav<C>, ctx: &mut C) {
        match nav {
            Nav::None => {
                if let Some(page) = ctx.take_page() {
                    self.page = page;
                }
            }
            Nav::Replace(page) => self.page = page,
            Nav::Quit => ctx.quit(),
        }
    }
}

/// Settings-specific context.
pub trait SettingsContext: AppContext {
    fn settings_has_sound(&self) -> bool;
    fn settings_kitty_available(&self) -> bool;

    fn settings_skin_label(&self) -> String;
    fn settings_sound_label(&self) -> String;
    fn settings_animations_label(&self) -> String;
    fn settings_bot_depth_label(&self) -> String;
    fn settings_bot_difficulty_label(&self) -> String;
    fn settings_kitty_label(&self) -> String;

    fn settings_cycle_skin(&mut self, next: bool);
    fn settings_toggle_sound(&mut self);
    fn settings_toggle_animations(&mut self);
    fn settings_open_engine_path(&mut self);
    fn settings_bot_depth_down(&mut self);
    fn settings_bot_depth_up(&mut self);
    fn settings_cycle_difficulty(&mut self, next: bool);
    fn settings_toggle_kitty(&mut self);
}

pub use game_mode::{AvailableGameMode, GameModeState};

/// Game-mode menu context.
pub trait GameModeContext: AppContext {
    fn game_mode_menu_cursor(&self) -> u8;
    fn game_mode_set_menu_cursor(&mut self, cursor: u8);

    fn game_mode_state(&self) -> &GameModeState;
    fn game_mode_state_mut(&mut self) -> &mut GameModeState;

    fn game_mode_bot_depth(&self) -> u8;
    fn game_mode_set_bot_depth(&mut self, depth: u8);
    fn game_mode_bot_difficulty(&self) -> Option<u8>;
    fn game_mode_cycle_difficulty(&mut self, next: bool);
    fn game_mode_engine_path(&self) -> Option<String>;

    fn game_mode_hosting(&self) -> Option<bool>;
    fn game_mode_set_hosting(&mut self, hosting: Option<bool>);

    fn game_mode_has_resume(&self, kind: AvailableGameMode) -> bool;
    fn game_mode_resume(&mut self, kind: AvailableGameMode);
    fn game_mode_submit(&mut self, kind: AvailableGameMode);
    fn game_mode_open_pgn_loader(&mut self);
}

/// Lichess landing menu context.
pub trait LichessMenuContext: AppContext {
    fn lichess_has_token(&self) -> bool;
    fn lichess_open_token_prompt(&mut self);

    fn lichess_profile(&self) -> Option<&chess_tui_lichess::UserProfile>;
    fn lichess_rating_history(&self) -> Option<&[chess_tui_lichess::RatingHistoryEntry]>;

    fn lichess_time_control(&self) -> u8;
    fn lichess_set_time_control(&mut self, cursor: u8);
    fn lichess_seek(&mut self, cursor: u8);
    fn lichess_start_puzzle(&mut self);
    fn lichess_open_ongoing_games(&mut self);
    fn lichess_open_join_code(&mut self);
    fn lichess_disconnect(&mut self);
}

/// Ongoing Lichess games list context.
pub trait OngoingGamesContext: AppContext {
    fn ongoing_games(&self) -> &[chess_tui_lichess::OngoingGame];
    fn ongoing_game_open(&mut self, index: usize);
    fn ongoing_game_request_resign(&mut self, index: usize);
    fn ongoing_games_back(&mut self);
}
