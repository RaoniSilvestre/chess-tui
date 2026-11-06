//! PGN viewer board synchronization.

use crate::App;
use chess_tui_core::PgnViewer;

/// Sync the viewer's current ply into the shared game board.
pub fn sync_viewer_to_board(app: &mut App, games: &[PgnViewer], game_idx: usize) {
    let Some(viewer) = games.get(game_idx) else {
        return;
    };

    let (positions, moves) = viewer.current_snapshot();
    let current_ply = viewer.current_ply();
    let (positions, moves) = (positions.to_vec(), moves.to_vec());

    let taken_pieces = app.game.runtime.taken_pieces().to_vec();
    let halfmove_clock = app.game.runtime.halfmove_clock();
    if let Err(e) = app
        .game
        .runtime
        .load_history(positions, moves, taken_pieces, halfmove_clock)
    {
        log::error!("Failed to sync PGN viewer to board: {e}");
        return;
    }
    app.game.ui.selected_square = None;
    app.game.runtime.start_playing();
    app.game.ui.hide_cursor = true;
    app.game.runtime.set_turn(if current_ply % 2 == 0 {
        shakmaty::Color::White
    } else {
        shakmaty::Color::Black
    });
}
