//! Bot game page. Owns the board and the runtime bot configuration.

use game::{Bot, BotMode};
use model::square_utils::normalize_lowercase_to_san;
use model::{Clock, GameState};
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{Frame, layout::Rect};
use shakmaty::Color;
use shakmaty::san::San;
use ui::prompt::Prompt;

use crate::constants::DOCS_URL;
use crate::game_mode::MatchConfig;
use crate::globals::{BotSettings, Globals};
use crate::page::{Page, PageAction, PageKind, PageRequest};
use crate::resume::{self, BotConfig, ResumeMode, ResumeTracker};
use ui::prompt::PromptResult;

use super::board::{BoardAction, GameBoard};

/// Runtime configuration for one bot game. The page owns this; persistent
/// defaults live in `config.toml`.
#[derive(Clone, Debug)]
pub struct BotGameConfig {
    pub settings: BotSettings,
    pub player_color: Color,
    pub random_color: bool,
}

impl BotGameConfig {
    fn to_save_config(&self) -> BotConfig {
        BotConfig {
            path: self.settings.engine_path.clone(),
            depth: self.settings.depth,
            difficulty: self.settings.difficulty,
            player_color: match self.player_color {
                Color::White => "white",
                Color::Black => "black",
            }
            .to_string(),
            is_random_color: self.random_color,
        }
    }
}

pub struct BotGamePage {
    board: GameBoard,
    config: MatchConfig,
    bot: BotGameConfig,
    resume_saved: bool,
    resume: ResumeTracker,
    confirm_restart: bool,
    move_input: Option<Prompt>,
}

impl BotGamePage {
    pub fn new(
        config: MatchConfig,
        bot: BotGameConfig,
        resume: bool,
        _globals: &mut Globals,
    ) -> Self {
        Self {
            board: GameBoard::new(),
            config,
            bot,
            resume_saved: resume,
            resume: ResumeTracker::default(),
            confirm_restart: false,
            move_input: None,
        }
    }

    pub fn game(&self) -> &session::Game {
        self.board.game()
    }

    pub fn game_mut(&mut self) -> &mut session::Game {
        self.board.game_mut()
    }

    pub fn autosave(&mut self) {
        let bot = self.bot.to_save_config();
        self.resume.autosave(
            ResumeMode::Bot,
            &mut self.board.game,
            self.config.clock_cursor,
            self.config.custom_time_minutes,
            Some(bot),
            false,
        );
    }

    pub fn flush(&mut self) {
        let bot = self.bot.to_save_config();
        self.resume.flush(
            ResumeMode::Bot,
            &mut self.board.game,
            self.config.clock_cursor,
            self.config.custom_time_minutes,
            Some(bot),
        );
    }

    pub fn tick_resume(&mut self) {
        let bot = self.bot.to_save_config();
        self.resume.tick(
            ResumeMode::Bot,
            &mut self.board.game,
            self.config.clock_cursor,
            self.config.custom_time_minutes,
            Some(bot),
        );
    }

    fn engine_error(&self) -> Option<String> {
        let path = self.bot.settings.engine_path.as_str();
        if path.is_empty() {
            return Some(format!(
                "Chess engine path not configured.\n\nTo configure the chess engine follow the documentation: {DOCS_URL}/Bot/bot-engines\n\nExample:\nchess-tui -e /opt/homebrew/opt/stockfish\n or execute the script: ./scripts/install-stockfish.sh\nto install stockfish automatically and set it as the chess engine path \n"
            ));
        }
        let command_path = path.split_whitespace().next().unwrap_or(path);
        if !std::path::Path::new(command_path).is_file() {
            return Some(format!(
                "Chess engine path is invalid.\n\nThe configured path does not exist or is not a file:\n{command_path}\n\nPlease check the path and update it in your configuration.\n\nTo configure the chess engine follow the documentation: {DOCS_URL}/Bot/bot-engines\n\nExample:\nchess-tui -e /opt/homebrew/bin/stockfish\nor execute the script: ./scripts/install-stockfish.sh\nto install stockfish automatically and set it as the chess engine path"
            ));
        }
        None
    }

    fn install_bot_mode(&mut self, starting: bool) {
        let bot = Bot::new(
            &self.bot.settings.engine_path,
            starting,
            self.bot.settings.depth,
            self.bot.settings.difficulty,
        );
        self.board
            .game
            .runtime
            .set_mode(Box::new(BotMode::new(bot)));
    }

    fn start_fresh(&mut self, globals: &mut Globals) {
        self.board = GameBoard::new();
        if self.bot.random_color {
            self.bot.player_color = self.config.resolve_color();
        }
        let bot_starting = self.bot.player_color == Color::Black;
        self.install_bot_mode(bot_starting);
        self.board
            .game
            .runtime
            .set_clock(Clock::new(self.config.time_control));
        if bot_starting {
            self.board.game.runtime.flip();
        }
        globals.theme.apply_to_game(&mut self.board.game);
    }

    fn load_resume(&mut self, globals: &mut Globals) -> bool {
        let Some(saved) = resume::load(ResumeMode::Bot) else {
            globals.show_error("No saved game to resume.");
            return false;
        };
        let Some(bot_cfg) = saved.bot.as_ref() else {
            globals.show_error("Saved bot game is missing its engine config.");
            return false;
        };
        if let Err(e) = resume::restore_position(&saved, &mut self.board.game) {
            globals.show_error(e);
            return false;
        }
        if let Some((cursor, custom)) = resume::restore_clock(&saved, &mut self.board.game) {
            self.config.clock_cursor = cursor;
            self.config.custom_time_minutes = custom;
        }
        self.bot.settings = BotSettings {
            engine_path: bot_cfg.path.clone(),
            depth: bot_cfg.depth,
            difficulty: bot_cfg.difficulty,
        };
        self.bot.player_color = match bot_cfg.player_color.as_str() {
            "black" => Color::Black,
            _ => Color::White,
        };
        self.bot.random_color = bot_cfg.is_random_color;
        let starting = self.board.game.runtime.turn() != self.bot.player_color;
        self.install_bot_mode(starting);
        globals.theme.apply_to_game(&mut self.board.game);
        true
    }

    pub fn restart(&mut self, globals: &mut Globals) {
        self.resume.clear(ResumeMode::Bot);
        self.start_fresh(globals);
        self.confirm_restart = false;
        self.move_input = None;
    }

    fn handle_board_action(&mut self, action: BoardAction, globals: &mut Globals) -> PageAction {
        match action {
            BoardAction::None => PageAction::None,
            BoardAction::Page(action) => action,
            BoardAction::Restart => {
                if resume::has_save(ResumeMode::Bot) {
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

impl Page for BotGamePage {
    fn kind(&self) -> PageKind {
        PageKind::BotGame
    }
    fn is_modal_active(&self) -> bool {
        self.move_input.is_some() || self.confirm_restart
    }

    fn on_enter(&mut self, globals: &mut Globals) -> PageAction {
        if self.resume_saved {
            if !self.load_resume(globals) {
                return PageAction::Navigate(PageRequest::GameModeMenu);
            }
        } else {
            if let Some(message) = self.engine_error() {
                globals.show_error_then(message, PageRequest::Home);
                return PageAction::None;
            }
            self.start_fresh(globals);
        }
        if let Some(message) = self.engine_error() {
            globals.show_error_then(message, PageRequest::Home);
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
                if resume::has_save(ResumeMode::Bot) {
                    self.confirm_restart = true;
                } else {
                    self.restart(globals);
                }
                PageAction::None
            }
            KeyCode::Char('b') => {
                self.flush();
                PageAction::Navigate(PageRequest::Home)
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
            self.resume.clear(ResumeMode::Bot);
        }
        PageAction::None
    }

    fn update(&mut self, _globals: &mut Globals) -> PageAction {
        let finished = self.board.update();
        if finished {
            self.resume.clear(ResumeMode::Bot);
        } else if self.board.game.runtime.state() == GameState::Playing {
            self.tick_resume();
        }
        PageAction::None
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
    use crate::test_support::{fake_engine_path, test_globals};

    fn bot_config(engine_path: String) -> BotGameConfig {
        BotGameConfig {
            settings: BotSettings {
                engine_path,
                depth: 1,
                difficulty: None,
            },
            player_color: Color::White,
            random_color: false,
        }
    }

    #[test]
    fn on_enter_initializes_bot_mode() {
        let mut globals = test_globals();
        let mut page = BotGamePage::new(
            MatchConfig::default(),
            bot_config(fake_engine_path()),
            false,
            &mut globals,
        );

        assert!(matches!(page.on_enter(&mut globals), PageAction::None));
        assert_eq!(page.board.game.runtime.kind(), game::GameKind::Bot);
        assert_ne!(
            page.board.game.runtime.clock().state(),
            model::ClockState::Disabled
        );
    }

    #[test]
    fn missing_engine_shows_error() {
        let mut globals = test_globals();
        let mut page = BotGamePage::new(
            MatchConfig::default(),
            bot_config(String::new()),
            false,
            &mut globals,
        );

        page.on_enter(&mut globals);
        assert!(matches!(
            globals.overlay,
            Some(crate::overlay::Overlay::Message { .. })
        ));
    }
}
