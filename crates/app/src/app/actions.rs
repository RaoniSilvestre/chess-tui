use crate::{
    App, AvailableGameMode, GameAction, GameModeAction, GameModeView, GameModeKind, GameView, LichessAction,
    LichessView, OngoingAction, PageAction, PageContext, PgnAction, SettingsAction,
    SettingsView,
};
use crate::state::resume::ResumeMode;
use ::game::GameKind;
use model::Clock;
use theme::DisplayMode;

impl App {
    pub(crate) fn page_context(&mut self, include_kitty: bool) -> PageContext<'_> {
        let has_local_resume = self.has_resume_save(ResumeMode::Local);
        let has_bot_resume = self.has_resume_save(ResumeMode::Bot);
        let skin_name = {
            let game = self.services.game();
            match game.ui.display_mode {
                DisplayMode::DEFAULT => "Default".to_string(),
                DisplayMode::ASCII => "ASCII".to_string(),
                DisplayMode::CUSTOM => game.ui.skin.name.clone(),
            }
        };
        let is_lichess = {
            let game = self.services.game();
            game.runtime.kind() == GameKind::Lichess
        };
        let settings = SettingsView {
            has_sound: cfg!(feature = "sound"),
            kitty_available: self.services.kitty_pieces.is_some(),
            skin_name,
            sound_enabled: self.services.sound_enabled,
            animations_enabled: self.services.animations_enabled,
            bot_depth: self.services.bot_state.bot_depth,
            bot_difficulty: self.services.bot_state.bot_difficulty,
            kitty_graphics_enabled: self.services.kitty_graphics_enabled,
        };
        let game_mode = GameModeView {
            bot_depth: self.services.bot_state.bot_depth,
            bot_difficulty: self.services.bot_state.bot_difficulty,
            engine_path: self.services.bot_state.chess_engine_path.clone(),
            hosting: self.services.multiplayer_state.hosting,
            has_local_resume,
            has_bot_resume,
        };
        let game = GameView {
            is_puzzle: self.services.lichess_state.puzzle_game.is_some(),
            selected_color: self.services.game_mode_state.selected_color,
            is_lichess,
        };
        let has_token = self.lichess_has_token();
        let kitty = if include_kitty {
            self.services.kitty_pieces.as_mut()
        } else {
            None
        };
        let lichess = LichessView {
            has_token,
            profile: self.services.lichess_state.user_profile.as_ref(),
            rating_history: self.services.lichess_state.rating_history.as_deref(),
            ongoing_games: &self.services.lichess_state.ongoing_games,
        };
        PageContext {
            settings,
            game_mode,
            game,
            lichess,
            kitty,
            kitty_enabled: self.services.kitty_graphics_enabled,
        }
    }
    pub(crate) fn apply_page_action(&mut self, action: PageAction) {
        match action {
            PageAction::None => {}
            PageAction::Quit => self.quit(),
            PageAction::OpenHome => self.show_home(),
            PageAction::OpenGameModeMenu => self.open_game_mode_menu(),
            PageAction::OpenLichessMenu => self.open_lichess_menu(),
            PageAction::OpenSettings => self.open_settings(),
            PageAction::OpenCredits => self.open_credits(),
            PageAction::OpenHelp => self.show_help(),
            PageAction::Settings(action) => self.apply_settings_action(action),
            PageAction::GameMode(action) => self.apply_game_mode_action(action),
            PageAction::Lichess(action) => self.apply_lichess_action(action),
            PageAction::Ongoing(action) => self.apply_ongoing_action(action),
            PageAction::Pgn(PgnAction::Exit) => self.reset_home(),
            PageAction::Game(action) => self.apply_game_action(action),
        }
    }
    fn apply_settings_action(&mut self, action: SettingsAction) {
        match action {
            SettingsAction::CycleSkin(next) => self.settings_cycle_skin(next),
            SettingsAction::ToggleSound => self.settings_toggle_sound(),
            SettingsAction::ToggleAnimations => self.settings_toggle_animations(),
            SettingsAction::OpenEnginePath => self.settings_open_engine_path(),
            SettingsAction::BotDepthDown => self.settings_bot_depth_down(),
            SettingsAction::BotDepthUp => self.settings_bot_depth_up(),
            SettingsAction::CycleDifficulty(next) => self.settings_cycle_difficulty(next),
            SettingsAction::ToggleKitty => self.settings_toggle_kitty(),
        }
    }
    fn apply_game_mode_action(&mut self, action: GameModeAction) {
        match action {
            GameModeAction::Start {
                mode,
                time_control,
                color,
                random_color,
                hosting,
            } => {
                self.services.game_mode_state.selected_color = color;
                self.services.game_mode_state.is_random_color = random_color;
                self.services.multiplayer_state.hosting = hosting;
                self.services.game_mut().runtime.set_clock(Clock::new(time_control));
                match mode {
                    AvailableGameMode::Local => crate::game_mode_menu::finish_form(
                        self,
                        GameModeKind::Local,
                    ),
                    AvailableGameMode::Bot => {
                        crate::game_mode_menu::finish_form(self, GameModeKind::Bot)
                    }
                    AvailableGameMode::Multiplayer => {
                        crate::game_mode_menu::finish_form(self, GameModeKind::Multiplayer)
                    }
                    AvailableGameMode::PGNLoader => self.show_load_pgn_path(),
                }
            }
            GameModeAction::Resume(mode) => match mode {
                AvailableGameMode::Local => {
                    self.resume_from_saved(ResumeMode::Local);
                }
                AvailableGameMode::Bot => {
                    self.resume_from_saved(ResumeMode::Bot);
                }
                _ => {}
            },
            GameModeAction::OpenPgnLoader => self.show_load_pgn_path(),
        }
    }
    fn apply_lichess_action(&mut self, action: LichessAction) {
        match action {
            LichessAction::Seek { time_control } => self.lichess_seek(time_control),
            LichessAction::Puzzle => self.lichess_start_puzzle(),
            LichessAction::OpenOngoingGames => self.lichess_open_ongoing_games(),
            LichessAction::JoinByCode => self.lichess_open_join_code(),
            LichessAction::Disconnect => self.lichess_disconnect(),
            LichessAction::PromptToken => self.lichess_open_token_prompt(),
        }
    }
    fn apply_ongoing_action(&mut self, action: OngoingAction) {
        match action {
            OngoingAction::Open(index) => self.ongoing_game_open(index),
            OngoingAction::RequestResign(index) => self.ongoing_game_request_resign(index),
            OngoingAction::Back => self.ongoing_games_back(),
        }
    }
    fn apply_game_action(&mut self, action: GameAction) {
        match action {
            GameAction::RequestRestart => self.request_restart(),
            GameAction::GoHome => self.reset_home(),
            GameAction::OpenMoveInput => self.show_move_input(),
            GameAction::OpenHelp => self.show_help(),
            GameAction::Quit => self.quit(),
            GameAction::PuzzleHint => {
                if let Some(puzzle_game) = &self.services.lichess_state.puzzle_game {
                    let mut game = self.services.game_mut();
                    puzzle_game.show_hint(&mut game);
                }
            }
            GameAction::ValidatePromotion { move_uci } => {
                self.validate_puzzle_move_uci(move_uci);
            }
            GameAction::ValidateMoveUci { move_uci } => {
                self.validate_puzzle_move_uci(move_uci);
            }
            GameAction::ValidateMove { from, to } => {
                self.validate_puzzle_move_after_execution(from, to);
            }
            GameAction::CheckEnd => {
                self.check_and_show_game_end();
            }
        }
    }
}
