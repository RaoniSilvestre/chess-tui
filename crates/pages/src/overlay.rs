//! Global overlays: help, errors, success and loading messages.
//!
//! Page-specific prompts live in their page module; only overlays that any
//! page can trigger are hosted here.

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;

use crate::globals::Globals;
use crate::page::{PageAction, PageKind, PageRequest};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MessageKind {
    Error,
    Success,
    Loading,
}

pub enum Overlay {
    Help,
    Message {
        kind: MessageKind,
        message: String,
        on_close: Option<PageRequest>,
    },
}

pub enum OverlayResult {
    None,
    Close,
    Action(PageAction),
}

impl Overlay {
    pub fn handle_key(&mut self, key: KeyEvent) -> OverlayResult {
        match self {
            Overlay::Help => match key.code {
                KeyCode::Char('?') | KeyCode::Esc => OverlayResult::Close,
                _ => OverlayResult::None,
            },
            Overlay::Message { on_close, .. } => match key.code {
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ') => {
                    if let Some(request) = on_close.take() {
                        OverlayResult::Action(PageAction::Navigate(request))
                    } else {
                        OverlayResult::Close
                    }
                }
                KeyCode::Char('q') => OverlayResult::Action(PageAction::Quit),
                _ => OverlayResult::None,
            },
        }
    }

    pub fn render(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        page_kind: PageKind,
        is_solo: bool,
        is_puzzle: bool,
    ) {
        match self {
            Overlay::Help => ui::overlay::help::render_help_popup(
                frame,
                ui::overlay::help::HelpContext {
                    is_pgn: page_kind == PageKind::PgnViewer,
                    is_solo,
                    is_puzzle,
                },
            ),
            Overlay::Message { kind, message, .. } => match kind {
                MessageKind::Error => ui::overlay::error::render_error_popup(frame, message),
                MessageKind::Success => ui::overlay::success::render_success_popup(frame, message),
                MessageKind::Loading => {
                    ui::overlay::loading::render_loading_popup(frame, message.clone())
                }
            },
        }
        let _ = area;
    }
}

impl Globals {
    pub fn show_help(&mut self) {
        self.overlay = Some(Overlay::Help);
    }

    pub fn show_error(&mut self, message: impl Into<String>) {
        self.show_message(MessageKind::Error, message.into(), None);
    }

    pub fn show_error_then(&mut self, message: impl Into<String>, request: PageRequest) {
        self.show_message(MessageKind::Error, message.into(), Some(request));
    }

    pub fn show_success(&mut self, message: impl Into<String>) {
        self.show_message(MessageKind::Success, message.into(), None);
    }

    pub fn show_success_then(&mut self, message: impl Into<String>, request: PageRequest) {
        self.show_message(MessageKind::Success, message.into(), Some(request));
    }

    pub fn show_loading(&mut self, message: impl Into<String>) {
        self.show_message(MessageKind::Loading, message.into(), None);
    }

    fn show_message(&mut self, kind: MessageKind, message: String, on_close: Option<PageRequest>) {
        self.overlay = Some(Overlay::Message {
            kind,
            message,
            on_close,
        });
    }

    pub fn close_overlay(&mut self) {
        self.overlay = None;
    }

    pub fn has_overlay(&self) -> bool {
        self.overlay.is_some()
    }

    pub fn handle_overlay_key(&mut self, key: KeyEvent) -> Option<PageAction> {
        let mut overlay = self.overlay.take()?;
        let result = overlay.handle_key(key);
        let action = match result {
            OverlayResult::None => {
                self.overlay = Some(overlay);
                None
            }
            OverlayResult::Close => None,
            OverlayResult::Action(action) => Some(action),
        };
        Some(action.unwrap_or(PageAction::None))
    }

    pub fn render_overlay(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        page_kind: PageKind,
        is_solo: bool,
        is_puzzle: bool,
    ) {
        if let Some(overlay) = &self.overlay {
            overlay.render(frame, area, page_kind, is_solo, is_puzzle);
        }
    }
}
