//! High-level state of a chess game.

use shakmaty::{Color, Square};

/// The phase and outcome of a game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameState {
    Playing,
    Promotion(PendingPromotion),
    Finished(GameOutcome),
}

/// A promotion move waiting for the player to choose a piece.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingPromotion {
    from: Square,
    to: Square,
}

impl PendingPromotion {
    pub fn new(from: Square, to: Square) -> Self {
        Self { from, to }
    }

    pub fn from(self) -> Square {
        self.from
    }

    pub fn to(self) -> Square {
        self.to
    }
}

/// Why a game ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameOutcome {
    Checkmate { winner: Color },
    Draw,
    Timeout { winner: Color },
    Resignation { winner: Option<Color> },
    Aborted,
}

impl GameOutcome {
    pub fn winner(self) -> Option<Color> {
        match self {
            GameOutcome::Checkmate { winner } | GameOutcome::Timeout { winner } => Some(winner),
            GameOutcome::Resignation { winner } => winner,
            GameOutcome::Draw | GameOutcome::Aborted => None,
        }
    }

    pub fn is_draw(self) -> bool {
        matches!(self, GameOutcome::Draw)
    }
}

impl GameState {
    pub fn is_playing(self) -> bool {
        matches!(self, GameState::Playing)
    }

    pub fn is_promotion(self) -> bool {
        matches!(self, GameState::Promotion(_))
    }

    pub fn is_finished(self) -> bool {
        matches!(self, GameState::Finished(_))
    }

    pub fn outcome(self) -> Option<GameOutcome> {
        match self {
            GameState::Finished(outcome) => Some(outcome),
            _ => None,
        }
    }

    pub fn promotion(self) -> Option<PendingPromotion> {
        match self {
            GameState::Promotion(promotion) => Some(promotion),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shakmaty::Square;

    #[test]
    fn promotion_carries_its_squares() {
        let promotion = PendingPromotion::new(Square::E7, Square::E8);
        let state = GameState::Promotion(promotion);
        assert!(state.is_promotion());
        assert_eq!(state.promotion(), Some(promotion));
        assert_eq!(promotion.from(), Square::E7);
        assert_eq!(promotion.to(), Square::E8);
    }

    #[test]
    fn finished_state_carries_the_outcome() {
        let outcome = GameOutcome::Timeout {
            winner: Color::White,
        };
        let state = GameState::Finished(outcome);
        assert!(state.is_finished());
        assert_eq!(state.outcome(), Some(outcome));
        assert_eq!(outcome.winner(), Some(Color::White));
    }

    #[test]
    fn draw_has_no_winner() {
        assert_eq!(GameOutcome::Draw.winner(), None);
        assert!(GameOutcome::Draw.is_draw());
    }

    #[test]
    fn resignation_may_have_no_winner() {
        assert_eq!(GameOutcome::Resignation { winner: None }.winner(), None);
        assert_eq!(
            GameOutcome::Resignation {
                winner: Some(Color::Black)
            }
            .winner(),
            Some(Color::Black)
        );
    }
}
