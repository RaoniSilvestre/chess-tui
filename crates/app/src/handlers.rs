//! Routes raw crossterm events to the active page.

use crate::{App, AppResult, event::Event, tui::Tui};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    pub fn handle_events(&mut self, tui: &mut Tui) -> AppResult<()> {
        if tui.animations_active() {
            match tui.events.try_next() {
                Some(Event::Tick) => self.tick(),
                Some(Event::Key(key_event)) => self.dispatch_key(key_event),
                Some(Event::Mouse(mouse_event)) => self.dispatch_mouse(mouse_event),
                Some(Event::Resize(_, _)) | None => {}
            }
            std::thread::sleep(std::time::Duration::from_millis(16));
        } else {
            match tui.events.next()? {
                Event::Tick => self.tick(),
                Event::Key(key_event) => self.dispatch_key(key_event),
                Event::Mouse(mouse_event) => self.dispatch_mouse(mouse_event),
                Event::Resize(_, _) => {}
            }
        }
        Ok(())
    }

    /// Fallback handler for keys that aren't handled by specific page/popup handlers.
    pub(crate) fn fallback_key_handler(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.quit(),
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => self.quit(),
            KeyCode::Char('s') => {
                self.globals.theme.cycle(true);
                self.globals.persist_ui_config();
            }
            _ => (),
        }
    }
}
