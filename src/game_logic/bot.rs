//! Bot game configuration and the UCI engine session used for bot moves.

use crate::constants::{BOT_DIFFICULTY_DEPTH, BOT_DIFFICULTY_ELO, BOT_DIFFICULTY_MOVETIME_MS};
use shakmaty::{CastlingMode, Chess, Move, fen::Fen, uci::UciMove};
use std::borrow::Cow;
use std::process::Command;
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::thread::{self, JoinHandle};
use suci::{Engine, Go, Position, SetOption};

/// Bot game configuration.
///
/// The engine process itself lives in [`BotSession`], which is owned by the bot
/// runtime state rather than by the game.
#[derive(Clone)]
pub struct Bot {
    /// Used to indicate if a bot move is following
    pub bot_will_move: bool,
    /// if the bot is starting, meaning the player is black
    pub is_bot_starting: bool,
    /// Base depth when difficulty is Off (full strength)
    pub depth: u8,
    /// Difficulty preset index: None = Off (full strength), Some(0..=3) = Easy/Medium/Hard/Magnus
    pub difficulty: Option<u8>,
}

impl Bot {
    pub fn new(is_bot_starting: bool, depth: u8, difficulty: Option<u8>) -> Bot {
        Self {
            bot_will_move: false,
            is_bot_starting,
            depth,
            difficulty,
        }
    }
}

/// Engine settings that stay fixed for one game session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineConfig {
    /// Path to the engine binary, optionally followed by command-line arguments.
    pub path: String,
    /// Base search depth when `difficulty` is `None`.
    pub depth: u8,
    /// Difficulty preset index, applied once when the engine starts.
    pub difficulty: Option<u8>,
}

/// Something the engine session reports back to the game.
#[derive(Debug)]
pub enum BotEvent {
    /// A legal move for the requested position.
    Move(Move),
    /// The session can no longer be used for this game.
    Failed(String),
}

/// One engine session, started once per game and reused for every move.
///
/// Lifecycle: [`BotSession::start`] spawns the worker, [`BotSession::request_move`]
/// queues one search, [`BotSession::poll`] collects its result, and
/// [`BotSession::shutdown`] (or dropping the session) stops the engine.
pub struct BotSession {
    worker: WorkerHandle,
    events: Receiver<BotEvent>,
    phase: Phase,
}

enum Phase {
    Idle,
    Searching,
    Failed(String),
}

impl BotSession {
    /// Spawns the worker thread and starts the engine process for `config`.
    ///
    /// Starting is asynchronous: the handshake happens on the worker thread, and a
    /// startup failure is reported through [`BotSession::poll`].
    pub fn start(config: EngineConfig) -> Result<Self, String> {
        let (worker, events) = WorkerHandle::spawn(config)?;
        Ok(Self {
            worker,
            events,
            phase: Phase::Idle,
        })
    }

    /// Queues a search for `fen`.
    pub fn request_move(&mut self, fen: String) -> Result<(), String> {
        match &self.phase {
            Phase::Idle => {}
            Phase::Searching => return Err("Bot is already thinking".to_string()),
            Phase::Failed(error) => return Err(error.clone()),
        }

        self.worker.send(WorkerCommand::Search(fen))?;
        self.phase = Phase::Searching;
        Ok(())
    }

    /// Returns the next engine event, if one is ready.
    ///
    /// After a failure the session is terminal: it reports the error once and
    /// rejects further searches until the game starts a new session.
    pub fn poll(&mut self) -> Option<BotEvent> {
        if !matches!(&self.phase, Phase::Idle | Phase::Searching) {
            return None;
        }

        match self.events.try_recv() {
            Ok(BotEvent::Move(bot_move)) => {
                self.phase = Phase::Idle;
                Some(BotEvent::Move(bot_move))
            }
            Ok(BotEvent::Failed(error)) => {
                self.phase = Phase::Failed(error.clone());
                Some(BotEvent::Failed(error))
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                let error = "Bot engine thread stopped unexpectedly".to_string();
                self.phase = Phase::Failed(error.clone());
                Some(BotEvent::Failed(error))
            }
        }
    }

    /// Whether a search has been queued and its result has not been polled yet.
    pub fn is_thinking(&self) -> bool {
        matches!(&self.phase, Phase::Searching)
    }

    /// Stops the engine session.
    pub fn shutdown(mut self) {
        self.worker.shutdown();
    }
}

/// Commands understood by the engine worker.
enum WorkerCommand {
    Search(String),
    Shutdown,
}

/// Owns the worker thread and the channel that feeds it.
struct WorkerHandle {
    commands: Sender<WorkerCommand>,
    join: Option<JoinHandle<()>>,
    shutdown_requested: bool,
}

impl WorkerHandle {
    fn spawn(config: EngineConfig) -> Result<(Self, Receiver<BotEvent>), String> {
        let (commands, command_receiver) = channel();
        let (event_sender, events) = channel();
        let worker = EngineWorker {
            config,
            events: event_sender,
        };

        let join = thread::Builder::new()
            .name("bot-engine".to_string())
            .spawn(move || worker.run(command_receiver))
            .map_err(|error| format!("Failed to start bot engine thread: {error}"))?;

        let handle = Self {
            commands,
            join: Some(join),
            shutdown_requested: false,
        };
        Ok((handle, events))
    }

    fn send(&self, command: WorkerCommand) -> Result<(), String> {
        self.commands
            .send(command)
            .map_err(|_| "Bot engine thread stopped unexpectedly".to_string())
    }

    /// Asks the worker to exit, without blocking the UI thread.
    ///
    /// A search that is still running finishes (bounded by its movetime) and the
    /// worker quits the engine afterwards. If the thread already finished, it is
    /// joined so the OS resources are reclaimed.
    fn shutdown(&mut self) {
        if self.shutdown_requested {
            return;
        }
        self.shutdown_requested = true;

        let _ = self.commands.send(WorkerCommand::Shutdown);
        if let Some(join) = self.join.take()
            && join.is_finished()
        {
            let _ = join.join();
        }
    }
}

impl Drop for WorkerHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Worker thread body: owns the engine process and answers searches in order.
struct EngineWorker {
    config: EngineConfig,
    events: Sender<BotEvent>,
}

impl EngineWorker {
    fn run(self, commands: Receiver<WorkerCommand>) {
        let mut process = match EngineProcess::start(&self.config) {
            Ok(process) => process,
            Err(error) => {
                let _ = self.events.send(BotEvent::Failed(error));
                return;
            }
        };

        for command in commands {
            match command {
                WorkerCommand::Search(fen) => {
                    let _ = self.events.send(process.search(&fen));
                }
                WorkerCommand::Shutdown => break,
            }
        }

        process.shutdown();
    }
}

/// Owns the `suci::Engine` process for the session.
struct EngineProcess {
    engine: Engine,
    depth: usize,
    move_time: Option<usize>,
}

impl EngineProcess {
    fn start(config: &EngineConfig) -> Result<Self, String> {
        let mut parts = config.path.split_whitespace();
        let Some(program) = parts.next() else {
            return Err("No chess engine configured".to_string());
        };

        let mut command = Command::new(program);
        command.args(parts);

        let mut engine = Engine::spawn(command)
            .map_err(|error| format!("Failed to start chess engine `{}`: {error}", config.path))?;

        // Strength options are process-wide: set them once, before the first search.
        if let Some(elo) = elo_for(config.difficulty) {
            engine
                .set_option(&SetOption {
                    name: Cow::Borrowed("UCI_LimitStrength"),
                    value: Some(Cow::Borrowed("true")),
                })
                .map_err(|error| format!("Failed to configure UCI_LimitStrength: {error}"))?;
            engine
                .set_option(&SetOption {
                    name: Cow::Borrowed("UCI_Elo"),
                    value: Some(Cow::Owned(elo.to_string())),
                })
                .map_err(|error| format!("Failed to configure UCI_Elo: {error}"))?;
        }

        engine
            .new_game()
            .map_err(|error| format!("Failed to start a new engine game: {error}"))?;
        engine
            .wait_until_ready()
            .map_err(|error| format!("Chess engine did not become ready: {error}"))?;

        let (depth, move_time) = search_limits(config.depth, config.difficulty);
        Ok(Self {
            engine,
            depth,
            move_time,
        })
    }

    fn search(&mut self, fen: &str) -> BotEvent {
        match self.compute_move(fen) {
            Ok(bot_move) => BotEvent::Move(bot_move),
            Err(error) => BotEvent::Failed(error),
        }
    }

    fn compute_move(&mut self, fen: &str) -> Result<Move, String> {
        let parsed_fen = Fen::from_ascii(fen.as_bytes())
            .map_err(|error| format!("Failed to parse FEN '{fen}': {error}"))?;
        let position = parsed_fen
            .clone()
            .into_position::<Chess>(CastlingMode::Standard)
            .map_err(|error| format!("Failed to build a position from FEN '{fen}': {error}"))?;

        self.engine
            .set_position(&Position::Fen {
                fen: Cow::Owned(parsed_fen),
                moves: Cow::Borrowed(&[]),
            })
            .map_err(|error| format!("Failed to send position to engine: {error}"))?;

        let best_move = self
            .engine
            .go(&Go {
                depth: Some(self.depth),
                move_time: self.move_time,
                ..Default::default()
            })
            .map_err(|error| format!("Engine failed to compute a move: {error}"))?;

        let uci_move: UciMove = best_move
            .normal()
            .ok_or_else(|| "Engine did not return a move".to_string())?
            .movement;

        uci_move
            .to_move(&position)
            .map_err(|error| format!("Engine returned an illegal move: {error}"))
    }

    fn shutdown(self) {
        if let Err(error) = self.engine.quit() {
            log::warn!("Failed to shut down chess engine: {error}");
        }
    }
}

/// Search depth and optional movetime for a difficulty preset.
fn search_limits(base_depth: u8, difficulty: Option<u8>) -> (usize, Option<usize>) {
    match difficulty
        .map(usize::from)
        .filter(|index| *index < BOT_DIFFICULTY_DEPTH.len())
    {
        Some(index) => (
            BOT_DIFFICULTY_DEPTH[index] as usize,
            BOT_DIFFICULTY_MOVETIME_MS
                .get(index)
                .map(|millis| *millis as usize),
        ),
        None => (base_depth as usize, None),
    }
}

/// ELO for `UCI_LimitStrength` when the difficulty preset defines one.
fn elo_for(difficulty: Option<u8>) -> Option<u16> {
    difficulty
        .map(usize::from)
        .and_then(|index| BOT_DIFFICULTY_ELO.get(index).copied())
}
