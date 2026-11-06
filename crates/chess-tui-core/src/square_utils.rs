//! Board coordinate helpers shared by the domain and UI layers.

use crate::coord::Coord;
use shakmaty::Square;

/// Returns the mirror square when the board is flipped, otherwise the original square.
#[must_use]
pub fn flip_square_if_needed(square: Square, is_flipped: bool) -> Square {
    if is_flipped {
        square.rotate_180()
    } else {
        square
    }
}

/// Converts a [`Coord`] to a [`Square`], mirroring it when the board is flipped.
#[must_use]
pub fn get_square_from_coord(coord: Coord, is_flipped: bool) -> Square {
    if is_flipped { coord.reverse() } else { coord }.into()
}

/// Converts a [`Square`] to a [`Coord`], mirroring it when the board is flipped.
#[must_use]
pub fn get_coord_from_square(square: Square, is_flipped: bool) -> Coord {
    if is_flipped {
        Coord::from(square).reverse()
    } else {
        Coord::from(square)
    }
}

/// Normalize lower case SAN into valid uppercase SAN.
#[must_use]
pub fn normalize_lowercase_to_san(input: &str) -> String {
    let s0 = input.trim();

    if s0.is_empty() {
        return String::new();
    }

    // ---- Castling normalization (accept common variants) ----
    let lower = s0.to_ascii_lowercase();
    match lower.as_str() {
        "o-o" | "0-0" => return "O-O".to_string(),
        "o-o-o" | "0-0-0" => return "O-O-O".to_string(),
        _ => {}
    }

    let mut s = s0.to_string();

    // ---- Uppercase leading piece letter (only first char) ----
    if let Some(first) = s.chars().next() {
        let up = match first {
            'n' => Some('N'),
            'b' => Some('B'),
            'r' => Some('R'),
            'q' => Some('Q'),
            'k' => Some('K'),
            _ => None, // pawn moves like "e4" should stay lowercase
        };

        if let Some(up) = up {
            s.replace_range(0..first.len_utf8(), &up.to_string());
        }
    }

    // ---- Uppercase promotion piece after '=' (e8=q -> e8=Q) ----
    if let Some(eq) = s.find('=') {
        // SAN promotion piece is a single ASCII letter right after '='
        let bytes = s.as_bytes();
        if eq + 1 < bytes.len() {
            let promo = bytes[eq + 1] as char;
            let up = match promo {
                'q' => Some('Q'),
                'r' => Some('R'),
                'b' => Some('B'),
                'n' => Some('N'),
                _ => None,
            };
            if let Some(up) = up {
                s.replace_range(eq + 1..eq + 2, &up.to_string());
            }
        }
    }

    s
}
