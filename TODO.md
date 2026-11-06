# TODO

## Deferred game-board quirks

- [x] `Board::reset()` now clears captured pieces as well as moves/positions.
- [x] `Board::undo_last_move()` now restores the captured piece, including en passant.

## Temporary API exceptions

- [x] `Board::record_move_for_draw_rule` is private; `Board::play` updates the fifty-move counter automatically.
- [x] Removed `Board::record_recovery_move`; unreplayable Lichess history moves are logged instead of creating inconsistent history.

## Temporary migration bridges

- [x] Removed `GameSession`'s `Deref`/`DerefMut` to `Board`; call sites use session forwarding methods or `board()`.
- [x] Collapsed `GameLogic` into `GameRuntime`.

## Game-state follow-ups

- [x] Parse the Lichess `winner` field for resignation/checkmate events.
- [x] Added `DrawReason` for stalemate, fifty-move, repetition, insufficient material, agreement, and unknown draws.

## UI migration follow-ups

- [x] Moved board rendering into `ui` behind the `BoardView` view model.
- [x] Removed the temporary `src/ui` re-exports of `ui`.
- [x] Moved game rendering and all page-specific popups into `pages`.

## Pages migration

- [x] Move Settings, GameModeMenu, LichessMenu, OngoingGames, and PgnViewer off `LegacyPage`.
- [x] Move game pages (Solo, Bot, Multiplayer, Lichess) into `pages`.
- [x] Move page-local state into page structs and the concrete `App` into `app`.
- [x] Remove `LegacyPage`, the `Pages` routing enum, and the central page dispatch.
- [x] Replace the `Popups` enum with modal objects owned by the application.
- [x] Remove the temporary per-page context traits; pages receive `&mut App` directly.

## Validation

- [ ] Live smoke-test the Lichess and TCP multiplayer paths.
- [x] `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo check --no-default-features`, and `cargo build --release` pass.
