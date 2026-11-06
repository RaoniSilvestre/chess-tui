use crate::{App, GameModeKind, Page, PgnViewerAccess, Router};

impl App {
    pub fn show_solo(&mut self) {
        let page = self.pages.solo(self.services.game.clone());
        self.set_page(page);
    }
    pub fn show_bot(&mut self) {
        let page = self.pages.bot(self.services.game.clone());
        self.set_page(page);
    }
    pub fn show_multiplayer(&mut self) {
        let page = self.pages.multiplayer(self.services.game.clone());
        self.set_page(page);
    }
    pub fn show_pgn_viewer(&mut self, games: Vec<model::PgnViewer>, game_idx: usize) {
        let page = self.pages.pgn_viewer(games, game_idx);
        self.set_page(page);
    }
    pub fn show_lichess(&mut self) {
        let page = self.pages.lichess(self.services.game.clone());
        self.set_page(page);
    }
    pub fn set_page(&mut self, page: Box<dyn Page>) {
        if self.router.is_some() {
            self.router = Some(Router::new(page));
        } else {
            self.pending_page = Some(page);
        }
    }
    pub fn take_pending_page(&mut self) -> Option<Box<dyn Page>> {
        self.pending_page.take()
    }
    pub fn sync_page(&mut self) {
        if let Some(page) = self.pending_page.take() {
            self.router = Some(Router::new(page));
        }
    }
    pub fn is_page<T: Page + 'static>(&self) -> bool {
        self.router
            .as_ref()
            .is_some_and(|router| router.page().as_any().is::<T>())
    }
    pub fn current_game_mode(&self) -> Option<GameModeKind> {
        self.router.as_ref().and_then(|router| router.page().game_mode())
    }
    pub fn current_page_type_id(&self) -> Option<std::any::TypeId> {
        self.router
            .as_ref()
            .map(|router| router.page().as_any().type_id())
    }
    pub fn menu_cursor(&self) -> Option<u8> {
        self.router.as_ref()?.page().menu_cursor()
    }
    pub fn form_active(&self) -> Option<bool> {
        self.router.as_ref()?.page().form_active()
    }
    pub fn is_home(&self) -> bool {
        self.router
            .as_ref()
            .is_some_and(|router| router.page().is_home())
    }
    pub fn pgn_viewer(&self) -> Option<&dyn PgnViewerAccess> {
        self.router.as_ref()?.page().pgn_viewer()
    }
    pub fn is_pgn_viewer(&self) -> bool {
        self.router
            .as_ref()
            .is_some_and(|router| router.page().is_pgn_viewer())
    }
    pub fn open_game_mode_menu(&mut self) {
        let page = self.pages.game_mode_menu();
        self.set_page(page);
    }
    pub fn show_home(&mut self) {
        let page = self.pages.home();
        self.set_page(page);
    }
    pub fn show_lichess_menu(&mut self) {
        let page = self.pages.lichess_menu();
        self.set_page(page);
    }
    pub fn show_ongoing_games(&mut self) {
        let page = self.pages.ongoing_games();
        self.set_page(page);
    }
    pub fn open_lichess_menu(&mut self) {
        if !self.lichess_has_token() {
            self.lichess_open_token_prompt();
            return;
        }
        self.show_lichess_menu();
        if self.services.lichess_state.user_profile.is_none() {
            self.show_loading("Loading your lichess account ...".to_string());
        }
        self.fetch_lichess_user_profile();
    }
    pub fn open_settings(&mut self) {
        let page = self.pages.settings();
        self.set_page(page);
    }
    pub fn open_help(&mut self) {
        self.toggle_help();
    }
    pub fn open_credits(&mut self) {
        let page = self.pages.credits();
        self.set_page(page);
    }
    pub fn go_home(&mut self) {
        self.show_home();
    }
}
