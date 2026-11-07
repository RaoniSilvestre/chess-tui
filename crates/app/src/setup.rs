use std::{panic, path::Path};

use clap::Parser;
use config::{Args, Config};
use log::LevelFilter;
use model::PgnViewer;
use pages::Globals;

use crate::{App, AppResult, PageRequest, logging};

/// Result of parsing CLI arguments and preparing the application.
pub enum AppSetup {
    Run(Box<App>),
    Done,
}

impl App {
    /// Parses CLI arguments, runs non-TUI commands, or prepares the TUI app.
    pub fn setup() -> AppResult<AppSetup> {
        let args = Args::parse();
        if args.update_skins {
            theme::Skin::run_update_skins()?;
            return Ok(AppSetup::Done);
        }
        Self::setup_with_args(&args).map(|app| AppSetup::Run(Box::new(app)))
    }

    /// Full setup path, kept separate from argument parsing for testability.
    pub fn setup_with_args(args: &Args) -> AppResult<Self> {
        setup_panic();

        let folder_path = config::chess_tui_dir().map_err(str::to_string)?;
        let config_path = folder_path.join("config.toml");
        let config = Config::config_create(args, &folder_path, &config_path)?;

        setup_logging(&config, &folder_path);

        let globals = Globals::from_config(&config, args);
        let mut app = Self::new(globals);

        setup_pgn(&mut app, args)?;
        enable_mouse_capture()?;

        Ok(app)
    }

    /// One-line entry point used by `main`.
    pub fn main() -> AppResult<()> {
        match Self::setup()? {
            AppSetup::Run(mut app) => app.run(),
            AppSetup::Done => Ok(()),
        }
    }
}

fn setup_panic() {
    let default_panic = std::panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        let _ = ratatui::crossterm::execute!(
            std::io::stdout(),
            ratatui::crossterm::event::DisableMouseCapture
        );
        default_panic(info);
    }));
}

fn enable_mouse_capture() -> AppResult<()> {
    Ok(ratatui::crossterm::execute!(
        std::io::stdout(),
        ratatui::crossterm::event::EnableMouseCapture
    )?)
}

fn setup_logging(config: &Config, folder_path: &Path) {
    let level = config
        .log_level
        .as_deref()
        .unwrap_or("OFF")
        .parse()
        .unwrap_or(LevelFilter::Off);
    if let Err(e) = logging::setup_logging(folder_path, &level) {
        eprintln!("Failed to initialize logging: {}", e);
    }
}

/// Load PGN file(s) if --pgn was provided.
fn setup_pgn(app: &mut App, args: &Args) -> AppResult<()> {
    let Some(ref pgn_path) = args.pgn else {
        return Ok(());
    };

    let games = load_pgn_games(Path::new(pgn_path)).unwrap_or_else(|e| {
        eprintln!("Failed to load PGN '{pgn_path}': {e}");
        std::process::exit(1);
    });

    app.navigate(PageRequest::PgnViewer { games, index: 0 });

    Ok(())
}

fn load_pgn_games(path: &Path) -> Result<Vec<PgnViewer>, String> {
    if path.is_dir() {
        load_pgn_from_dir(path)
    } else {
        PgnViewer::from_file(path.to_str().unwrap_or(path.to_string_lossy().as_ref()))
    }
}

fn load_pgn_from_dir(dir_path: &Path) -> Result<Vec<PgnViewer>, String> {
    let mut entries: Vec<_> = std::fs::read_dir(dir_path)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "pgn"))
        .collect();

    entries.sort_by_key(|e| e.path());

    let mut all_games = Vec::new();
    for entry in entries {
        let file_path = entry.path();
        match PgnViewer::from_file(file_path.to_str().unwrap_or("")) {
            Ok(mut games) => all_games.append(&mut games),
            Err(e) => eprintln!("Skipping {file_path:?}: {e}"),
        }
    }

    if all_games.is_empty() {
        Err(format!(
            "No valid .pgn files found in '{}'",
            dir_path.display()
        ))
    } else {
        Ok(all_games)
    }
}
