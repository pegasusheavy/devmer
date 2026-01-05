//! # devmer-tui
//!
//! Terminal User Interface for Devmer using ratatui.
//!
//! This crate provides an interactive TUI for:
//! - Viewing deployment progress in real-time
//! - Browsing resource trees
//! - Previewing changes with diffs
//! - Managing stacks and state
//!
//! ## Architecture
//!
//! The TUI follows an Elm-inspired architecture:
//! - **Model**: Application state
//! - **View**: Render state to terminal
//! - **Update**: Handle events and update state
//!
//! ## Example
//!
//! ```rust,ignore
//! use devmer_tui::App;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let app = App::new()?;
//!     app.run().await
//! }
//! ```

pub mod app;
pub mod components;
pub mod event;
pub mod state;
pub mod terminal;
pub mod theme;
pub mod views;

pub use app::App;
pub use event::{Event, EventHandler};
pub use state::AppState;
pub use terminal::Terminal;
pub use theme::Theme;

/// Result type for TUI operations
pub type Result<T> = std::result::Result<T, TuiError>;

/// TUI-specific errors
#[derive(Debug, thiserror::Error)]
pub enum TuiError {
    #[error("Terminal error: {0}")]
    Terminal(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Event handling error: {0}")]
    Event(String),

    #[error("Render error: {0}")]
    Render(String),

    #[error("State error: {0}")]
    State(String),

    #[error("Configuration error: {0}")]
    Config(String),
}
