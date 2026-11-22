# chess-tui-uci

A deliberately small [UCI](https://backscattering.de/chess/uci) client for
[chess-tui](https://github.com/thomas-mauran/chess-tui).

It implements only the subset of the protocol chess-tui needs to make a move:

- `uci` → waits for `uciok`
- `setoption name … [value …]`
- `position fen <fen>`
- `go [depth N] [movetime N]` → waits for `bestmove`

Everything else an engine sends (`id`, `option`, `info`, unknown or
vendor-specific lines) is ignored. That keeps the crate dependency-free and
makes it tolerant of engines that do not strictly follow the protocol.

## Example

```rust,no_run
use chess_tui_uci::{Engine, Go};
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = Engine::spawn(Command::new("stockfish"))?;
    engine.handshake()?;
    engine.set_option("UCI_Elo", Some("1500"))?;
    engine.position_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")?;

    let best = engine.go(Go { depth: Some(10), ..Go::default() })?;
    println!("{:?}", best.map(|m| m.r#move));

    Ok(())
}
```
