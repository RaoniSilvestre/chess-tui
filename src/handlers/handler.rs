//! Routes raw crossterm events to the active page handler and shared chess input bindings.

use app::{App, AppResult};
use model::Coord;
use model::square_utils::get_coord_from_square;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// Handles keyboard input events and updates the application state accordingly.
///
/// This is the main entry point for all keyboard interactions. It filters out
/// non-press events, handles mouse-to-keyboard transitions, and routes events
/// to either popup handlers or page handlers based on the current application state.
pub fn handle_key_events(key_event: KeyEvent, app: &mut App) -> AppResult<()> {
    // Only process key press events, ignore release and repeat events
    // (crossterm on Windows sends Release and Repeat events as well)
    if key_event.kind != KeyEventKind::Press {
        return Ok(());
    }

    // Reset cursor position after mouse interaction when keyboard is used
    // This ensures the cursor is in a valid position for keyboard navigation
    if app.game.ui.mouse_used {
        app.game.ui.mouse_used = false;
        if let Some(selected_square) = app.game.ui.selected_square {
            // If a piece was selected via mouse, move cursor to that square
            app.game.ui.cursor_coordinates =
                // Board flipping is already accounted for when selecting the square
                get_coord_from_square(selected_square, false);
            app.game.ui.selected_square = None;
        } else {
            app.game.ui.cursor_coordinates = Coord::default();
            // Otherwise, reset cursor to center of board (e4/e5)
        }
    }

    app.dispatch_key(key_event);

    Ok(())
}

/// Fallback handler for keys that aren't handled by specific page/popup handlers.
/// Provides global shortcuts like quit that work from anywhere.
pub fn fallback_key_handler(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Char('q') => app.quit(), // Quit application
        KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => app.quit(), // Ctrl+C to quit
        KeyCode::Char('s') => app.cycle_skin(true), // Cycle through available skins
        _ => (),                                    // Ignore other keys
    }
}
