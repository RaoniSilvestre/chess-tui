use crate::tui::Tui;
use crate::{App, AppResult};

impl App {
    /// Runs the TUI until the application quits, then restores the terminal.
    pub fn run(&mut self) -> AppResult<()> {
        let mut tui = Tui::new()?;
        let result = self.event_loop(&mut tui);
        let cleanup = cleanup_terminal();
        result.and(cleanup)
    }

    fn event_loop(&mut self, tui: &mut Tui) -> AppResult<()> {
        while self.running {
            tui.draw(self)?;
            self.handle_events(tui)?;
            self.update();
        }
        Ok(())
    }
}

fn cleanup_terminal() -> AppResult<()> {
    ratatui::try_restore()?;
    ratatui::crossterm::execute!(
        std::io::stdout(),
        ratatui::crossterm::event::DisableMouseCapture
    )?;
    Ok(())
}
