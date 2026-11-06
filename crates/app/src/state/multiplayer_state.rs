//! Tracks whether this instance is hosting, the peer's IP address, and the game-start receive channel.

/// TCP peer-to-peer session state held by [`crate::App`].
///
/// `hosting` distinguishes the two roles: the host listens for an incoming
/// connection while the guest connects to `host_ip`.
#[derive(Default)]
pub struct MultiplayerState {
    /// Whether the player is hosting
    pub hosting: Option<bool>,
    /// Host IP address with port
    pub host_ip: Option<String>,
}

impl MultiplayerState {
    /// Clears all session state so a new connection can be set up.
    pub fn reset(&mut self) {
        self.hosting = None;
        self.host_ip = None;
    }
}
