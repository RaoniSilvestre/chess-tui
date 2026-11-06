//! Root application bootstrap and runtime loop.

pub use app::{App, AppResult};

pub mod run;

pub use run::run;

/// Build an application wired to the real page factory.
pub fn test_app() -> App {
    App::new(Box::new(pages::DefaultPageFactory))
}
