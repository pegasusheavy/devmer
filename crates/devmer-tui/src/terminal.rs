//! Terminal handling for the TUI

use crate::{Result, TuiError};
use crossterm::{
    cursor,
    event::{DisableMouseCapture, EnableMouseCapture},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use std::io::{self, Stdout};
use std::panic;

/// Type alias for the terminal
pub type Terminal = ratatui::Terminal<CrosstermBackend<Stdout>>;

/// Initialize the terminal for TUI mode
pub fn init() -> Result<Terminal> {
    // Set up panic hook to restore terminal on panic
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = restore();
        original_hook(panic_info);
    }));

    // Enable raw mode
    terminal::enable_raw_mode().map_err(|e| TuiError::Terminal(e.to_string()))?;

    // Enter alternate screen
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .map_err(|e| TuiError::Terminal(e.to_string()))?;

    // Create terminal
    let backend = CrosstermBackend::new(stdout);
    let terminal =
        ratatui::Terminal::new(backend).map_err(|e| TuiError::Terminal(e.to_string()))?;

    Ok(terminal)
}

/// Restore the terminal to normal mode
pub fn restore() -> Result<()> {
    // Disable raw mode
    terminal::disable_raw_mode().map_err(|e| TuiError::Terminal(e.to_string()))?;

    // Leave alternate screen
    crossterm::execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)
        .map_err(|e| TuiError::Terminal(e.to_string()))?;

    // Show cursor
    crossterm::execute!(io::stdout(), cursor::Show)
        .map_err(|e| TuiError::Terminal(e.to_string()))?;

    Ok(())
}

/// Terminal guard that restores terminal on drop
pub struct TerminalGuard {
    terminal: Terminal,
}

impl TerminalGuard {
    /// Create a new terminal guard
    pub fn new() -> Result<Self> {
        let terminal = init()?;
        Ok(Self { terminal })
    }

    /// Get a reference to the terminal
    pub fn terminal(&self) -> &Terminal {
        &self.terminal
    }

    /// Get a mutable reference to the terminal
    pub fn terminal_mut(&mut self) -> &mut Terminal {
        &mut self.terminal
    }

    /// Take ownership of the terminal
    pub fn into_terminal(self) -> Terminal {
        // Don't run drop
        let terminal = self.terminal;
        std::mem::forget(self);
        terminal
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = restore();
    }
}

impl std::ops::Deref for TerminalGuard {
    type Target = Terminal;

    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl std::ops::DerefMut for TerminalGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}
