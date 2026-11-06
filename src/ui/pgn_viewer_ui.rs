//! PGN viewer board synchronization.
//!
//! The viewer page itself lives in `chess-tui-pages`; this module keeps the
//! bridge that loads a viewer snapshot into the shared game board.

use crate::app::App;
use chess_tui_core::PgnViewer;

/// Sync the viewer's current ply into `app.game.logic` so that the existing
/// board and history renderers see the right position.
pub fn sync_viewer_to_board(app: &mut App, games: &[PgnViewer], game_idx: usize) {
    let Some(viewer) = games.get(game_idx) else {
        return;
    };

    let (positions, moves) = viewer.current_snapshot();
    let current_ply = viewer.current_ply();
    let (positions, moves) = (positions.to_vec(), moves.to_vec());

    let taken_pieces = app.game.logic.taken_pieces().to_vec();
    let halfmove_clock = app.game.logic.halfmove_clock();
    if let Err(e) = app
        .game
        .logic
        .load_history(positions, moves, taken_pieces, halfmove_clock)
    {
        log::error!("Failed to sync PGN viewer to board: {e}");
        return;
    }
    app.game.ui.selected_square = None;
    app.game.logic.start_playing();
    // Hide the square cursor - viewer is read-only, nothing to select.
    app.game.ui.hide_cursor = true;
    // Set player_turn so history panel colouring is correct
    app.game.logic.set_turn(if current_ply % 2 == 0 {
        shakmaty::Color::White
    } else {
        shakmaty::Color::Black
    });
}

/// Sync the compatibility viewer fields held by [`App`] into the board.
pub fn sync_pgn_to_board(app: &mut App) {
    let Some(games) = app.pgn_viewer_state.clone() else {
        return;
    };
    let game_idx = app.pgn_viewer_game_idx;
    sync_viewer_to_board(app, &games, game_idx);
}
