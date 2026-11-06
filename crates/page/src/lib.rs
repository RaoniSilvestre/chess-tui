//! Page API: page lifecycle, actions, and read-only render context.
//!
//! This crate deliberately does not depend on the application crate. Pages
//! return [`PageAction`] values and render from a [`PageContext`] snapshot.

use std::any::Any;

use model::{PgnViewer, TimeControl};
use session::GameHandle;
use ratatui::Frame;
use ratatui::crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use shakmaty::Color;

use ui::graphics::KittyPieces;

/// Object-safe downcasting helper for pages.
pub trait AsAny {
    fn as_any(&self) -> &dyn Any;
}

impl<T: 'static> AsAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The concrete game page kind.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameModeKind {
    Local,
    Bot,
    Multiplayer,
    Lichess,
}

pub const TIME_CONTROL_CUSTOM_INDEX: u8 = 6;
pub const TIME_CONTROL_OPTIONS: &[&str] = &[
    "UltraBullet",
    "Bullet",
    "Blitz",
    "Rapid",
    "Classical",
    "No clock",
    "Custom",
];
pub const BOT_DIFFICULTY_NAMES: [&str; 4] =
    ["Easy (400)", "Medium (900)", "Hard (1500)", "Magnus (2700)"];
pub const BOT_DIFFICULTY_COUNT: usize = 4;

/// The four game modes available from the game-mode selection menu.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum AvailableGameMode {
    Local,
    Multiplayer,
    Bot,
    PGNLoader,
}

impl AvailableGameMode {
    pub const COUNT: u8 = 4;
}

/// Why a page requested a transition or a service mutation.
pub enum PageAction {
    None,
    Quit,
    OpenHome,
    OpenGameModeMenu,
    OpenLichessMenu,
    OpenSettings,
    OpenCredits,
    OpenHelp,
    Settings(SettingsAction),
    GameMode(GameModeAction),
    Lichess(LichessAction),
    Ongoing(OngoingAction),
    Pgn(PgnAction),
    Game(GameAction),
}

pub enum SettingsAction {
    CycleSkin(bool),
    ToggleSound,
    ToggleAnimations,
    OpenEnginePath,
    BotDepthDown,
    BotDepthUp,
    CycleDifficulty(bool),
    ToggleKitty,
}

pub enum GameModeAction {
    Start {
        mode: AvailableGameMode,
        time_control: TimeControl,
        color: Option<Color>,
        random_color: bool,
        hosting: Option<bool>,
    },
    Resume(AvailableGameMode),
    OpenPgnLoader,
}

pub enum LichessAction {
    Seek { time_control: u8 },
    Puzzle,
    OpenOngoingGames,
    JoinByCode,
    Disconnect,
    PromptToken,
}

pub enum OngoingAction {
    Open(usize),
    RequestResign(usize),
    Back,
}

pub enum PgnAction {
    Exit,
}

pub enum GameAction {
    RequestRestart,
    GoHome,
    OpenMoveInput,
    OpenHelp,
    Quit,
    PuzzleHint,
    ValidatePromotion { move_uci: String },
    ValidateMoveUci { move_uci: String },
    ValidateMove { from: shakmaty::Square, to: shakmaty::Square },
    CheckEnd,
}

pub trait PgnViewerAccess {
    fn pgn_games(&self) -> &[PgnViewer];
    fn pgn_game_idx(&self) -> usize;
}

/// Read-only snapshots of shared services used while rendering pages.
pub struct PageContext<'a> {
    pub settings: SettingsView,
    pub game_mode: GameModeView,
    pub game: GameView,
    pub lichess: LichessView<'a>,
    pub kitty: Option<&'a mut KittyPieces>,
    pub kitty_enabled: bool,
}

pub struct SettingsView {
    pub has_sound: bool,
    pub kitty_available: bool,
    pub skin_name: String,
    pub sound_enabled: bool,
    pub animations_enabled: bool,
    pub bot_depth: u8,
    pub bot_difficulty: Option<u8>,
    pub kitty_graphics_enabled: bool,
}

pub struct GameView {
    pub is_puzzle: bool,
    pub selected_color: Option<Color>,
    pub is_lichess: bool,
}

pub struct GameModeView {
    pub bot_depth: u8,
    pub bot_difficulty: Option<u8>,
    pub engine_path: Option<String>,
    pub hosting: Option<bool>,
    pub has_local_resume: bool,
    pub has_bot_resume: bool,
}

pub struct LichessView<'a> {
    pub has_token: bool,
    pub profile: Option<&'a lichess::UserProfile>,
    pub rating_history: Option<&'a [lichess::RatingHistoryEntry]>,
    pub ongoing_games: &'a [lichess::OngoingGame],
}

/// A full-screen page. Pages own their local state and return intents.
pub trait Page: AsAny {
    fn handle_key(&mut self, key: KeyEvent, ctx: &PageContext<'_>) -> PageAction;

    fn handle_mouse(&mut self, _mouse: MouseEvent, _ctx: &PageContext<'_>) -> PageAction {
        PageAction::None
    }

    fn tick(&mut self, _ctx: &PageContext<'_>) -> PageAction {
        PageAction::None
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, ctx: &mut PageContext<'_>);

    fn game_mode(&self) -> Option<GameModeKind> {
        None
    }

    fn is_pgn_viewer(&self) -> bool {
        false
    }

    fn is_home(&self) -> bool {
        false
    }

    fn is_ongoing_games(&self) -> bool {
        false
    }

    fn pgn_viewer(&self) -> Option<&dyn PgnViewerAccess> {
        None
    }

    /// Optional cursor for tests and accessibility checks.
    fn menu_cursor(&self) -> Option<u8> {
        None
    }

    fn form_active(&self) -> Option<bool> {
        None
    }

    /// Prepare page-owned board state for external rendering/tests.
    fn sync_pgn(&mut self) {}

    /// Submit a typed move (used by the move-input modal).
    fn submit_move(&mut self, _chess_move: &shakmaty::Move, _ctx: &PageContext<'_>) -> PageAction {
        PageAction::None
    }
}

/// Owns the active page and returns the action it produced.
pub struct Router {
    page: Box<dyn Page>,
}

impl Router {
    pub fn new(page: Box<dyn Page>) -> Self {
        Self { page }
    }

    pub fn page(&self) -> &dyn Page {
        self.page.as_ref()
    }

    pub fn page_mut(&mut self) -> &mut dyn Page {
        self.page.as_mut()
    }

    pub fn handle_key(&mut self, key: KeyEvent, ctx: &PageContext<'_>) -> PageAction {
        self.page.handle_key(key, ctx)
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent, ctx: &PageContext<'_>) -> PageAction {
        self.page.handle_mouse(mouse, ctx)
    }

    pub fn tick(&mut self, ctx: &PageContext<'_>) -> PageAction {
        self.page.tick(ctx)
    }

    pub fn render(&mut self, frame: &mut Frame<'_>, area: Rect, ctx: &mut PageContext<'_>) {
        self.page.render(frame, area, ctx);
    }
}

/// Constructs page instances. Implemented by `pages`; used by the app.
pub trait PageFactory {
    fn home(&self) -> Box<dyn Page>;
    fn solo(&self, game: GameHandle) -> Box<dyn Page>;
    fn bot(&self, game: GameHandle) -> Box<dyn Page>;
    fn multiplayer(&self, game: GameHandle) -> Box<dyn Page>;
    fn lichess(&self, game: GameHandle) -> Box<dyn Page>;
    fn lichess_menu(&self) -> Box<dyn Page>;
    fn ongoing_games(&self) -> Box<dyn Page>;
    fn game_mode_menu(&self) -> Box<dyn Page>;
    fn settings(&self) -> Box<dyn Page>;
    fn credits(&self) -> Box<dyn Page>;
    fn pgn_viewer(&self, games: Vec<PgnViewer>, game_idx: usize) -> Box<dyn Page>;
}
