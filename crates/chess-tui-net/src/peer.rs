//! Client-side TCP peer connection.

use crate::NETWORK_BUFFER_SIZE;
use shakmaty::{Color, Move, Role};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};

/// A non-blocking TCP peer. `color` is the color of the remote player.
pub struct TcpPeer {
    stream: TcpStream,
    color: Color,
}

impl TcpPeer {
    pub fn connect(addr: &str, color: Option<Color>) -> Result<Self, String> {
        let mut stream = None;
        for attempt in 1..=5 {
            log::debug!("Connection attempt {} to {}", attempt, addr);
            match TcpStream::connect(addr) {
                Ok(s) => {
                    stream = Some(s);
                    break;
                }
                Err(e) => {
                    log::error!("Failed connection attempt {} to {}: {}", attempt, addr, e);
                }
            }
        }

        let Some(stream) = stream else {
            return Err(format!(
                "Failed to connect to the server after 5 attempts to {addr}"
            ));
        };

        let color = match color {
            Some(color) => color,
            None => read_color(&stream)?,
        };

        stream
            .set_nonblocking(true)
            .map_err(|e| format!("Failed to set stream to non-blocking: {e}"))?;

        Ok(Self { stream, color })
    }

    pub fn color(&self) -> Color {
        self.color
    }

    pub fn wait_for_start(&mut self) -> Result<bool, String> {
        let mut buffer = [0; NETWORK_BUFFER_SIZE];
        match self.stream.read(&mut buffer) {
            Ok(0) => Ok(false),
            Ok(bytes_read) => {
                let response = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                match response.as_str() {
                    "s" => Ok(true),
                    _ => Err("Failed to get start signal from stream".to_string()),
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(false),
            Err(e) => Err(format!("Failed to read start signal from stream: {e}")),
        }
    }

    /// Returns `None` when no move is available yet.
    pub fn try_read_move(&mut self) -> Result<Option<String>, String> {
        let mut buffer = [0; NETWORK_BUFFER_SIZE];
        match self.stream.read(&mut buffer) {
            Ok(0) => Ok(None),
            Ok(bytes_read) => {
                let response = String::from_utf8_lossy(&buffer[..bytes_read]);
                if response.trim() == "ended" {
                    Err("Game ended by the other opponent".to_string())
                } else if response.trim().is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(response.to_string()))
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(format!("Failed to read from stream: {e}")),
        }
    }

    pub fn send_move(&mut self, chess_move: &Move) -> Result<(), String> {
        let from = chess_move.from().map(|s| s.to_string()).unwrap_or_default();
        let to = chess_move.to().to_string();
        let promotion = match chess_move.promotion() {
            Some(Role::Queen) => "q",
            Some(Role::Rook) => "r",
            Some(Role::Bishop) => "b",
            Some(Role::Knight) => "n",
            _ => "",
        };
        self.stream
            .write_all(format!("{from}{to}{promotion}").as_bytes())
            .map_err(|e| format!("Failed to send move: {e}"))
    }

    pub fn send_end(&mut self) {
        let _ = self.stream.write_all(b"ended");
        let _ = self.stream.shutdown(Shutdown::Both);
    }
}

fn read_color(mut stream: &TcpStream) -> Result<Color, String> {
    let mut buffer = [0; NETWORK_BUFFER_SIZE];
    match stream.read(&mut buffer) {
        Ok(bytes_read) => {
            let response = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
            match response.as_str() {
                "w" => Ok(Color::White),
                "b" => Ok(Color::Black),
                _ => Err("Failed to get color from stream".to_string()),
            }
        }
        Err(e) => Err(format!("Failed to read color from stream: {e}")),
    }
}
