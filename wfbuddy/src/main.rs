//! WFBuddy (iced edition).
//!
//! This binary hosts the retained-mode GUI and orchestrates background polling.

mod app;
mod capture;
mod config;

fn main() -> iced::Result {
    // Structured logging. Use `RUST_LOG=info` etc.
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    app::run()
}
