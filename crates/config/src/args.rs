//! Command-line arguments.

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path for the chess engine
    #[arg(short, long, default_value = "")]
    pub engine_path: String,
    /// Bot thinking depth for chess engine (1-255)
    #[arg(short, long)]
    pub depth: Option<u8>,
    /// Bot difficulty: easy, medium, hard, or magnus. Omit for full strength (Off).
    #[arg(long)]
    pub difficulty: Option<String>,
    /// Lichess API token
    #[arg(short, long)]
    pub lichess_token: Option<String>,
    /// Disable sound effects
    #[arg(long)]
    pub no_sound: bool,
    /// Skin/theme name (e.g. Default, ASCII). Overrides config for reproducible demos.
    #[arg(long)]
    pub skin: Option<String>,
    /// Update skin config with built-in default (prompts for confirmation, archives current file)
    #[arg(long)]
    pub update_skins: bool,
    /// Open a PGN file or directory of .pgn files directly in the viewer.
    #[arg(short = 'p', long)]
    pub pgn: Option<String>,
}
