use chess_tui_core::Coord;
use chess_tui_core::square_utils::{
    flip_square_if_needed, get_coord_from_square, get_square_from_coord,
};
use shakmaty::Square;

#[test]
fn test_flip_square_if_needed_unflipped() {
    assert_eq!(flip_square_if_needed(Square::E4, false), Square::E4);
    assert_eq!(flip_square_if_needed(Square::A1, false), Square::A1);
    assert_eq!(flip_square_if_needed(Square::H8, false), Square::H8);
}

#[test]
fn test_flip_square_if_needed_flipped() {
    assert_eq!(flip_square_if_needed(Square::A1, true), Square::H8);
    assert_eq!(flip_square_if_needed(Square::H8, true), Square::A1);
    assert_eq!(flip_square_if_needed(Square::E4, true), Square::D5);
}

#[test]
fn test_get_square_from_coord_unflipped() {
    assert_eq!(get_square_from_coord(Coord::new(7, 0), false), Square::A1);
    assert_eq!(get_square_from_coord(Coord::new(0, 7), false), Square::H8);
    assert_eq!(get_square_from_coord(Coord::new(4, 4), false), Square::E4);
}

#[test]
fn test_get_square_from_coord_flipped() {
    // Flipped: coord is mirrored before conversion
    assert_eq!(get_square_from_coord(Coord::new(7, 0), true), Square::H8);
    assert_eq!(get_square_from_coord(Coord::new(0, 7), true), Square::A1);
}

#[test]
fn test_get_coord_from_square_unflipped() {
    assert_eq!(get_coord_from_square(Square::A1, false), Coord::new(7, 0));
    assert_eq!(get_coord_from_square(Square::H8, false), Coord::new(0, 7));
    assert_eq!(get_coord_from_square(Square::E4, false), Coord::new(4, 4));
}

#[test]
fn test_get_coord_from_square_flipped() {
    // Flipped: square coord is reversed
    assert_eq!(get_coord_from_square(Square::A1, true), Coord::new(0, 7));
    assert_eq!(get_coord_from_square(Square::H8, true), Coord::new(7, 0));
}

#[test]
fn test_normalize_lowercase_to_san() {
    // possible lower case first letters
    let moves = ["ne4", "be4", "re4", "qe4", "ke4"];
    let expected = ["Ne4", "Be4", "Re4", "Qe4", "Ke4"];

    for (&input, &expected) in moves.iter().zip(expected.iter()) {
        assert_eq!(
            chess_tui_core::square_utils::normalize_lowercase_to_san(input),
            expected
        );
    }

    // check for castling
    let moves = ["o-o", "o-o-o"];
    let expected = ["O-O", "O-O-O"];
    for (&input, &expected) in moves.iter().zip(expected.iter()) {
        assert_eq!(
            chess_tui_core::square_utils::normalize_lowercase_to_san(input),
            expected
        );
    }

    // check for promotion
    let moves = ["e8=q", "e8=r", "e8=b", "e8=n"];
    let expected = ["e8=Q", "e8=R", "e8=B", "e8=N"];
    for (&input, &expected) in moves.iter().zip(expected.iter()) {
        assert_eq!(
            chess_tui_core::square_utils::normalize_lowercase_to_san(input),
            expected
        );
    }

    // check empty string
    assert_eq!(
        chess_tui_core::square_utils::normalize_lowercase_to_san("                       "),
        ""
    );
    assert_eq!(
        chess_tui_core::square_utils::normalize_lowercase_to_san("      kh4             "),
        "Kh4"
    );
}
