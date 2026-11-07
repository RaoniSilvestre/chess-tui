//! Terminal lifecycle wrapper.

use crate::animations::AnimationState;
use crate::event::EventHandler;
use crate::{App, AppResult};
use ratatui::DefaultTerminal;

/// Representation of a terminal user interface.
pub struct Tui {
    /// Interface to the Terminal.
    terminal: DefaultTerminal,
    /// Terminal event handler.
    pub events: EventHandler,
    /// Page-transition animations.
    pub animations: AnimationState,
}

impl Tui {
    /// Constructs a new instance of [`Tui`].
    pub fn new() -> AppResult<Self> {
        let terminal = ratatui::try_init()?;
        let events = EventHandler::new(250);

        Ok(Self {
            terminal,
            events,
            animations: AnimationState::default(),
        })
    }

    pub fn animations_active(&self) -> bool {
        self.animations.is_active()
    }

    /// Draws one frame: the active page, then global overlays and animations.
    pub fn draw(&mut self, app: &mut App) -> AppResult<()> {
        let animations = &mut self.animations;
        self.terminal.draw(|frame| {
            let area = frame.area();
            app.render_page(frame, area);
            app.render_overlay(frame, area);
            animations.render(frame, app, area);
        })?;
        Ok(())
    }
}
