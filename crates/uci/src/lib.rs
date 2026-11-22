//! A minimal [UCI](https://backscattering.de/chess/uci) client.
//!
//! This crate implements only the subset of the protocol used by
//! [chess-tui](https://github.com/thomas-mauran/chess-tui):
//!
//! - `uci` → waits for `uciok`
//! - `setoption name … [value …]`
//! - `position fen <fen>`
//! - `go [depth N] [movetime N]` → waits for `bestmove`
//!
//! Any other engine output (`id`, `option`, `info`, vendor-specific lines, …)
//! is ignored. That keeps the crate dependency-free and tolerant of engines
//! that do not strictly follow the protocol.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::fmt::Write as _;
use std::io::{self, BufReader, Write as _};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

mod error;
mod utils;

pub use error::EngineError;

/// A running UCI engine process.
///
/// Dropping the value sends `quit`, terminates the process and reaps it, so no
/// engine process is left behind.
#[derive(Debug)]
pub struct Engine {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl Engine {
    /// Starts `command` as a UCI engine, capturing its stdin and stdout.
    ///
    /// The caller sets the program, arguments and (optionally) stderr on
    /// `command` before calling this function.
    pub fn spawn(mut command: Command) -> Result<Self, EngineError> {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let input = child
            .stdin
            .take()
            .ok_or_else(|| EngineError::Io(io::Error::other("engine stdin was not captured")))?;
        let output = child
            .stdout
            .take()
            .ok_or_else(|| EngineError::Io(io::Error::other("engine stdout was not captured")))?;

        Ok(Self {
            child,
            input,
            output: BufReader::new(output),
        })
    }

    /// Performs the UCI handshake: sends `uci` and waits for `uciok`.
    ///
    /// Every line sent before `uciok` is ignored.
    pub fn handshake(&mut self) -> Result<(), EngineError> {
        self.send("uci")?;
        utils::read_until_uciok(&mut self.output)
    }

    /// Sends a `setoption` command.
    pub fn set_option(&mut self, name: &str, value: Option<&str>) -> Result<(), EngineError> {
        match value {
            Some(value) => self.send(&format!("setoption name {name} value {value}")),
            None => self.send(&format!("setoption name {name}")),
        }
    }

    /// Sends `position fen <fen>`.
    pub fn position_fen(&mut self, fen: &str) -> Result<(), EngineError> {
        self.send(&format!("position fen {fen}"))
    }

    /// Sends `go` and waits for `bestmove`.
    ///
    /// Every line sent before `bestmove` is ignored. Returns `None` when the
    /// engine reports that it has no move (`bestmove (none)` or a bare
    /// `bestmove`).
    pub fn go(&mut self, options: Go) -> Result<Option<BestMove>, EngineError> {
        self.send(&options.to_command())?;
        utils::read_best_move(&mut self.output)
    }

    fn send(&mut self, line: &str) -> Result<(), EngineError> {
        writeln!(self.input, "{line}")?;
        self.input.flush()?;
        Ok(())
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        // Ask politely, then make sure the process cannot outlive us.
        let _ = writeln!(self.input, "quit");
        let _ = self.input.flush();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Options for the UCI `go` command.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Go {
    /// Search depth in plies.
    pub depth: Option<u32>,
    /// Search time limit in milliseconds.
    pub move_time: Option<u64>,
}

impl Go {
    fn to_command(self) -> String {
        let mut command = String::from("go");

        if let Some(depth) = self.depth {
            let _ = write!(command, " depth {depth}");
        }

        if let Some(move_time) = self.move_time {
            let _ = write!(command, " movetime {move_time}");
        }

        command
    }
}

/// The move chosen by the engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BestMove {
    /// The move in coordinate notation, e.g. `e2e4`.
    pub movement: String,
    /// The move the engine would like to ponder on, if any.
    pub ponder: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn go_command_only_includes_requested_limits() {
        assert_eq!(Go::default().to_command(), "go");
        assert_eq!(
            Go {
                depth: Some(10),
                move_time: None,
            }
            .to_command(),
            "go depth 10"
        );
        assert_eq!(
            Go {
                depth: None,
                move_time: Some(500),
            }
            .to_command(),
            "go movetime 500"
        );
        assert_eq!(
            Go {
                depth: Some(20),
                move_time: Some(12_000),
            }
            .to_command(),
            "go depth 20 movetime 12000"
        );
    }

    #[test]
    fn handshake_ignores_engine_banner_and_options() {
        let mut output = Cursor::new(
            b"Stockfish 16\n\
              id name Stockfish 16\n\
              option name Threads type spin default 1 min 1 max 1024\n\
              uciok\n",
        );

        utils::read_until_uciok(&mut output).expect("handshake should succeed");
    }

    #[test]
    fn handshake_reports_eof() {
        let mut output = Cursor::new(b"id name Stockfish 16\n");

        assert!(matches!(
            utils::read_until_uciok(&mut output),
            Err(EngineError::UnexpectedEof)
        ));
    }

    #[test]
    fn best_move_ignores_info_lines() {
        let mut output = Cursor::new(
            b"info depth 10 score cp 20 pv e2e4 e7e5\n\
              info depth 11 score cp 25 pv e2e4 e7e5 g1f3\n\
              bestmove e2e4 ponder e7e5\n",
        );

        assert_eq!(
            utils::read_best_move(&mut output).expect("bestmove should be read"),
            Some(BestMove {
                movement: "e2e4".to_owned(),
                ponder: Some("e7e5".to_owned()),
            })
        );
    }

    #[test]
    fn best_move_without_ponder_is_parsed() {
        let mut output = Cursor::new(b"bestmove d2d4\n");

        assert_eq!(
            utils::read_best_move(&mut output).expect("bestmove should be read"),
            Some(BestMove {
                movement: "d2d4".to_owned(),
                ponder: None,
            })
        );
    }

    #[test]
    fn no_move_is_reported_as_none() {
        for input in [&b"bestmove (none)\n"[..], b"bestmove\n"] {
            let mut output = Cursor::new(input);
            assert_eq!(
                utils::read_best_move(&mut output).expect("bestmove should be read"),
                None
            );
        }
    }

    #[test]
    fn best_move_reports_eof() {
        let mut output = Cursor::new(b"info depth 10\n");

        assert!(matches!(
            utils::read_best_move(&mut output),
            Err(EngineError::UnexpectedEof)
        ));
    }
}
