//! TCP multiplayer protocol, server, and peer connection.

pub const NETWORK_PORT: u16 = 2308;
pub const NETWORK_BUFFER_SIZE: usize = 5;
pub const SLEEP_DURATION_SHORT_MS: u64 = 50;
pub const SLEEP_DURATION_LONG_MS: u64 = 100;

mod peer;
mod server;

pub use peer::TcpPeer;
pub use server::{get_host_ip, setup_game_server};
