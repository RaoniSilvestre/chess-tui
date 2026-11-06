use crate::{App, GameModeKind, Modal, ModalResult, PageAction};

impl App {
    pub fn open_modal(&mut self, modal: Box<dyn Modal>) {
        self.services.modal = Some(modal);
    }
    pub fn close_modal(&mut self) {
        self.services.modal = None;
    }
    pub fn has_any_modal(&self) -> bool {
        self.services.modal.is_some()
    }
    pub fn has_modal<T: Modal + 'static>(&self) -> bool {
        self.services.modal
            .as_ref()
            .is_some_and(|modal| modal.as_any().type_id() == std::any::TypeId::of::<T>())
    }
    pub fn show_help(&mut self) {
        self.open_modal(Box::new(crate::modal::HelpModal));
    }
    pub fn toggle_help(&mut self) {
        if self.has_modal::<crate::modal::HelpModal>() {
            self.close_modal();
        } else {
            self.show_help();
        }
    }
    pub fn show_error(&mut self, message: String) {
        self.open_modal(Box::new(crate::modal::MessageModal::error(message)));
    }
    pub fn show_success(&mut self, message: String) {
        self.open_modal(Box::new(crate::modal::MessageModal::success(message)));
    }
    pub fn show_loading(&mut self, message: String) {
        self.open_modal(Box::new(crate::modal::MessageModal::loading(message)));
    }
    pub fn show_enter_host_ip(&mut self) {
        self.services.game_mut().ui.prompt.reset();
        self.open_modal(Box::new(crate::modal::PromptModal::enter_host_ip()));
    }
    pub fn show_enter_game_code(&mut self) {
        self.services.game_mut().ui.prompt.reset();
        self.open_modal(Box::new(crate::modal::PromptModal::enter_game_code()));
    }
    pub fn show_enter_lichess_token(&mut self) {
        self.services.game_mut().ui.prompt.reset();
        self.services.game_mut().ui.prompt.message = "Enter your Lichess API token:".to_string();
        self.open_modal(Box::new(crate::modal::PromptModal::enter_lichess_token()));
    }
    pub fn show_load_pgn_path(&mut self) {
        self.services.game_mut().ui.prompt.reset();
        self.open_modal(Box::new(crate::modal::PromptModal::load_pgn_path()));
    }
    pub fn show_enter_engine_path(&mut self) {
        {
            let mut game = self.services.game_mut();
            game.ui.prompt.input =
                self.services.bot_state.chess_engine_path.clone().unwrap_or_default();
            game.ui.prompt.character_index = game.ui.prompt.input.chars().count();
        }
        self.open_modal(Box::new(crate::modal::PromptModal::enter_engine_path()));
    }
    pub fn show_move_input(&mut self) {
        self.services.game_mut().ui.prompt.reset();
        self.open_modal(Box::new(crate::modal::PromptModal::move_input()));
    }
    pub fn show_seeking_lichess_game(&mut self) {
        self.open_modal(Box::new(crate::modal::SeekingLichessModal));
    }
    pub fn show_waiting_for_opponent(&mut self) {
        self.open_modal(Box::new(crate::modal::WaitingForOpponentModal));
    }
    pub fn show_resign_confirmation(&mut self, index: usize) {
        self.open_modal(Box::new(crate::modal::ResignConfirmationModal::new(index)));
    }
    pub fn show_new_game_confirmation(&mut self, pending: crate::game_mode_menu::NewGame) {
        self.open_modal(Box::new(crate::modal::NewGameConfirmationModal::new(
            pending,
        )));
    }
    pub fn show_puzzle_end(&mut self, message: String) {
        self.open_modal(Box::new(crate::modal::PuzzleEndModal::new(message)));
    }
    pub fn after_error_popup(&mut self) {
        if self.current_game_mode() == Some(GameModeKind::Lichess)
            || self
                .router
                .as_ref()
                .is_some_and(|router| router.page().is_ongoing_games())
        {
            self.show_lichess_menu();
        } else if matches!(
            self.current_game_mode(),
            Some(GameModeKind::Multiplayer | GameModeKind::Bot)
        ) {
            self.show_home();
        }
    }
    pub fn after_success_popup(&mut self) {
        if self.current_game_mode() == Some(GameModeKind::Lichess) {
            self.show_lichess_menu();
        } else if self
            .router
            .as_ref()
            .is_some_and(|router| router.page().is_ongoing_games())
        {
            self.fetch_ongoing_games();
            self.show_ongoing_games();
        } else if matches!(
            self.current_game_mode(),
            Some(GameModeKind::Multiplayer | GameModeKind::Bot)
        ) {
            self.show_home();
        }
    }
    pub fn render_modal(&mut self, frame: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect) {
        if let Some(mut modal) = self.services.modal.take() {
            modal.render(frame, area, self);
            self.services.modal = Some(modal);
        }
    }
    pub fn handle_modal_key(&mut self, key: ratatui::crossterm::event::KeyEvent) -> bool {
        let Some(mut modal) = self.services.modal.take() else {
            return false;
        };
        let result = modal.handle_key(key, self);
        match result {
            ModalResult::None => self.services.modal = Some(modal),
            ModalResult::Close => {}
            ModalResult::Replace(next) => self.services.modal = Some(next),
        }
        true
    }
    pub fn submit_move_to_page(&mut self, chess_move: &shakmaty::Move) -> bool {
        let Some(mut router) = self.router.take() else {
            return false;
        };
        let action = {
            let ctx = self.page_context(false);
            router.page_mut().submit_move(chess_move, &ctx)
        };
        self.router = Some(router);
        let applied = !matches!(action, PageAction::None);
        self.apply_page_action(action);
        applied
    }
}
