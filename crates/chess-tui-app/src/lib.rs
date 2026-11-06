//! Page and modal framework for the application.

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
