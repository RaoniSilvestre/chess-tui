use crate::{BestMove, EngineError};
use std::io::BufRead;

pub(crate) fn read_until_uciok<R: BufRead>(output: &mut R) -> Result<(), EngineError> {
    let mut line = String::new();

    loop {
        read_line(output, &mut line)?;

        if line.trim() == "uciok" {
            return Ok(());
        }
    }
}

pub(crate) fn read_best_move<R: BufRead>(output: &mut R) -> Result<Option<BestMove>, EngineError> {
    let mut line = String::new();

    loop {
        read_line(output, &mut line)?;

        if line.split_whitespace().next() == Some("bestmove") {
            return Ok(parse_best_move(&line));
        }
    }
}

fn read_line<R: BufRead>(output: &mut R, line: &mut String) -> Result<(), EngineError> {
    line.clear();

    if output.read_line(line)? == 0 {
        return Err(EngineError::UnexpectedEof);
    }

    let len = line.trim_end_matches(['\r', '\n']).len();
    line.truncate(len);

    Ok(())
}

fn parse_best_move(line: &str) -> Option<BestMove> {
    let mut parts = line.split_whitespace();

    if parts.next() != Some("bestmove") {
        return None;
    }

    let r#move = parts.next()?;

    if r#move == "(none)" {
        return None;
    }

    let ponder = match (parts.next(), parts.next()) {
        (Some("ponder"), Some(ponder)) => Some(ponder.to_owned()),
        _ => None,
    };

    Some(BestMove {
        movement: r#move.to_owned(),
        ponder,
    })
}
