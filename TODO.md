# TODO

## Deferred game-board quirks

- [ ] `GameBoard::reset()` clears moves/positions but leaves `taken_pieces` untouched. Decide whether `reset()` should clear captured pieces or whether a separate `reset_position()` should preserve them.
- [ ] Undoing a move (`undo_last_move`, used by promotion and puzzle reset paths) pops the move and position but does not restore the captured piece. Fix once the new `GameSession`/mode refactor settles.

## Temporary API exceptions

- [ ] Remove or hide `GameBoard::record_move_for_draw_rule` once `GameSession` owns move application and the fifty-move counter.
- [ ] Remove `GameBoard::record_recovery_move` once Lichess historical replay moves into `LichessMode`.

## Temporary migration bridges

- [ ] Remove `GameSession`'s `Deref`/`DerefMut` to `GameBoard` once call sites use `board()` and session methods directly.
- [ ] Collapse the remaining `GameLogic` wrapper into `GameRuntime` now that bot/opponent orchestration lives in modes.

## Game-state follow-ups

- [ ] Parse the Lichess `winner` field for resignation events and populate `GameOutcome::Resignation { winner: Some(...) }`.
- [ ] Add a `DrawReason` if the UI needs to distinguish stalemate, fifty-move, repetition, insufficient material, and agreement.

## UI migration follow-ups

- [ ] Move board rendering from `src/game_logic/ui.rs` into `chess-tui-ui` using a `BoardView` view model once `BoardViewState` lives in the game page.
- [ ] Remove the temporary `src/ui` re-exports of `chess-tui-ui` once pages import the UI crate directly.
- [ ] Move page-specific menu/popup renderers into `chess-tui-pages`.

## Pages migration

- [x] Move Settings, GameModeMenu, LichessMenu, OngoingGames, and PgnViewer off `LegacyPage`.
- [ ] Move game pages (Solo, Bot, Multiplayer, Lichess) into `chess-tui-pages` with `GameRuntime` + `BoardViewState`.
- [ ] Move page-local state out of `App`/`UIState` into page structs.
- [ ] Remove the temporary per-page context traits (`SettingsContext`, `GameModeContext`, `LichessMenuContext`, `OngoingGamesContext`, `PgnViewerContext`) once pages own their services/state directly. They are scaffolding, not the target API.
- [ ] Remove `LegacyPage`, `Pages`, and `Popups` dispatch.
