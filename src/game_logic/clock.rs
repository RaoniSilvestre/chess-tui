//! Chess clock for both players.

use shakmaty::Color;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum ClockState {
    #[default]
    NotStarted,
    Running {
        active_color: Color,
        turn_start: Instant,
    },
    Paused {
        active_color: Color,
    },
    TimeUp {
        loser_color: Color,
    },
}

#[derive(Clone, Debug)]
pub struct Clock {
    /// Time remaining for White (in seconds)
    white_time: Duration,
    /// Time remaining for Black (in seconds)
    black_time: Duration,
    state: ClockState,
}

impl Default for Clock {
    fn default() -> Self {
        let time_per_player = Duration::from_mins(10);
        Self {
            white_time: time_per_player,
            black_time: time_per_player,
            state: ClockState::NotStarted,
        }
    }
}

impl Clock {
    /// Create a new clock with the specified time per player (in seconds)
    pub fn new(seconds: u32) -> Self {
        let time_per_player = Duration::from_secs(seconds as u64);
        Self {
            white_time: time_per_player,
            black_time: time_per_player,
            state: ClockState::NotStarted,
        }
    }

    /// Start the clock for the given color
    pub fn start(&mut self, now: Instant) {
        if let ClockState::NotStarted = self.state {
            self.state = ClockState::Running {
                active_color: Color::White,
                turn_start: now,
            };
        }
    }

    pub fn resume(&mut self, now: Instant) {
        if let ClockState::Paused { active_color } = self.state {
            self.state = ClockState::Running {
                active_color,
                turn_start: now,
            };
        }
    }

    pub fn pause(&mut self, now: Instant) {
        if let ClockState::Running {
            active_color,
            turn_start,
        } = self.state
        {
            let elapsed = now.saturating_duration_since(turn_start);
            self.deduct_time(active_color, elapsed);

            self.state = ClockState::Paused { active_color }
        }
    }

    /// Troca o turno, processando o desconto de tempo do jogador atual
    pub fn switch_turn(&mut self, now: Instant) {
        if let ClockState::Running {
            active_color,
            turn_start,
        } = self.state
        {
            let elapsed = now.saturating_duration_since(turn_start);
            self.deduct_time(active_color, elapsed);

            if self.is_time_up(active_color, now) {
                self.state = ClockState::TimeUp {
                    loser_color: active_color,
                };
                return;
            }

            self.state = ClockState::Running {
                active_color: !active_color,
                turn_start: now,
            };
        }
    }

    pub fn is_time_up(&self, color: Color, instant: Instant) -> bool {
        self.get_time(color, instant) == Duration::ZERO
    }

    pub fn get_time(&self, color: Color, now: Instant) -> Duration {
        let base_time = self.get_base_time(color);

        if let ClockState::Running {
            active_color,
            turn_start,
        } = self.state
            && active_color == color
        {
            let elapsed = now.saturating_duration_since(turn_start);
            return base_time.saturating_sub(elapsed);
        }
        base_time
    }

    pub fn format_time(&self, color: Color, now: Instant) -> String {
        let time = self.get_time(color, now);
        let total_secs = time.as_secs();
        let millis = time.subsec_millis();
        let minutes = total_secs / 60;
        let seconds = total_secs % 60;

        if minutes > 0 {
            // Over 1 minute: show MM:SS without milliseconds
            format!("{:02}:{:02}", minutes, seconds)
        } else {
            // Under 1 minute: show SS.mmm with milliseconds
            format!("{:02}.{:03}", seconds, millis)
        }
    }

    fn deduct_time(&mut self, color: Color, amount: Duration) {
        match color {
            Color::White => self.white_time = self.white_time.saturating_sub(amount),
            Color::Black => self.black_time = self.black_time.saturating_sub(amount),
        }
    }

    fn get_base_time(&self, color: Color) -> Duration {
        match color {
            Color::White => self.white_time,
            Color::Black => self.black_time,
        }
    }

    pub fn white_time(&self) -> Duration {
        self.white_time
    }

    pub fn black_time(&self) -> Duration {
        self.black_time
    }

    pub fn state(&self) -> ClockState {
        self.state
    }
}
