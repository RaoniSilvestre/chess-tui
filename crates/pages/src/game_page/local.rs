//! Local two-player game page. Owns its board, clock, and resume save.

use crate::page::PageKind;
use model::square_utils::normalize_lowercase_to_san;
use model::{Clock, GameState};
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{Frame, layout::Rect};
use shakmaty::san::San;
use ui::prompt::Prompt;

use crate::game_mode::MatchConfig;
use crate::globals::Globals;
use crate::page::{Page, PageAction, PageRequest};
use crate::resume::{self, ResumeMode, ResumeTracker};
use ui::prompt::PromptResult;

use super::board::{BoardAction, GameBoard};

pub struct LocalGamePage {
    board: GameBoard,
    config: MatchConfig,
    resume_requested: bool,
    resume: ResumeTracker,
    confirm_restart: bool,
    move_input: Option<Prompt>,
}

impl LocalGamePage {
    pub fn new(config: MatchConfig, resume: bool, _globals: &mut Globals) -> Self {
        Self {
            board: GameBoard::new(),
            config,
            resume_requested: resume,
            resume: ResumeTracker::default(),
            confirm_restart: false,
            move_input: None,
        }
    }

    fn load_resume(&mut self, globals: &mut Globals) -> bool {
        let Some(saved) = resume::load(ResumeMode::Local) else {
            globals.show_error("No saved game to resume.");
            return false;
        };
        if let Err(e) = resume::restore_position(&saved, &mut self.board.game) {
            globals.show_error(e);
            resume::delete(ResumeMode::Local);
            return false;
        }
        if let Some((cursor, custom)) = resume::restore_clock(&saved, &mut self.board.game) {
            self.config.clock_cursor = cursor;
            self.config.custom_time_minutes = custom;
        }
        globals.theme.apply_to_game(&mut self.board.game);
        true
    }

    pub fn game(&self) -> &session::Game {
        self.board.game()
    }

    pub fn game_mut(&mut self) -> &mut session::Game {
        self.board.game_mut()
    }

    pub fn autosave(&mut self) {
        self.resume.autosave(
            ResumeMode::Local,
            &mut self.board.game,
            self.config.clock_cursor,
            self.config.custom_time_minutes,
            None,
            false,
        );
    }

    pub fn flush(&mut self) {
        self.resume.flush(
            ResumeMode::Local,
            &mut self.board.game,
            self.config.clock_cursor,
            self.config.custom_time_minutes,
            None,
        );
    }

    pub fn tick_resume(&mut self) {
        self.resume.tick(
            ResumeMode::Local,
            &mut self.board.game,
            self.config.clock_cursor,
            self.config.custom_time_minutes,
            None,
        );
    }

    fn apply_config(&mut self, globals: &mut Globals) {
        self.board
            .game
            .runtime
            .set_clock(Clock::new(self.config.time_control));
        globals.theme.apply_to_game(&mut self.board.game);
    }

    pub fn restart(&mut self, globals: &mut Globals) {
        self.resume.clear(ResumeMode::Local);
        self.board = GameBoard::new();
        self.apply_config(globals);
        self.confirm_restart = false;
        self.move_input = None;
    }

    fn handle_board_action(&mut self, action: BoardAction, globals: &mut Globals) -> PageAction {
        match action {
            BoardAction::None => PageAction::None,
            BoardAction::Page(action) => action,
            BoardAction::Restart => {
                if resume::has_save(ResumeMode::Local) {
                    self.confirm_restart = true;
                } else {
                    self.restart(globals);
                }
                PageAction::None
            }
            BoardAction::OpenMoveInput => {
                let mut prompt = Prompt::new();
                prompt.message = "Enter a move (e.g. e4, Nf3, O-O):".to_string();
                self.move_input = Some(prompt);
                PageAction::None
            }
            BoardAction::PuzzleMove { .. } => PageAction::None,
        }
    }

    fn handle_move_input(&mut self, key: KeyEvent, globals: &mut Globals) -> PageAction {
        let Some(prompt) = self.move_input.as_mut() else {
            return PageAction::None;
        };
        match ui::prompt::handle_key(prompt, key) {
            PromptResult::Cancel => {
                self.move_input = None;
                PageAction::None
            }
            PromptResult::Submit => {
                let raw = prompt.input.trim().to_string();
                if raw.is_empty() {
                    self.move_input = None;
                    return PageAction::None;
                }
                let san = match San::from_ascii(normalize_lowercase_to_san(&raw).as_bytes()) {
                    Ok(san) => san,
                    Err(_) => {
                        prompt.error = Some("Invalid notation".into());
                        return PageAction::None;
                    }
                };
                let position = self.board.game.runtime.current_position().clone();
                let Ok(chess_move) = san.to_move(&position) else {
                    prompt.error = Some("Illegal move".into());
                    return PageAction::None;
                };
                let action = self.board.submit_move(&chess_move);
                if let Some(prompt) = self.move_input.as_mut() {
                    prompt.input.clear();
                    prompt.character_index = 0;
                    prompt.error = None;
                }
                self.handle_board_action(action, globals)
            }
            PromptResult::None => PageAction::None,
        }
    }
}

impl Page for LocalGamePage {
    fn kind(&self) -> PageKind {
        PageKind::LocalGame
    }
    fn is_modal_active(&self) -> bool {
        self.move_input.is_some() || self.confirm_restart
    }

    fn on_enter(&mut self, globals: &mut Globals) -> PageAction {
        if self.resume_requested {
            if !self.load_resume(globals) {
                return PageAction::Navigate(PageRequest::GameModeMenu);
            }
        } else {
            self.apply_config(globals);
        }
        PageAction::None
    }

    fn on_exit(&mut self, _globals: &mut Globals) {
        if self.board.game.runtime.state().is_playing() {
            self.flush();
        }
    }

    fn handle_key(&mut self, key: KeyEvent, globals: &mut Globals) -> PageAction {
        if self.move_input.is_some() {
            return self.handle_move_input(key, globals);
        }
        if self.confirm_restart {
            match key.code {
                KeyCode::Char('y' | 'Y') => {
                    self.restart(globals);
                }
                KeyCode::Char('n' | 'N') | KeyCode::Esc => {
                    self.confirm_restart = false;
                }
                _ => {}
            }
            return PageAction::None;
        }
        if self.board.end_screen_visible {
            let action = self.board.handle_end_screen_key(key);
            return self.handle_board_action(action, globals);
        }
        match key.code {
            KeyCode::Char('r') => {
                if resume::has_save(ResumeMode::Local) {
                    self.confirm_restart = true;
                } else {
                    self.restart(globals);
                }
                PageAction::None
            }
            KeyCode::Char('b') => {
                self.flush();
                PageAction::Navigate(crate::page::PageRequest::Home)
            }
            KeyCode::Char('n' | 'N') => {
                self.board.navigate_history_next();
                PageAction::None
            }
            KeyCode::Char('p' | 'P') => {
                self.board.navigate_history_previous();
                PageAction::None
            }
            _ => {
                let action = self.board.handle_common_key(key, globals);
                self.handle_board_action(action, globals)
            }
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, globals: &mut Globals) -> PageAction {
        let action = self.board.handle_mouse(mouse);
        self.handle_board_action(action, globals)
    }

    fn tick(&mut self, _globals: &mut Globals) -> PageAction {
        self.board.tick();
        if self.board.game.runtime.state().is_finished() {
            self.resume.clear(ResumeMode::Local);
        }
        PageAction::None
    }

    fn update(&mut self, _globals: &mut Globals) -> PageAction {
        let finished = self.board.update();
        if finished {
            self.resume.clear(ResumeMode::Local);
        } else if self.board.game.runtime.state() == GameState::Playing {
            self.tick_resume();
        }
        PageAction::None
    }

    fn help_context(&self) -> (bool, bool) {
        (true, false)
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, globals: &mut Globals) {
        self.board.render(frame, area, globals);
        if self.confirm_restart {
            ui::overlay::new_game::render_new_game_confirmation_popup(frame, true);
        }
        if let Some(prompt) = &self.move_input {
            ui::prompt::render_move_input_popup(frame, prompt);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::PageRequest;
    use crate::resume::{
        ClockState, ResumeMode, SavedGame, TakenPieceRecord, has_save, save, save_path,
    };
    use crate::test_support::{ResumeDirGuard, send, test_globals};
    use model::{Clock, GameOutcome, GameState, TimeControl};
    use ratatui::crossterm::event::KeyCode;
    use shakmaty::Color;

    fn started_local() -> (LocalGamePage, Globals) {
        let mut globals = test_globals();
        let mut page = LocalGamePage::new(MatchConfig::default(), false, &mut globals);
        assert!(matches!(page.on_enter(&mut globals), PageAction::None));
        (page, globals)
    }

    fn resumed_local() -> (LocalGamePage, Globals) {
        let mut globals = test_globals();
        let mut page = LocalGamePage::new(MatchConfig::default(), true, &mut globals);
        assert!(matches!(page.on_enter(&mut globals), PageAction::None));
        (page, globals)
    }

    fn play_e2e4(page: &mut LocalGamePage, globals: &mut Globals) {
        send(
            page,
            globals,
            &[
                KeyCode::Down,
                KeyCode::Down,
                KeyCode::Enter,
                KeyCode::Up,
                KeyCode::Up,
                KeyCode::Enter,
            ],
        );
    }

    fn saved_after_e5() -> SavedGame {
        SavedGame {
            mode: ResumeMode::Local,
            fen: "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2".into(),
            is_flipped: false,
            taken_pieces: vec![],
            bot: None,
            clock: None,
            moves_uci: vec!["e2e4".into(), "e7e5".into()],
        }
    }

    fn read_save() -> SavedGame {
        let path = save_path(ResumeMode::Local).expect("save path");
        serde_json::from_str(&std::fs::read_to_string(path).expect("save body")).expect("save json")
    }

    #[test]
    fn valid_first_move_e2_to_e4() {
        let (mut page, mut globals) = started_local();
        play_e2e4(&mut page, &mut globals);

        assert!(page.board.game.ui.selected_square.is_none());
        assert_eq!(page.board.game.runtime.moves().len(), 1);
        assert_eq!(page.board.game.runtime.turn(), Color::Black);
    }

    #[test]
    fn invalid_first_move_is_rejected() {
        let (mut page, mut globals) = started_local();
        send(
            &mut page,
            &mut globals,
            &[
                KeyCode::Down,
                KeyCode::Down,
                KeyCode::Enter,
                KeyCode::Up,
                KeyCode::Up,
                KeyCode::Up,
                KeyCode::Enter,
            ],
        );
        assert_eq!(page.board.game.runtime.moves().len(), 0);
        assert_eq!(page.board.game.runtime.turn(), Color::White);
    }

    #[test]
    fn m_opens_move_input_popup_and_esc_closes() {
        let (mut page, mut globals) = started_local();
        send(&mut page, &mut globals, &[KeyCode::Char('m')]);
        assert!(page.move_input.is_some());

        send(&mut page, &mut globals, &[KeyCode::Esc]);
        assert!(page.move_input.is_none());

        let before = page.board.game.ui.cursor_coordinates;
        send(&mut page, &mut globals, &[KeyCode::Down]);
        assert_ne!(page.board.game.ui.cursor_coordinates, before);
    }

    #[test]
    fn m_then_e4_enter_makes_move_and_keeps_popup_open() {
        let (mut page, mut globals) = started_local();
        send(
            &mut page,
            &mut globals,
            &[
                KeyCode::Char('m'),
                KeyCode::Char('e'),
                KeyCode::Char('4'),
                KeyCode::Enter,
            ],
        );

        let prompt = page.move_input.as_ref().expect("popup stays open");
        assert_eq!(prompt.input, "");
        assert!(prompt.error.is_none());
        assert_eq!(page.board.game.runtime.turn(), Color::Black);
        assert_eq!(
            page.board
                .game
                .runtime
                .get_role_at_square(&shakmaty::Square::E4),
            Some(shakmaty::Role::Pawn)
        );
    }

    #[test]
    fn invalid_san_keeps_popup_open_and_sets_error() {
        let (mut page, mut globals) = started_local();
        send(
            &mut page,
            &mut globals,
            &[
                KeyCode::Char('m'),
                KeyCode::Char('z'),
                KeyCode::Char('z'),
                KeyCode::Char('z'),
                KeyCode::Enter,
            ],
        );
        let prompt = page.move_input.as_ref().expect("popup stays open");
        assert!(prompt.error.is_some());
        assert_eq!(prompt.input, "zzz");
        assert_eq!(page.board.game.runtime.moves().len(), 0);
    }

    #[test]
    fn illegal_san_keeps_popup_open_and_sets_error() {
        let (mut page, mut globals) = started_local();
        send(
            &mut page,
            &mut globals,
            &[
                KeyCode::Char('m'),
                KeyCode::Char('e'),
                KeyCode::Char('5'),
                KeyCode::Enter,
            ],
        );
        let prompt = page.move_input.as_ref().expect("popup stays open");
        assert!(prompt.error.is_some());
        assert_eq!(page.board.game.runtime.turn(), Color::White);
        assert!(
            page.board
                .game
                .runtime
                .get_role_at_square(&shakmaty::Square::E5)
                .is_none()
        );
    }

    #[test]
    fn typing_after_error_clears_it() {
        let (mut page, mut globals) = started_local();
        send(
            &mut page,
            &mut globals,
            &[
                KeyCode::Char('m'),
                KeyCode::Char('z'),
                KeyCode::Char('z'),
                KeyCode::Char('z'),
                KeyCode::Enter,
            ],
        );
        assert!(page.move_input.as_ref().unwrap().error.is_some());
        send(&mut page, &mut globals, &[KeyCode::Backspace]);
        assert!(page.move_input.as_ref().unwrap().error.is_none());
    }

    #[test]
    fn deselect_piece_with_keyboard() {
        let (mut page, mut globals) = started_local();
        send(
            &mut page,
            &mut globals,
            &[KeyCode::Down, KeyCode::Down, KeyCode::Enter],
        );
        assert!(page.board.game.ui.selected_square.is_some());
        send(&mut page, &mut globals, &[KeyCode::Enter]);
        assert!(page.board.game.ui.selected_square.is_none());
        assert_eq!(page.board.game.runtime.moves().len(), 0);
    }

    #[test]
    fn autosave_writes_after_first_move() {
        let _guard = ResumeDirGuard::new();
        let (mut page, mut globals) = started_local();
        assert!(!has_save(ResumeMode::Local));

        play_e2e4(&mut page, &mut globals);
        page.autosave();

        let saved = read_save();
        assert_eq!(saved.mode, ResumeMode::Local);
        assert!(saved.fen.starts_with("rnbqkbnr/pppppppp/8/8/4P3/"));
        assert!(saved.bot.is_none());
    }

    #[test]
    fn autosave_is_skipped_before_any_move() {
        let _guard = ResumeDirGuard::new();
        let (mut page, _globals) = started_local();
        page.autosave();
        assert!(!has_save(ResumeMode::Local));
    }

    #[test]
    fn resume_from_saved_rebuilds_position() {
        let _guard = ResumeDirGuard::new();
        save(ResumeMode::Local, &saved_after_e5());

        let (page, _globals) = resumed_local();
        assert_eq!(page.board.game.runtime.state(), GameState::Playing);
        assert_eq!(page.board.game.runtime.turn(), Color::White);
        assert!(
            page.board
                .game
                .runtime
                .fen()
                .starts_with("rnbqkbnr/pppp1ppp/8/4p3/4P3/")
        );
    }

    #[test]
    fn resume_from_missing_save_shows_error() {
        let _guard = ResumeDirGuard::new();
        let mut globals = test_globals();
        let mut page = LocalGamePage::new(MatchConfig::default(), true, &mut globals);

        assert!(matches!(
            page.on_enter(&mut globals),
            PageAction::Navigate(PageRequest::GameModeMenu)
        ));
        assert!(matches!(
            globals.overlay,
            Some(crate::overlay::Overlay::Message { .. })
        ));
    }

    #[test]
    fn restart_mid_game_asks_first() {
        let _guard = ResumeDirGuard::new();
        let (mut page, mut globals) = started_local();
        play_e2e4(&mut page, &mut globals);
        page.autosave();
        assert!(has_save(ResumeMode::Local));

        send(&mut page, &mut globals, &[KeyCode::Char('r')]);
        assert!(page.confirm_restart);

        send(&mut page, &mut globals, &[KeyCode::Char('n')]);
        assert!(!page.confirm_restart);
        assert_eq!(page.board.game.runtime.moves().len(), 1);
        assert!(has_save(ResumeMode::Local));

        send(
            &mut page,
            &mut globals,
            &[KeyCode::Char('r'), KeyCode::Char('y')],
        );
        assert_eq!(page.board.game.runtime.moves().len(), 0);
        assert!(!has_save(ResumeMode::Local));
    }

    #[test]
    fn restart_before_any_move_does_not_ask() {
        let _guard = ResumeDirGuard::new();
        let (mut page, mut globals) = started_local();
        send(&mut page, &mut globals, &[KeyCode::Char('r')]);
        assert!(!page.confirm_restart);
    }

    #[test]
    fn resume_restores_saved_clock_state() {
        let _guard = ResumeDirGuard::new();
        let mut saved = saved_after_e5();
        saved.clock = Some(ClockState {
            white_ms: 123_456,
            black_ms: 234_567,
            clock_cursor: 4,
            custom_minutes: 7,
        });
        save(ResumeMode::Local, &saved);

        let (page, _globals) = resumed_local();
        let instant = std::time::Instant::now();
        let clock = page.board.game.runtime.clock().clone();
        let white = clock.get_time(Color::White, instant).as_millis() as u64;
        assert!(white <= 123_456);
        assert_eq!(
            clock.get_time(Color::Black, instant).as_millis() as u64,
            234_567
        );
        assert_eq!(page.config.clock_cursor, 4);
        assert_eq!(page.config.custom_time_minutes, 7);
    }

    #[test]
    fn flush_captures_clock_between_moves() {
        let _guard = ResumeDirGuard::new();
        let (mut page, _globals) = started_local();
        page.board
            .game
            .runtime
            .set_clock(Clock::new(TimeControl::Blitz));
        let mut globals = test_globals();
        play_e2e4(&mut page, &mut globals);
        page.autosave();
        let baseline = read_save().clock.expect("clock saved").black_ms;

        std::thread::sleep(std::time::Duration::from_millis(50));
        page.flush();
        let later = read_save().clock.expect("clock saved").black_ms;
        assert!(later + 10 <= baseline);
    }

    #[test]
    fn resume_rebuilds_full_move_history() {
        let _guard = ResumeDirGuard::new();
        let (mut page, mut globals) = started_local();
        play_e2e4(&mut page, &mut globals);
        page.autosave();

        let (fresh, _globals) = resumed_local();
        assert_eq!(fresh.board.game.runtime.moves().len(), 1);
        assert_eq!(fresh.board.game.runtime.positions().len(), 2);
    }

    #[test]
    fn move_count_guard_prevents_duplicate_writes() {
        let _guard = ResumeDirGuard::new();
        let (mut page, mut globals) = started_local();
        play_e2e4(&mut page, &mut globals);
        page.autosave();

        let path = save_path(ResumeMode::Local).expect("save path");
        let first = std::fs::metadata(&path).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(30));
        page.autosave();
        let second = std::fs::metadata(path).unwrap().modified().unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn resume_restores_flip_and_taken_pieces() {
        let _guard = ResumeDirGuard::new();
        let mut saved = saved_after_e5();
        saved.is_flipped = true;
        saved.taken_pieces = vec![
            TakenPieceRecord {
                color: "black".into(),
                role: "pawn".into(),
            },
            TakenPieceRecord {
                color: "white".into(),
                role: "knight".into(),
            },
        ];
        saved.moves_uci.clear();
        save(ResumeMode::Local, &saved);

        let (page, _globals) = resumed_local();
        assert!(page.board.game.runtime.is_flipped());
        assert_eq!(page.board.game.runtime.taken_pieces().len(), 2);
    }

    #[test]
    fn legacy_save_without_moves_uses_fen_directly() {
        let _guard = ResumeDirGuard::new();
        let mut saved = saved_after_e5();
        saved.moves_uci.clear();
        save(ResumeMode::Local, &saved);

        let (page, _globals) = resumed_local();
        assert!(page.board.game.runtime.moves().is_empty());
        assert!(
            page.board
                .game
                .runtime
                .fen()
                .starts_with("rnbqkbnr/pppp1ppp/8/4p3/4P3/")
        );
    }

    #[test]
    fn corrupt_fen_in_save_deletes_file_and_errors() {
        let _guard = ResumeDirGuard::new();
        let mut saved = saved_after_e5();
        saved.fen = "this is not a fen".into();
        save(ResumeMode::Local, &saved);
        assert!(has_save(ResumeMode::Local));

        let mut globals = test_globals();
        let mut page = LocalGamePage::new(MatchConfig::default(), true, &mut globals);
        assert!(matches!(
            page.on_enter(&mut globals),
            PageAction::Navigate(PageRequest::GameModeMenu)
        ));
        assert!(matches!(
            globals.overlay,
            Some(crate::overlay::Overlay::Message { .. })
        ));
        assert!(!has_save(ResumeMode::Local));
    }

    #[test]
    fn quit_flushes_clock_state() {
        let _guard = ResumeDirGuard::new();
        let (mut page, mut globals) = started_local();
        page.board
            .game
            .runtime
            .set_clock(Clock::new(TimeControl::Blitz));
        play_e2e4(&mut page, &mut globals);
        page.autosave();
        let baseline = read_save().clock.expect("clock saved").black_ms;

        std::thread::sleep(std::time::Duration::from_millis(50));
        page.on_exit(&mut globals);
        let later = read_save().clock.expect("clock saved").black_ms;
        assert!(later + 10 <= baseline);
        assert!(has_save(ResumeMode::Local));
    }

    #[test]
    fn tick_throttles_clock_only_writes() {
        let _guard = ResumeDirGuard::new();
        let (mut page, mut globals) = started_local();
        page.board
            .game
            .runtime
            .set_clock(Clock::new(TimeControl::Blitz));
        play_e2e4(&mut page, &mut globals);

        page.tick_resume();
        let path = save_path(ResumeMode::Local).expect("save path");
        let first = std::fs::metadata(&path).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(30));
        page.tick_resume();
        let second = std::fs::metadata(path).unwrap().modified().unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn player_checkmate_via_update_clears_save() {
        let _guard = ResumeDirGuard::new();
        let mut saved = saved_after_e5();
        saved.fen = "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3".into();
        saved.moves_uci.clear();
        save(ResumeMode::Local, &saved);

        let (mut page, mut globals) = resumed_local();
        assert!(has_save(ResumeMode::Local));
        page.update(&mut globals);

        assert!(matches!(
            page.board.game.runtime.state(),
            GameState::Finished(GameOutcome::Checkmate { .. })
        ));
        assert!(!has_save(ResumeMode::Local));
    }

    #[test]
    fn restart_clears_save_so_old_state_does_not_shadow_new_game() {
        let _guard = ResumeDirGuard::new();
        let (mut page, mut globals) = started_local();
        play_e2e4(&mut page, &mut globals);
        page.autosave();
        assert!(has_save(ResumeMode::Local));

        page.restart(&mut globals);
        assert!(!has_save(ResumeMode::Local));
    }

    #[test]
    fn clock_runout_clears_save_via_tick() {
        let _guard = ResumeDirGuard::new();
        let (mut page, mut globals) = started_local();
        page.board
            .game
            .runtime
            .set_clock(Clock::new(TimeControl::Custom(0)));
        play_e2e4(&mut page, &mut globals);
        page.autosave();
        assert!(has_save(ResumeMode::Local));

        page.tick(&mut globals);

        assert!(matches!(
            page.board.game.runtime.state(),
            GameState::Finished(GameOutcome::Timeout { .. })
        ));
        assert!(!has_save(ResumeMode::Local));
    }
}
