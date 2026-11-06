//! Engine-backed bot mode.

use crate::{Bot, EndScreenKind, GameEvent, GameKind, GameMode, GameSession};
use shakmaty::{Color, Move};
use std::sync::mpsc::{Receiver, channel};

pub struct BotMode {
    bot: Bot,
    bot_color: Color,
    human_color: Color,
    receiver: Option<Receiver<Result<Move, String>>>,
}

impl BotMode {
    pub fn new(bot: Bot) -> Self {
        let bot_color = if bot.is_starting() {
            Color::White
        } else {
            Color::Black
        };
        Self {
            bot,
            bot_color,
            human_color: bot_color.other(),
            receiver: None,
        }
    }

    pub fn bot(&self) -> &Bot {
        &self.bot
    }

    pub fn bot_color(&self) -> Color {
        self.bot_color
    }

    pub fn human_color(&self) -> Color {
        self.human_color
    }

    fn start_thinking(&mut self, session: &GameSession) {
        if self.receiver.is_some() {
            return;
        }

        let fen = session.fen();
        let bot = self.bot.clone();
        let (tx, rx) = channel();
        self.receiver = Some(rx);

        std::thread::spawn(move || {
            let uci_move = match bot.find_move(&fen) {
                Ok(m) => m,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
            };

            let position: Option<shakmaty::Chess> = shakmaty::fen::Fen::from_ascii(fen.as_bytes())
                .ok()
                .and_then(|fen| fen.into_position(shakmaty::CastlingMode::Standard).ok());

            if let Some(position) = position
                && let Ok(chess_move) = uci_move.to_move(&position)
            {
                let _ = tx.send(Ok(chess_move));
            }
        });
    }

    fn apply_bot_move(&mut self, session: &mut GameSession, bot_move: Move) {
        let Some(from) = bot_move.from() else {
            return;
        };
        let to = bot_move.to();
        let role = bot_move.role();
        let capture = bot_move.capture();
        let promotion = bot_move.promotion();

        let played = session.play_and_record(from, to, promotion, |_| Move::Normal {
            role,
            from,
            capture,
            to,
            promotion,
        });

        if played.is_some() {
            session.switch_turn();
            session.update_game_state();
        }
    }
}

impl GameMode for BotMode {
    fn kind(&self) -> GameKind {
        GameKind::Bot
    }

    fn can_player_move(&self, session: &GameSession) -> bool {
        session.turn() == self.human_color
            && !session.state().is_finished()
            && !session.state().is_promotion()
    }

    fn on_player_move(&mut self, session: &mut GameSession, events: &mut Vec<GameEvent>) {
        if session.state().is_finished() || session.state().is_promotion() {
            return;
        }
        if session.turn() == self.bot_color {
            self.start_thinking(session);
        }
        let _ = events;
    }

    fn tick(&mut self, session: &mut GameSession, events: &mut Vec<GameEvent>) {
        if session.state().is_finished() || session.state().is_promotion() {
            return;
        }

        if let Some(rx) = &self.receiver {
            match rx.try_recv() {
                Ok(Ok(bot_move)) => {
                    self.receiver = None;
                    self.apply_bot_move(session, bot_move);
                    events.push(GameEvent::MovePlayed);
                }
                Ok(Err(error)) => {
                    self.receiver = None;
                    events.push(GameEvent::Error(error));
                }
                Err(_) => {}
            }
        } else if session.turn() == self.bot_color {
            self.start_thinking(session);
        }
    }

    fn shows_last_move_from(&self, color: shakmaty::Color) -> bool {
        color == self.bot_color
    }

    fn end_screen(&self) -> EndScreenKind {
        EndScreenKind::Standard
    }
}
