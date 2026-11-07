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
- [x] Remove the temporary per-page context traits.
- [x] Add the `page` crate for `Page`, `PageAction`, `PageContext`, `Router`, and `PageFactory`.
- [x] Remove the `pages` crate's dependency on `app`; pages now return actions instead of mutating `App`.
- [x] Move page-local state into pages (Home, Settings, GameModeMenu, LichessMenu, OngoingGames, PgnViewer, GamePage).
- [x] Split the generic game page into `LocalGamePage`, `BotGamePage`, `MultiplayerGamePage`, and `LichessGamePage` with a shared `GameBoard` component.
- [x] Give game pages ownership of the game handle and board view state.
- [x] Split `App` into composition/dispatch, navigation, action reducer, and modal helper modules.
- [x] Move all domain state out of `App` into a `Services` container; `App` now only owns router, page factory, services, lifecycle, and pending page.

## Page ownership refactor

- [x] Delete the `page` crate and move `Page`, `PageAction`, `PageRequest`, and `create_page` into `pages`.
- [x] Remove `Services`, `GameHandle`, and all shared page/domain state; each page owns its own game and runtime state.
- [x] Move persistent settings into the new `config` crate (`Args`, `Config`, `ConfigStore`, paths).
- [x] Reduce `App` to `running`, the active page, and `Globals`.
- [x] Keep only truly app-wide state in `Globals` (config store, loaded theme, sound/animations/kitty, global overlays).
- [x] Restore Bot/Multiplayer setup via page lifecycle hooks and add regression tests.
- [x] Move page-specific prompts/modals into their page structs.
- [x] Remove the dead `game_state.rs` duplicate, the duplicate app runtime loop, and the old action reducer.

## Validation

- [ ] Live smoke-test the Lichess and TCP multiplayer paths.
- [x] `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo check --workspace --no-default-features`, and `cargo build --release` pass.
