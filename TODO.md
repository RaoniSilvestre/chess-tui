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

- [x] Moved board rendering into `chess-tui-ui` behind the `BoardView` view model.
- [x] Removed the temporary `src/ui` re-exports of `chess-tui-ui`.
- [x] Moved game rendering, promotion, and end-screen popups into `chess-tui-pages`.
- [ ] Move the remaining page-specific popups (move input, new game, resignation, puzzle end) into `chess-tui-pages`.

## Pages migration

- [x] Move Settings, GameModeMenu, LichessMenu, OngoingGames, and PgnViewer off `LegacyPage`.
- [x] Move game pages (Solo, Bot, Multiplayer, Lichess) into `chess-tui-pages`.
- [x] Move page-local state (home/settings/game-mode/Lichess/PGN cursors and game view state) into page structs.
- [x] Remove `LegacyPage` and the central legacy page dispatch.
- [ ] Replace the remaining `Pages`/`Popups` state enums with page-owned navigation results.
- [ ] Remove the temporary per-page context traits once the concrete `App` moves into `chess-tui-app`; they are scaffolding, not the target API.

## Validation

- [ ] Live smoke-test the Lichess and TCP multiplayer paths.
- [x] `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, and `cargo build --release` pass.
