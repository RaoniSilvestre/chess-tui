//! Command-line arguments, persisted configuration, and filesystem paths.

mod args;
mod config;
mod paths;
mod store;

pub use args::Args;
pub use config::{Config, load_or_create};
pub use paths::{chess_tui_dir, config_dir, config_path, resume_dir, skins_path};
pub use store::ConfigStore;
