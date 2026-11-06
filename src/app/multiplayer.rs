//! TCP multiplayer session setup and teardown.

use crate::app::App;
use crate::constants::Popups;
use chess_tui_game::TcpMode;
use chess_tui_net::{NETWORK_PORT, get_host_ip};
use shakmaty::Color;

impl App {
    /// Connects to a TCP multiplayer game. If hosting, opens a waiting popup and resolves the local IP.
    /// If joining, fetches the assigned color from the server. Flips the board for the Black player.
    pub fn create_opponent(&mut self) {
        let hosting = self.multiplayer_state.hosting.unwrap_or(false);
        // Host passes the guest's color; guest reads the host's color from the server.
        let other_player_color = if hosting {
            self.game_mode_state.selected_color.map(|c| c.other())
        } else {
            None
        };

        if hosting {
            self.ui_state.current_popup = Some(Popups::WaitingForOpponentToJoin);
            if let Some(ip) = get_host_ip() {
                self.multiplayer_state.host_ip = Some(format!("{}:{}", ip, NETWORK_PORT));
            } else {
                log::error!("Could not get local IP, defaulting to 127.0.0.1");
                self.multiplayer_state.host_ip = Some(format!("127.0.0.1:{}", NETWORK_PORT));
            }
        }

        let addr_with_port = self
            .multiplayer_state
            .host_ip
            .as_deref()
            .unwrap_or(&format!("127.0.0.1:{}", NETWORK_PORT))
            .to_string();

        match TcpMode::new(&addr_with_port, other_player_color) {
            Ok(mode) => {
                if !hosting {
                    self.game_mode_state.selected_color = Some(mode.my_color());
                    self.game_mode_state.is_random_color = false;
                }
                self.game.runtime.set_mode(Box::new(mode));
            }
            Err(e) => {
                log::error!("Failed to create TCP peer: {}", e);
                self.multiplayer_state.host_ip = None;
                self.ui_state
                    .show_message_popup(format!("Connection failed: {}", e), Popups::Error);
                return;
            }
        }

        if self.game_mode_state.selected_color.unwrap_or(Color::White) == Color::Black {
            log::debug!("Flipping board for black player");
            self.game.runtime.flip();
        }

        // Ensure skin is preserved when starting multiplayer
        if let Some(skin) = &self.theme_state.loaded_skin {
            self.game.ui.skin = skin.clone();
        }
    }

    /// Cancels an in-progress hosting session. Shuts down the TCP socket and resets
    /// multiplayer, board, and UI state.
    pub fn cancel_hosting_cleanup(&mut self) {
        log::info!("Cancelling hosting and cleaning multiplayer state");

        self.game.runtime.disconnect();
        self.multiplayer_state.reset();
        self.game_mode_state.reset_selected_color();
        self.game.runtime.reset();
        self.game.ui.reset();
    }
}
