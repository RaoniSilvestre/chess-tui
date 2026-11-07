//! Application lifecycle, terminal presentation, and page dispatch.

#![warn(dead_code)]
#![warn(unreachable_pub)]

pub mod animations;
mod app;
pub mod event;
pub mod handlers;
pub mod logging;
pub mod run;
pub mod setup;
pub mod tui;

pub use app::{App, AppResult};
pub use pages::{BotGameConfig, Globals, Page, PageAction, PageKind, PageRequest, Theme};
pub use setup::AppSetup;
