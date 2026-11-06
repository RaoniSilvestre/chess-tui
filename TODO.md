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
