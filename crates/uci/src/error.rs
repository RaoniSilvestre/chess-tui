use std::fmt;
use std::io::{self};

/// An error returned by [`Engine`].
#[derive(Debug)]
pub enum EngineError {
    /// An I/O error occurred while talking to the engine.
    Io(io::Error),
    /// The engine closed its output before sending the expected message.
    UnexpectedEof,
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "engine I/O error: {error}"),
            Self::UnexpectedEof => f.write_str("engine closed its output unexpectedly"),
        }
    }
}

impl std::error::Error for EngineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::UnexpectedEof => None,
        }
    }
}

impl From<io::Error> for EngineError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
