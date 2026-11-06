//! Lichess game mode: board stream, clock sync, and remote moves.

use crate::{EndScreenKind, GameEvent, GameKind, GameMode, GameSession};
use chess_tui_core::{DrawReason, GameOutcome};
use chess_tui_lichess::LichessClient;
use shakmaty::{Color, Move, Role, Square};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::time::Instant;

pub struct LichessMode {
    color: Color,
    move_tx: Sender<String>,
    event_rx: Receiver<String>,
    initial_move_count: usize,
    moves_received: usize,
    closed: bool,
}

impl LichessMode {
    pub fn new(
        client: LichessClient,
        game_id: String,
        color: Color,
        initial_move_count: usize,
        immediate_last_move: Option<String>,
    ) -> Result<Self, String> {
        let (event_tx, event_rx) = channel::<String>();

        if let Some(last_move) = immediate_last_move {
            let _ = event_tx.send(last_move);
            if initial_move_count > 0 {
                let _ = event_tx.send(format!("INIT_MOVES:{}", initial_move_count));
            }
        }

        client
            .stream_game(game_id.clone(), event_tx, Some(color))
            .map_err(|e| e.to_string())?;

        let (move_tx, move_rx) = channel::<String>();
        let client_clone = client.clone();
        std::thread::spawn(move || {
            while let Ok(move_str) = move_rx.recv() {
                if let Err(e) = client_clone.make_move(&game_id, &move_str) {
                    log::error!("Failed to make move on Lichess: {}", e);
                }
            }
        });

        Ok(Self {
            color,
            move_tx,
            event_rx,
            initial_move_count,
            moves_received: 0,
            closed: false,
        })
    }

    pub fn color(&self) -> Color {
        self.color
    }

    fn handle_message(
        &mut self,
        session: &mut GameSession,
        message: String,
        events: &mut Vec<GameEvent>,
    ) {
        if self.handle_control_message(session, &message) {
            return;
        }

        self.moves_received += 1;

        if self.handle_historical_move(session, &message) {
            return;
        }

        if self.process_live_move(session, &message) {
            session.switch_turn();
            session.update_game_state();
            events.push(GameEvent::MovePlayed);
        }
    }

    fn handle_control_message(&mut self, session: &mut GameSession, message: &str) -> bool {
        if let Some(count_str) = message.strip_prefix("INIT_MOVES:")
            && let Ok(count) = count_str.parse::<usize>()
        {
            self.initial_move_count = count;
            if self.moves_received < count {
                self.moves_received = count;
            }
            return true;
        }

        if let Some(payload) = message.strip_prefix("CLOCK_SYNC:") {
            let parts: Vec<&str> = payload.split(':').collect();
            if let [white, black, color] = parts.as_slice() {
                let white_ms = white.parse::<u64>().unwrap_or(0);
                let black_ms = black.parse::<u64>().unwrap_or(0);
                let active_color = if *color == "white" {
                    Color::White
                } else {
                    Color::Black
                };
                session.clock_mut().sync_from_server(
                    white_ms,
                    black_ms,
                    active_color,
                    Instant::now(),
                );
            }
            return true;
        }

        if let Some(status) = message.strip_prefix("GAME_STATUS:") {
            let (status, winner) = match status.split_once(':') {
                Some((status, winner)) => (
                    status,
                    match winner {
                        "white" => Some(Color::White),
                        "black" => Some(Color::Black),
                        _ => None,
                    },
                ),
                None => (status, None),
            };
            match status {
                "checkmate" => {
                    session.update_game_state();
                    if !session.state().is_finished() {
                        session.finish(GameOutcome::Checkmate {
                            winner: winner.unwrap_or_else(|| session.turn().other()),
                        });
                    }
                }
                "draw" => {
                    session.finish(GameOutcome::Draw {
                        reason: DrawReason::Agreement,
                    });
                }
                "stalemate" => {
                    session.finish(GameOutcome::Draw {
                        reason: DrawReason::Stalemate,
                    });
                }
                "repetition" => {
                    session.finish(GameOutcome::Draw {
                        reason: DrawReason::Repetition,
                    });
                }
                "insufficient" => {
                    session.finish(GameOutcome::Draw {
                        reason: DrawReason::InsufficientMaterial,
                    });
                }
                "fifty" => {
                    session.finish(GameOutcome::Draw {
                        reason: DrawReason::FiftyMove,
                    });
                }
                "resign" => {
                    session.finish(GameOutcome::Resignation { winner });
                }
                "aborted" => {
                    session.finish(GameOutcome::Aborted);
                }
                other => log::warn!("Unknown Lichess game status: {}", other),
            }
            return true;
        }

        false
    }

    fn handle_historical_move(&mut self, session: &mut GameSession, move_str: &str) -> bool {
        let history_is_empty = session.moves().is_empty();

        let move_already_in_history = if !history_is_empty {
            if let Some((from, to, _)) = Self::parse_move(move_str) {
                session
                    .moves()
                    .iter()
                    .any(|m| m.from() == Some(from) && m.to() == to)
            } else {
                false
            }
        } else {
            false
        };

        let is_historical = if history_is_empty {
            self.initial_move_count > 0
        } else {
            self.moves_received <= self.initial_move_count || move_already_in_history
        };

        if !is_historical {
            return false;
        }

        if !history_is_empty {
            return true;
        }

        if session.positions().is_empty() {
            session.ensure_initial_position();
        }

        let Some((from, to, promotion)) = Self::parse_move(move_str) else {
            log::warn!("Failed to parse historical Lichess move: {}", move_str);
            return true;
        };

        let piece_type_from = session.get_role_at_square(&from);
        let executed = session.play_and_record(from, to, promotion, |executed| {
            if let Some(piece_type) = piece_type_from {
                Move::Normal {
                    role: piece_type,
                    from,
                    capture: executed.capture(),
                    to,
                    promotion: executed.promotion(),
                }
            } else {
                executed.clone()
            }
        });

        if executed.is_none() {
            log::warn!("Could not replay historical Lichess move: {}", move_str);
        }

        true
    }

    fn process_live_move(&mut self, session: &mut GameSession, move_str: &str) -> bool {
        let Some((from, to, promotion)) = Self::parse_move(move_str) else {
            log::error!("Failed to parse Lichess move: {}", move_str);
            return false;
        };

        let piece_type_from = session.get_role_at_square(&from);

        if session
            .moves()
            .iter()
            .any(|m| m.from() == Some(from) && m.to() == to)
        {
            log::warn!("Move {} already in history, skipping duplicate", move_str);
            return false;
        }

        let history_is_empty = session.moves().is_empty();
        if history_is_empty && self.moves_received <= self.initial_move_count {
            return self.handle_forced_historical_move(
                session,
                from,
                to,
                promotion,
                piece_type_from,
            );
        }

        let executed = session.play_and_record(from, to, promotion, |executed| {
            if let Some(piece_type) = piece_type_from {
                Move::Normal {
                    role: piece_type,
                    from,
                    capture: executed.capture(),
                    to,
                    promotion: executed.promotion(),
                }
            } else {
                executed.clone()
            }
        });

        if executed.is_some() {
            true
        } else {
            log::warn!("Failed to execute Lichess move: {}", move_str);
            false
        }
    }

    fn handle_forced_historical_move(
        &mut self,
        session: &mut GameSession,
        from: Square,
        to: Square,
        promotion: Option<Role>,
        piece_type_from: Option<Role>,
    ) -> bool {
        let piece_type = piece_type_from
            .or_else(|| session.get_role_at_square(&to))
            .unwrap_or(Role::Pawn);

        let executed = session.play_and_record(from, to, promotion, |executed| Move::Normal {
            role: piece_type,
            from,
            capture: executed.capture(),
            to,
            promotion: executed.promotion(),
        });

        if executed.is_some() {
            return false;
        }

        log::warn!("Could not replay forced historical Lichess move: {from}{to}");
        false
    }

    fn parse_move(move_str: &str) -> Option<(Square, Square, Option<Role>)> {
        if move_str.len() < 4 {
            return None;
        }

        let mut chars = move_str.chars();
        let from_file = chars.next()?;
        let from_rank = chars.next()?;
        let to_file = chars.next()?;
        let to_rank = chars.next()?;

        let promotion = if move_str.chars().count() == 5 {
            match move_str.chars().nth(4) {
                Some('q') => Some(Role::Queen),
                Some('r') => Some(Role::Rook),
                Some('b') => Some(Role::Bishop),
                Some('n') => Some(Role::Knight),
                _ => None,
            }
        } else {
            None
        };

        let from = Square::from_ascii(format!("{from_file}{from_rank}").as_bytes()).ok()?;
        let to = Square::from_ascii(format!("{to_file}{to_rank}").as_bytes()).ok()?;
        Some((from, to, promotion))
    }

    fn send_last_move(&self, session: &GameSession) {
        if let Some(last_move) = session.moves().last() {
            let from = last_move.from().map(|s| s.to_string()).unwrap_or_default();
            let to = last_move.to().to_string();
            let promotion = match last_move.promotion() {
                Some(Role::Queen) => "q",
                Some(Role::Rook) => "r",
                Some(Role::Bishop) => "b",
                Some(Role::Knight) => "n",
                _ => "",
            };
            let _ = self.move_tx.send(format!("{from}{to}{promotion}"));
        }
    }
}

impl GameMode for LichessMode {
    fn kind(&self) -> GameKind {
        GameKind::Lichess
    }

    fn can_player_move(&self, session: &GameSession) -> bool {
        session.turn() == self.color
            && !session.state().is_finished()
            && !session.state().is_promotion()
    }

    fn on_player_move(&mut self, session: &mut GameSession, _events: &mut Vec<GameEvent>) {
        self.send_last_move(session);
    }

    fn on_promotion(&mut self, session: &mut GameSession, _events: &mut Vec<GameEvent>) {
        self.send_last_move(session);
    }

    fn tick(&mut self, session: &mut GameSession, events: &mut Vec<GameEvent>) {
        if self.closed {
            return;
        }

        loop {
            match self.event_rx.try_recv() {
                Ok(message) => self.handle_message(session, message, events),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.closed = true;
                    events.push(GameEvent::Error("Lichess game stream ended".to_string()));
                    break;
                }
            }
        }
    }

    fn end_screen(&self) -> EndScreenKind {
        EndScreenKind::Lichess
    }
}
