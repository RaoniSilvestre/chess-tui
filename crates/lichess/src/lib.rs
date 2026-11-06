//! Lichess API client, models, and game streaming.

pub const LICHESS_API_URL: &str = "https://lichess.org/api";

mod account;
mod game;
mod models;
mod puzzle;
mod stream;

pub use models::*;
