use crate::{
    app::{App, AppResult},
    event::Event,
    handlers::handler::handle_key_events,
    sound::play_move_sound,
    ui::tui::Tui,
};
use chess_tui_game::GameEvent;

impl App {
    pub fn run(&mut self) -> AppResult<()> {
        // Initialize the terminal user interface.
        let mut tui = Tui::new()?;

        // Start the main loop.
        while self.running {
            // Render the user interface.
            tui.draw(self)?;

            self.handle_events(&mut tui)?;

            self.handle_game_events();
            self.check_game_end_status();

            self.check_lichess_updates();

            // Persist local/bot games each loop iteration. A move always
            // triggers a write; clock-only changes are throttled to ~1 Hz inside
            // so we don't hit the disk on every animation frame.
            self.tick_resume_state();
        }

        Ok(())
    }

    fn handle_events(&mut self, tui: &mut Tui) -> AppResult<()> {
        // Handle events fast loop during animations, blocking otherwise.
        if self.animations.is_active() {
            match tui.events.try_next() {
                Some(Event::Tick) => self.tick(),
                Some(Event::Key(key_event)) => handle_key_events(key_event, self)?,
                Some(Event::Mouse(mouse_event)) => self.dispatch_mouse(mouse_event),
                Some(Event::Resize(_, _)) | None => {}
            }
            std::thread::sleep(std::time::Duration::from_millis(16));
        } else {
            match tui.events.next()? {
                Event::Tick => self.tick(),
                Event::Key(key_event) => handle_key_events(key_event, self)?,
                Event::Mouse(mouse_event) => self.dispatch_mouse(mouse_event),
                Event::Resize(_, _) => {}
            }
        };

        Ok(())
    }

    fn handle_game_events(&mut self) {
        for event in self.game.runtime.take_events() {
            match event {
                GameEvent::MovePlayed => play_move_sound(),
                GameEvent::Error(message) => {
                    log::error!("Game mode error: {message}");
                    self.ui_state.show_message_popup(
                        format!("Game error:\n{message}"),
                        crate::constants::Popups::Error,
                    );
                }
                GameEvent::OpponentJoined => {
                    self.ui_state.close_popup();
                }
            }
        }
    }
}
