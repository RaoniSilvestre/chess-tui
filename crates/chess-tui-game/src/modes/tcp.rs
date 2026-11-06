//! TCP peer-to-peer mode.

use crate::{EndScreenKind, GameEvent, GameKind, GameMode, GameSession};
use chess_tui_net::TcpPeer;
use shakmaty::{Color, Move, Role, Square};

pub struct TcpMode {
    peer: TcpPeer,
    my_color: Color,
    opponent_color: Color,
    started: bool,
    closed: bool,
}

impl TcpMode {
    /// `opponent_color` is `Some` for the host and `None` for the joining player.
    pub fn new(addr: &str, opponent_color: Option<Color>) -> Result<Self, String> {
        let must_wait_for_start = opponent_color.is_some();
        let peer = TcpPeer::connect(addr, opponent_color)?;
        let opponent_color = peer.color();
        let my_color = opponent_color.other();
        Ok(Self {
            peer,
            my_color,
            opponent_color,
            started: !must_wait_for_start,
            closed: false,
        })
    }

    pub fn my_color(&self) -> Color {
        self.my_color
    }

    pub fn opponent_color(&self) -> Color {
        self.opponent_color
    }

    fn apply_opponent_move(&mut self, session: &mut GameSession, move_str: &str) -> bool {
        let Some((from, to, promotion)) = Self::parse_move(move_str) else {
            log::error!("Failed to parse TCP move: {}", move_str);
            return false;
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

        if executed.is_some() {
            session.switch_turn();
            session.update_game_state();
            true
        } else {
            log::warn!("Failed to execute TCP move: {}", move_str);
            false
        }
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

    fn send_last_move(&mut self, session: &GameSession) {
        if let Some(last_move) = session.moves().last()
            && let Err(e) = self.peer.send_move(last_move)
        {
            log::warn!("Failed to send TCP move: {}", e);
        }
    }
}

impl GameMode for TcpMode {
    fn kind(&self) -> GameKind {
        GameKind::Tcp
    }

    fn is_ready(&self) -> bool {
        self.started
    }

    fn can_player_move(&self, session: &GameSession) -> bool {
        self.started
            && session.turn() == self.my_color
            && !session.state().is_finished()
            && !session.state().is_promotion()
    }

    fn on_player_move(&mut self, session: &mut GameSession, _events: &mut Vec<GameEvent>) {
        if self.started {
            self.send_last_move(session);
        }
    }

    fn on_promotion(&mut self, session: &mut GameSession, _events: &mut Vec<GameEvent>) {
        if self.started {
            self.send_last_move(session);
        }
    }

    fn tick(&mut self, session: &mut GameSession, events: &mut Vec<GameEvent>) {
        if self.closed {
            return;
        }

        if !self.started {
            match self.peer.wait_for_start() {
                Ok(true) => {
                    self.started = true;
                    events.push(GameEvent::OpponentJoined);
                }
                Ok(false) => {}
                Err(e) => {
                    self.closed = true;
                    events.push(GameEvent::Error(e));
                }
            }
            return;
        }

        loop {
            match self.peer.try_read_move() {
                Ok(Some(move_str)) => {
                    if self.apply_opponent_move(session, &move_str) {
                        events.push(GameEvent::MovePlayed);
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    self.closed = true;
                    events.push(GameEvent::Error(e));
                    break;
                }
            }
        }
    }

    fn end_screen(&self) -> EndScreenKind {
        EndScreenKind::Standard
    }

    fn disconnect(&mut self) {
        self.peer.send_end();
        self.closed = true;
    }
}
