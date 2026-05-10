use chess_tui::game_logic::clock::{Clock, ClockState};
use shakmaty::Color;
use std::time::{Duration, Instant};

#[test]
fn test_new_clock_initial_time() {
    let clock = Clock::new(300);
    let duration = Duration::from_secs(300);
    let instant = Instant::now();

    assert_eq!(clock.get_time(Color::White, instant), duration);
    assert_eq!(clock.get_time(Color::Black, instant), duration);
    assert_eq!(clock.state(), ClockState::NotStarted);
}

#[test]
fn test_default_clock_is_ten_minutes() {
    let clock = Clock::default();
    let instant = Instant::now();
    let duration = Duration::from_secs(600);
    assert_eq!(clock.get_time(Color::White, instant), duration);
    assert_eq!(clock.get_time(Color::Black, instant), duration);
}

#[test]
fn test_start_sets_running_state() {
    let mut clock = Clock::new(300);
    let instant = Instant::now();
    clock.start(instant);
    assert_eq!(
        ClockState::Running {
            active_color: Color::White,
            turn_start: instant
        },
        clock.state()
    );
}

#[test]
fn test_stop_clears_running_state() {
    let mut clock = Clock::new(300);
    let instant = Instant::now();
    clock.start(instant);
    clock.pause(instant + Duration::from_secs(10));
    assert_eq!(
        ClockState::Paused {
            active_color: Color::White
        },
        clock.state()
    );
    assert_eq!(clock.white_time(), Duration::from_secs(290));
}

#[test]
fn test_stop_without_start_does_change_state() {
    let mut clock = Clock::new(300);
    let instant = Instant::now();
    clock.pause(instant);
    assert_eq!(ClockState::NotStarted, clock.state());
}

#[test]
fn test_start_switches_active_color() {
    let mut clock = Clock::new(300);
    let instant = Instant::now();
    let instant_plus_hundred_secs = instant + Duration::from_secs(100);

    let whites_turn = ClockState::Running {
        active_color: Color::White,
        turn_start: instant,
    };

    let blacks_turn = ClockState::Running {
        active_color: Color::Black,
        turn_start: instant_plus_hundred_secs,
    };

    clock.start(instant);
    assert_eq!(clock.state(), whites_turn);

    clock.switch_turn(instant_plus_hundred_secs);

    assert_eq!(clock.state(), blacks_turn);
    assert_eq!(clock.white_time(), Duration::from_secs(200));
    assert_eq!(clock.black_time(), Duration::from_secs(300));
}

#[test]
fn test_get_time_returns_full_time_when_stopped() {
    let clock = Clock::new(300);
    let duration = Duration::from_secs(300);
    assert_eq!(clock.white_time(), duration);
    assert_eq!(clock.black_time(), duration);
}

#[test]
fn test_is_time_up_false_with_time_remaining() {
    let mut clock = Clock::new(300);

    let i = Instant::now();
    clock.start(i);
    let i = i + Duration::from_secs(10);
    clock.switch_turn(i);
    let i = i + Duration::from_secs(10);
    clock.switch_turn(i);

    assert!(!clock.is_time_up(Color::White, i));
    assert!(!clock.is_time_up(Color::Black, i));
}
#[test]
fn test_is_time_up_true_with_zero_duration() {
    let mut clock = Clock::new(0);

    let i = Instant::now();
    clock.start(i);

    assert!(clock.is_time_up(Color::White, i));
    assert!(clock.is_time_up(Color::Black, i));
}

#[test]
fn test_format_time_over_one_minute() {
    let clock = Clock::new(300); // 5:00
    let i = Instant::now();
    assert_eq!(clock.format_time(Color::White, i), "05:00");
    assert_eq!(clock.format_time(Color::Black, i), "05:00");
}

#[test]
fn test_format_time_exactly_one_minute() {
    let clock = Clock::new(60);
    let i = Instant::now();
    assert_eq!(clock.format_time(Color::White, i), "01:00");
}

#[test]
fn test_format_time_under_one_minute() {
    let clock = Clock::new(45);
    let i = Instant::now();
    assert_eq!(clock.format_time(Color::White, i), "45.000");
}

#[test]
fn test_format_time_zero() {
    let clock = Clock::new(0);
    let i = Instant::now();
    assert_eq!(clock.format_time(Color::White, i), "00.000");
}

#[test]
fn test_format_time_mixed_minutes_seconds() {
    let clock = Clock::new(65); // 1:05
    let i = Instant::now();
    assert_eq!(clock.format_time(Color::White, i), "01:05");
}
