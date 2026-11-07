//! Application shell: owns the active page and application-wide globals.

use std::error;

use pages::{Globals, Page, PageAction, PageKind, PageRequest, create_page};
use ratatui::crossterm::event::{KeyEvent, KeyEventKind, MouseEvent};
use ratatui::{Frame, layout::Rect};

pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

pub struct App {
    pub running: bool,
    pub page: Box<dyn Page>,
    pub globals: Globals,
}

impl App {
    pub fn new(mut globals: Globals) -> Self {
        let page = create_page(PageRequest::Home, &mut globals);
        Self {
            running: true,
            page,
            globals,
        }
    }

    pub fn page_kind(&self) -> PageKind {
        self.page.kind()
    }

    pub fn navigate(&mut self, request: PageRequest) {
        self.page.on_exit(&mut self.globals);
        let mut page = create_page(request, &mut self.globals);
        let action = page.on_enter(&mut self.globals);
        self.page = page;
        self.apply(action);
    }

    pub fn dispatch_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        if let Some(action) = self.globals.handle_overlay_key(key) {
            self.apply(action);
            return;
        }
        let action = self.page.handle_key(key, &mut self.globals);
        if matches!(action, PageAction::None) && !self.page.is_modal_active() {
            self.fallback_key_handler(key);
        }
        self.apply(action);
    }

    pub fn dispatch_mouse(&mut self, mouse: MouseEvent) {
        if self.globals.has_overlay() {
            return;
        }
        let action = self.page.handle_mouse(mouse, &mut self.globals);
        self.apply(action);
    }

    pub fn tick(&mut self) {
        let action = self.page.tick(&mut self.globals);
        self.apply(action);
    }

    pub fn update(&mut self) {
        let action = self.page.update(&mut self.globals);
        self.apply(action);
    }

    pub fn render_page(&mut self, frame: &mut Frame<'_>, area: Rect) {
        self.page.render(frame, area, &mut self.globals);
    }

    pub fn render_overlay(&self, frame: &mut Frame<'_>, area: Rect) {
        let (is_solo, is_puzzle) = self.page.help_context();
        self.globals
            .render_overlay(frame, area, self.page.kind(), is_solo, is_puzzle);
    }

    pub fn quit(&mut self) {
        self.page.on_exit(&mut self.globals);
        self.running = false;
    }

    fn apply(&mut self, action: PageAction) {
        match action {
            PageAction::None | PageAction::Handled => {}
            PageAction::Quit => self.quit(),
            PageAction::Navigate(request) => self.navigate(request),
        }
    }
}
