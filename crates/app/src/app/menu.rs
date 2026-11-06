//! Menu navigation and skin cycling.

use crate::App;
use crate::constants::{DEFAULT_CUSTOM_TIME_VALUE, DEFAULT_TIME_CONTROL_SELECTED};
use crate::game_mode_menu::{cycle_difficulty_next, cycle_difficulty_prev};
use session::Game;
use theme::DisplayMode;
use ui::sound::play_menu_nav_sound;
impl App {
    /// Cycles to the next or previous skin and updates the display mode to match.
    /// Pass `true` to advance forward, `false` to go back.
    pub fn cycle_skin(&mut self, next: bool) {
        let future_skin = self.theme_state.get_skin(next);
        let future_skin_name = future_skin.clone().name;

        play_menu_nav_sound(); // move

        // Update selected skin name and apply it
        self.theme_state.loaded_skin = Some(future_skin.clone());
        self.theme_state.selected_skin_name = future_skin_name.clone();
        self.game_mut().ui.skin = future_skin;

        // Set display mode based on skin name
        let display_mode = match future_skin_name.as_str() {
            "Default" => DisplayMode::DEFAULT,
            "ASCII" => DisplayMode::ASCII,
            _ => DisplayMode::CUSTOM,
        };

        self.game_mut().ui.display_mode = display_mode;
    }

    /// Cycles to the next or previous bot difficulty level.
    /// Pass `true` to advance forward, `false` to go back.
    pub fn cycle_bot_difficulty(&mut self, next: bool) {
        if next {
            cycle_difficulty_next(self);
        } else {
            cycle_difficulty_prev(self);
        }

        play_menu_nav_sound(); // move
    }

    /// Resets the application state and returns to the home page.
    /// Preserves display mode preference while cleaning up all game state,
    /// bot state, and multiplayer connections.
    pub fn reset_home(&mut self) {
        // Snapshot the clock before tearing down the game — otherwise a
        // resumed game would start with the time from the previous move
        // rather than the time at the moment the user pressed `b`.
        self.flush_resume_state();

        // Preserve display mode and skin preference
        let display_mode = self.game_mut().ui.display_mode;
        let current_skin = self.game_mut().ui.skin.clone();

        // Cancel any active Lichess seek before resetting
        if let Some(cancellation_token) = &self.lichess_state.cancellation_token {
            cancellation_token.store(true, std::sync::atomic::Ordering::Relaxed);
            log::info!("Cancelling Lichess seek before returning to home");
        }
        self.lichess_state.cancellation_token = None;
        self.lichess_state.receiver = None;

        // Reset game-related state
        self.game_mode_state.selected_color = None;
        self.game_mode_state.is_random_color = false;

        // Clean up any active network mode.
        self.game_mut().runtime.disconnect();
        self.multiplayer_state.hosting = None;
        self.multiplayer_state.host_ip = None;

        // Clear puzzle state
        self.lichess_state.puzzle_game = None;

        // Reset game completely but preserve display mode, skin and piece styles
        self.replace_game(Game::default());
        self.game_mut().ui.display_mode = display_mode;
        self.game_mut().ui.skin = current_skin;
        self.game_mut().ui.available_piece_styles = self.theme_state.available_piece_styles.clone();
        self.game_mode_state.clock_cursor = 3; // Reset to default (Rapid)
        self.game_mode_state.custom_time_minutes = 10; // Reset custom time
        let page = self.pages.home();
        self.set_page(page);
        self.close_modal();
    }

    /// Closes popup and navigates to home page.
    /// Used by popups that should return to home when closed.
    pub fn close_popup_and_go_home(&mut self) {
        self.close_modal();
        self.game_mode_state.clock_cursor = DEFAULT_TIME_CONTROL_SELECTED; // Reset to default (Rapid)
        self.game_mode_state.custom_time_minutes = DEFAULT_CUSTOM_TIME_VALUE; // Reset custom time
        let page = self.pages.home();
        self.set_page(page);
    }
}
