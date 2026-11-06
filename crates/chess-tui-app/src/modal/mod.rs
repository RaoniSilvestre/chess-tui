//! Modal overlays.

pub mod render;

use crate::{App, GamePageKind, Modal, ModalResult};
use chess_tui_core::square_utils::normalize_lowercase_to_san;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::{Frame, layout::Rect};
use shakmaty::san::San;

pub struct HelpModal;

impl Modal for HelpModal {
    fn handle_key(&mut self, key: KeyEvent, _app: &mut App) -> ModalResult {
        match key.code {
            KeyCode::Char('?') | KeyCode::Esc => ModalResult::Close,
            _ => ModalResult::None,
        }
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, app: &App) {
        render::help::render_help_popup(frame, app);
        let _ = area;
    }
}

#[derive(Clone, Copy)]
enum MessageKind {
    Error,
    Success,
    Loading,
}

pub struct MessageModal {
    kind: MessageKind,
    message: String,
}

impl MessageModal {
    pub fn error(message: String) -> Self {
        Self {
            kind: MessageKind::Error,
            message,
        }
    }

    pub fn success(message: String) -> Self {
        Self {
            kind: MessageKind::Success,
            message,
        }
    }

    pub fn loading(message: String) -> Self {
        Self {
            kind: MessageKind::Loading,
            message,
        }
    }
}

impl Modal for MessageModal {
    fn handle_key(&mut self, key: KeyEvent, app: &mut App) -> ModalResult {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ') => {
                match self.kind {
                    MessageKind::Error => app.after_error_popup(),
                    MessageKind::Success => app.after_success_popup(),
                    MessageKind::Loading => {}
                }
                ModalResult::Close
            }
            KeyCode::Char('q') => {
                app.quit();
                ModalResult::None
            }
            _ => ModalResult::None,
        }
    }

    fn render(&mut self, frame: &mut Frame<'_>, _area: Rect, _app: &App) {
        match self.kind {
            MessageKind::Error => render::error::render_error_popup(frame, &self.message),
            MessageKind::Success => render::success::render_success_popup(frame, &self.message),
            MessageKind::Loading => {
                render::loading::render_loading_popup(frame, self.message.clone())
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PromptAction {
    EnterHostIp,
    EnterGameCode,
    EnterLichessToken,
    LoadPgnPath,
    EnterEnginePath,
    MoveInput,
}

pub struct PromptModal {
    action: PromptAction,
}

impl PromptModal {
    pub fn enter_host_ip() -> Self {
        Self {
            action: PromptAction::EnterHostIp,
        }
    }

    pub fn enter_game_code() -> Self {
        Self {
            action: PromptAction::EnterGameCode,
        }
    }

    pub fn enter_lichess_token() -> Self {
        Self {
            action: PromptAction::EnterLichessToken,
        }
    }

    pub fn load_pgn_path() -> Self {
        Self {
            action: PromptAction::LoadPgnPath,
        }
    }

    pub fn enter_engine_path() -> Self {
        Self {
            action: PromptAction::EnterEnginePath,
        }
    }

    pub fn move_input() -> Self {
        Self {
            action: PromptAction::MoveInput,
        }
    }

    fn submit(&self, app: &mut App) -> ModalResult {
        if self.action == PromptAction::MoveInput {
            let raw = app.game.ui.prompt.input.trim().to_string();
            if raw.is_empty() {
                return ModalResult::Close;
            }
            let player_move = normalize_lowercase_to_san(&raw);
            let san = match San::from_ascii(player_move.as_bytes()) {
                Ok(san) => san,
                Err(_) => {
                    app.game.ui.prompt.error = Some("Invalid notation".into());
                    return ModalResult::None;
                }
            };
            let position = app.game.runtime.current_position().clone();
            let Ok(chess_move) = san.to_move(&position) else {
                app.game.ui.prompt.error = Some("Illegal move".into());
                return ModalResult::None;
            };
            if app.submit_typed_move(&chess_move) {
                app.game.ui.prompt.reset();
                return ModalResult::None;
            }
            app.game.ui.prompt.error = Some("Illegal move".into());
            return ModalResult::None;
        }

        app.game.ui.prompt.submit_message();
        let message = app.game.ui.prompt.message.clone();

        match self.action {
            PromptAction::EnterHostIp => {
                if app.current_game_kind() == Some(GamePageKind::Multiplayer) {
                    app.multiplayer_state.host_ip = Some(message);
                }
                ModalResult::Close
            }
            PromptAction::EnterGameCode => {
                if !message.is_empty() {
                    app.show_lichess();
                    app.join_lichess_game_by_code(message);
                } else {
                    app.show_lichess_menu();
                }
                ModalResult::Close
            }
            PromptAction::EnterLichessToken => {
                let token = message.trim().to_string();
                if !token.is_empty() {
                    app.save_and_validate_lichess_token(token);
                }
                ModalResult::Close
            }
            PromptAction::LoadPgnPath => {
                let path = message.trim().to_string();
                if path.is_empty() {
                    return ModalResult::Close;
                }
                match chess_tui_core::PgnViewer::from_file(&path) {
                    Ok(games) => {
                        app.show_pgn_viewer(games, 0);
                        ModalResult::Close
                    }
                    Err(e) => {
                        app.game.ui.prompt.reset();
                        ModalResult::Replace(Box::new(MessageModal::error(format!(
                            "Failed to load PGN:\n{e}"
                        ))))
                    }
                }
            }
            PromptAction::EnterEnginePath => {
                let path = message.trim().trim_matches(&['\'', '"'][..]).to_string();
                app.bot_state.chess_engine_path = Some(path);
                app.update_config_from_app();
                ModalResult::Close
            }
            PromptAction::MoveInput => ModalResult::None,
        }
    }
}

impl Modal for PromptModal {
    fn handle_key(&mut self, key: KeyEvent, app: &mut App) -> ModalResult {
        match key.code {
            KeyCode::Enter => self.submit(app),
            KeyCode::Esc => {
                app.game.ui.prompt.reset();
                if self.action == PromptAction::EnterGameCode {
                    app.show_lichess_menu();
                }
                ModalResult::Close
            }
            KeyCode::Char(to_insert) => {
                app.game.ui.prompt.enter_char(to_insert);
                ModalResult::None
            }
            KeyCode::Backspace => {
                app.game.ui.prompt.delete_char();
                ModalResult::None
            }
            KeyCode::Left => {
                app.game.ui.prompt.move_cursor_left();
                ModalResult::None
            }
            KeyCode::Right => {
                app.game.ui.prompt.move_cursor_right();
                ModalResult::None
            }
            _ => ModalResult::None,
        }
    }

    fn render(&mut self, frame: &mut Frame<'_>, _area: Rect, app: &App) {
        let prompt = &app.game.ui.prompt;
        match self.action {
            PromptAction::EnterHostIp => {
                render::multiplayer::enter_ip::render_enter_multiplayer_ip(frame, prompt)
            }
            PromptAction::EnterGameCode => {
                render::lichess::game_code::render_enter_game_code_popup(frame, prompt)
            }
            PromptAction::EnterLichessToken => {
                render::lichess::token::render_enter_lichess_token_popup(frame, prompt)
            }
            PromptAction::LoadPgnPath => render::pgn::path::render_load_pgn_popup(frame, prompt),
            PromptAction::EnterEnginePath => {
                render::engine_path::render_enter_engine_path_popup(frame, prompt)
            }
            PromptAction::MoveInput => render::move_input::render_move_input_popup(frame, prompt),
        }
    }
}

pub struct SeekingLichessModal;

impl Modal for SeekingLichessModal {
    fn handle_key(&mut self, key: KeyEvent, app: &mut App) -> ModalResult {
        if key.code == KeyCode::Esc {
            if let Some(token) = &app.lichess_state.cancellation_token {
                token.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            app.lichess_state.receiver = None;
            app.lichess_state.cancellation_token = None;
            app.show_home();
            return ModalResult::Close;
        }
        ModalResult::None
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, _app: &App) {
        render::seeking::render_seeking_popup(frame, area);
    }
}

pub struct WaitingForOpponentModal;

impl Modal for WaitingForOpponentModal {
    fn handle_key(&mut self, key: KeyEvent, app: &mut App) -> ModalResult {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ') => {
                app.show_home();
                app.cancel_hosting_cleanup();
                ModalResult::Close
            }
            _ => ModalResult::None,
        }
    }

    fn render(&mut self, frame: &mut Frame<'_>, _area: Rect, _app: &App) {
        render::multiplayer::wait_for_player::render_wait_for_other_player(
            frame,
            chess_tui_net::get_host_ip(),
        );
    }
}

pub struct ResignConfirmationModal;

impl Modal for ResignConfirmationModal {
    fn handle_key(&mut self, key: KeyEvent, app: &mut App) -> ModalResult {
        match key.code {
            KeyCode::Char('y' | 'Y') | KeyCode::Enter => {
                app.confirm_resign_game();
                ModalResult::Close
            }
            KeyCode::Char('n' | 'N') | KeyCode::Esc => ModalResult::Close,
            _ => ModalResult::None,
        }
    }

    fn render(&mut self, frame: &mut Frame<'_>, _area: Rect, app: &App) {
        let opponent = app
            .lichess_state
            .ongoing_games
            .get(app.ui_state.menu_cursor as usize)
            .map(|game| game.opponent.username.as_str());
        render::resignation::render_resign_confirmation_popup(frame, opponent);
    }
}

pub struct NewGameConfirmationModal;

impl Modal for NewGameConfirmationModal {
    fn handle_key(&mut self, key: KeyEvent, app: &mut App) -> ModalResult {
        use crate::state::ui_state::NewGame;
        match key.code {
            KeyCode::Char('y' | 'Y') => {
                match app.ui_state.pending_new_game.take() {
                    Some(NewGame::Start(page)) => crate::game_mode_menu::start_game(app, page),
                    Some(NewGame::Restart) => app.restart(),
                    None => {}
                }
                ModalResult::Close
            }
            KeyCode::Char('r' | 'R') => {
                if let Some(NewGame::Start(page)) = app.ui_state.pending_new_game {
                    app.ui_state.pending_new_game = None;
                    let mode = if page == GamePageKind::Bot {
                        crate::state::resume::ResumeMode::Bot
                    } else {
                        crate::state::resume::ResumeMode::Local
                    };
                    app.game_mode_state.form_active = false;
                    app.game_mode_state.form_cursor = 0;
                    app.resume_from_saved(mode);
                }
                ModalResult::Close
            }
            KeyCode::Char('n' | 'N') | KeyCode::Esc => {
                app.ui_state.pending_new_game = None;
                ModalResult::Close
            }
            _ => ModalResult::None,
        }
    }

    fn render(&mut self, frame: &mut Frame<'_>, _area: Rect, app: &App) {
        let restart =
            app.ui_state.pending_new_game == Some(crate::state::ui_state::NewGame::Restart);
        render::new_game::render_new_game_confirmation_popup(frame, restart);
    }
}

pub struct PuzzleEndModal {
    message: String,
}

impl PuzzleEndModal {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl Modal for PuzzleEndModal {
    fn handle_key(&mut self, key: KeyEvent, app: &mut App) -> ModalResult {
        match key.code {
            KeyCode::Char('h' | 'H') | KeyCode::Esc => ModalResult::Close,
            KeyCode::Char('n' | 'N') => {
                app.start_puzzle_mode();
                ModalResult::Close
            }
            KeyCode::Char('b' | 'B') => {
                app.reset_home();
                ModalResult::Close
            }
            _ => ModalResult::None,
        }
    }

    fn render(&mut self, frame: &mut Frame<'_>, _area: Rect, app: &App) {
        let (elo_change, is_calculating) = if let Some(puzzle_game) = &app.lichess_state.puzzle_game
        {
            (
                puzzle_game.elo_change,
                puzzle_game.elo_change.is_none() && puzzle_game.elo_change_receiver.is_some(),
            )
        } else {
            (None, false)
        };
        render::puzzle_end::render_puzzle_end_popup(
            frame,
            &self.message,
            elo_change,
            is_calculating,
        );
    }
}
