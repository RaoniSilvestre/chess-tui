//! Root application bootstrap and runtime loop.

pub mod run;

pub use run::run;

/// Build an application wired to the real page factory.
pub fn new_app() -> app::App {
    app::App::new(Box::new(pages::DefaultPageFactory))
}
