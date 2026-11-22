//! Ask a UCI engine for its best move.
//!
//! Usage:
//! `cargo run -p chess-tui-uci --example best_move -- [engine] [fen]`

use chess_tui_uci::{Engine, Go};
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let program = args.next().unwrap_or_else(|| "stockfish".to_owned());
    let fen = args
        .next()
        .unwrap_or_else(|| "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_owned());

    let mut engine = Engine::spawn(Command::new(program))?;
    engine.handshake()?;
    engine.position_fen(&fen)?;

    let best_move = engine.go(Go {
        depth: Some(10),
        move_time: None,
    })?;

    match best_move {
        Some(best_move) => println!(
            "bestmove {} (ponder: {:?})",
            best_move.movement, best_move.ponder
        ),
        None => println!("engine reported no move"),
    }

    Ok(())
}
