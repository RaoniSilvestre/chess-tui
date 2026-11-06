//! Terminal lifecycle wrapper.

use crate::app::{App, AppResult};
use crate::event::EventHandler;
use crate::ui::main_ui;
use ratatui::DefaultTerminal;

/// Representation of a terminal user interface.
///
/// It is responsible for setting up the terminal,
/// initializing the interface and handling the draw events.
#[derive(Debug)]
pub struct Tui {
    /// Interface to the Terminal.
    terminal: DefaultTerminal,
    /// Terminal event handler.
    pub events: EventHandler,
}

impl Tui {
    /// Constructs a new instance of [`Tui`].
    pub fn new() -> AppResult<Self> {
        let terminal = ratatui::try_init()?;
        let events = EventHandler::new(250);

        Ok(Self { terminal, events })
    }

    /// Draws one frame: the active page, then global overlays and animations.
    pub fn draw(&mut self, app: &mut App) -> AppResult<()> {
        self.terminal.draw(|frame| {
            let area = frame.area();
            app.sync_page();
            if let Some(mut router) = app.router.take() {
                router.render(frame, area, app);
                app.router = Some(router);
            }
            main_ui::render_popups(frame, app, area);
            main_ui::render_animations(frame, app, area);
        })?;
        Ok(())
    }
}
