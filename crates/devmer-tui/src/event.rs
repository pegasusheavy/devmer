//! Event handling for the TUI

use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use futures::{FutureExt, StreamExt};
use std::time::Duration;
use tokio::sync::mpsc;

/// Application events
#[derive(Debug, Clone)]
pub enum Event {
    /// Terminal tick for animations/updates
    Tick,
    /// Keyboard input
    Key(KeyEvent),
    /// Mouse input
    Mouse(MouseEvent),
    /// Terminal resize
    Resize(u16, u16),
    /// Deployment progress update
    DeploymentProgress(DeploymentProgressEvent),
    /// Resource state changed
    ResourceUpdate(ResourceUpdateEvent),
    /// Error occurred
    Error(String),
    /// Quit the application
    Quit,
}

/// Deployment progress event data
#[derive(Debug, Clone)]
pub struct DeploymentProgressEvent {
    /// Resource URN
    pub urn: String,
    /// Resource name
    pub name: String,
    /// Operation being performed
    pub operation: OperationType,
    /// Current status
    pub status: OperationStatus,
    /// Optional message
    pub message: Option<String>,
}

/// Type of operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationType {
    Create,
    Update,
    Delete,
    Replace,
    Refresh,
    Import,
}

impl std::fmt::Display for OperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Create => write!(f, "Creating"),
            Self::Update => write!(f, "Updating"),
            Self::Delete => write!(f, "Deleting"),
            Self::Replace => write!(f, "Replacing"),
            Self::Refresh => write!(f, "Refreshing"),
            Self::Import => write!(f, "Importing"),
        }
    }
}

/// Status of an operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationStatus {
    Pending,
    InProgress,
    Succeeded,
    Failed,
    Skipped,
}

/// Resource update event data
#[derive(Debug, Clone)]
pub struct ResourceUpdateEvent {
    /// Resource URN
    pub urn: String,
    /// Update type
    pub update_type: ResourceUpdateType,
}

/// Type of resource update
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceUpdateType {
    Created,
    Updated,
    Deleted,
    DriftDetected,
}

/// Event handler that polls for terminal events
pub struct EventHandler {
    /// Event sender
    sender: mpsc::UnboundedSender<Event>,
    /// Event receiver
    receiver: mpsc::UnboundedReceiver<Event>,
    /// Handler task
    handler: Option<tokio::task::JoinHandle<()>>,
}

impl EventHandler {
    /// Create a new event handler
    pub fn new(tick_rate: Duration) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let handler_sender = sender.clone();

        let handler = tokio::spawn(async move {
            let mut reader = crossterm::event::EventStream::new();
            let mut tick_interval = tokio::time::interval(tick_rate);

            loop {
                let tick_delay = tick_interval.tick();
                let crossterm_event = reader.next().fuse();

                tokio::select! {
                    _ = tick_delay => {
                        if handler_sender.send(Event::Tick).is_err() {
                            break;
                        }
                    }
                    Some(Ok(evt)) = crossterm_event => {
                        match evt {
                            CrosstermEvent::Key(key) => {
                                // Check for quit shortcuts
                                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                                    let _ = handler_sender.send(Event::Quit);
                                } else if key.code == KeyCode::Char('q') {
                                    let _ = handler_sender.send(Event::Quit);
                                } else {
                                    let _ = handler_sender.send(Event::Key(key));
                                }
                            }
                            CrosstermEvent::Mouse(mouse) => {
                                let _ = handler_sender.send(Event::Mouse(mouse));
                            }
                            CrosstermEvent::Resize(w, h) => {
                                let _ = handler_sender.send(Event::Resize(w, h));
                            }
                            _ => {}
                        }
                    }
                }
            }
        });

        Self {
            sender,
            receiver,
            handler: Some(handler),
        }
    }

    /// Get the next event
    pub async fn next(&mut self) -> Option<Event> {
        self.receiver.recv().await
    }

    /// Send an event programmatically
    pub fn send(&self, event: Event) -> Result<(), mpsc::error::SendError<Event>> {
        self.sender.send(event)
    }

    /// Get a sender clone for external use
    pub fn sender(&self) -> mpsc::UnboundedSender<Event> {
        self.sender.clone()
    }
}

impl Drop for EventHandler {
    fn drop(&mut self) {
        if let Some(handler) = self.handler.take() {
            handler.abort();
        }
    }
}

/// Key binding configuration
#[derive(Debug, Clone)]
pub struct KeyBindings {
    /// Quit the application
    pub quit: Vec<KeyBinding>,
    /// Navigate up
    pub up: Vec<KeyBinding>,
    /// Navigate down
    pub down: Vec<KeyBinding>,
    /// Navigate left
    pub left: Vec<KeyBinding>,
    /// Navigate right
    pub right: Vec<KeyBinding>,
    /// Select/confirm
    pub select: Vec<KeyBinding>,
    /// Go back/cancel
    pub back: Vec<KeyBinding>,
    /// Help menu
    pub help: Vec<KeyBinding>,
    /// Search/filter
    pub search: Vec<KeyBinding>,
    /// Refresh
    pub refresh: Vec<KeyBinding>,
    /// Switch tabs
    pub next_tab: Vec<KeyBinding>,
    pub prev_tab: Vec<KeyBinding>,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            quit: vec![
                KeyBinding::new(KeyCode::Char('q'), KeyModifiers::NONE),
                KeyBinding::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            ],
            up: vec![
                KeyBinding::new(KeyCode::Up, KeyModifiers::NONE),
                KeyBinding::new(KeyCode::Char('k'), KeyModifiers::NONE),
            ],
            down: vec![
                KeyBinding::new(KeyCode::Down, KeyModifiers::NONE),
                KeyBinding::new(KeyCode::Char('j'), KeyModifiers::NONE),
            ],
            left: vec![
                KeyBinding::new(KeyCode::Left, KeyModifiers::NONE),
                KeyBinding::new(KeyCode::Char('h'), KeyModifiers::NONE),
            ],
            right: vec![
                KeyBinding::new(KeyCode::Right, KeyModifiers::NONE),
                KeyBinding::new(KeyCode::Char('l'), KeyModifiers::NONE),
            ],
            select: vec![
                KeyBinding::new(KeyCode::Enter, KeyModifiers::NONE),
                KeyBinding::new(KeyCode::Char(' '), KeyModifiers::NONE),
            ],
            back: vec![
                KeyBinding::new(KeyCode::Esc, KeyModifiers::NONE),
                KeyBinding::new(KeyCode::Backspace, KeyModifiers::NONE),
            ],
            help: vec![
                KeyBinding::new(KeyCode::Char('?'), KeyModifiers::NONE),
                KeyBinding::new(KeyCode::F(1), KeyModifiers::NONE),
            ],
            search: vec![
                KeyBinding::new(KeyCode::Char('/'), KeyModifiers::NONE),
                KeyBinding::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
            ],
            refresh: vec![
                KeyBinding::new(KeyCode::Char('r'), KeyModifiers::NONE),
                KeyBinding::new(KeyCode::F(5), KeyModifiers::NONE),
            ],
            next_tab: vec![
                KeyBinding::new(KeyCode::Tab, KeyModifiers::NONE),
                KeyBinding::new(KeyCode::Char(']'), KeyModifiers::NONE),
            ],
            prev_tab: vec![
                KeyBinding::new(KeyCode::BackTab, KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::Char('['), KeyModifiers::NONE),
            ],
        }
    }
}

/// A single key binding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyBinding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyBinding {
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    pub fn matches(&self, event: &KeyEvent) -> bool {
        self.code == event.code && self.modifiers == event.modifiers
    }
}

impl KeyBindings {
    /// Check if a key event matches quit
    pub fn is_quit(&self, event: &KeyEvent) -> bool {
        self.quit.iter().any(|b| b.matches(event))
    }

    /// Check if a key event matches up
    pub fn is_up(&self, event: &KeyEvent) -> bool {
        self.up.iter().any(|b| b.matches(event))
    }

    /// Check if a key event matches down
    pub fn is_down(&self, event: &KeyEvent) -> bool {
        self.down.iter().any(|b| b.matches(event))
    }

    /// Check if a key event matches left
    pub fn is_left(&self, event: &KeyEvent) -> bool {
        self.left.iter().any(|b| b.matches(event))
    }

    /// Check if a key event matches right
    pub fn is_right(&self, event: &KeyEvent) -> bool {
        self.right.iter().any(|b| b.matches(event))
    }

    /// Check if a key event matches select
    pub fn is_select(&self, event: &KeyEvent) -> bool {
        self.select.iter().any(|b| b.matches(event))
    }

    /// Check if a key event matches back
    pub fn is_back(&self, event: &KeyEvent) -> bool {
        self.back.iter().any(|b| b.matches(event))
    }

    /// Check if a key event matches help
    pub fn is_help(&self, event: &KeyEvent) -> bool {
        self.help.iter().any(|b| b.matches(event))
    }

    /// Check if a key event matches search
    pub fn is_search(&self, event: &KeyEvent) -> bool {
        self.search.iter().any(|b| b.matches(event))
    }

    /// Check if a key event matches refresh
    pub fn is_refresh(&self, event: &KeyEvent) -> bool {
        self.refresh.iter().any(|b| b.matches(event))
    }

    /// Check if a key event matches next tab
    pub fn is_next_tab(&self, event: &KeyEvent) -> bool {
        self.next_tab.iter().any(|b| b.matches(event))
    }

    /// Check if a key event matches prev tab
    pub fn is_prev_tab(&self, event: &KeyEvent) -> bool {
        self.prev_tab.iter().any(|b| b.matches(event))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_binding() {
        let binding = KeyBinding::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let event = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(binding.matches(&event));

        let wrong_event = KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE);
        assert!(!binding.matches(&wrong_event));
    }

    #[test]
    fn test_key_bindings() {
        let bindings = KeyBindings::default();
        
        let quit_event = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(bindings.is_quit(&quit_event));

        let up_event = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert!(bindings.is_up(&up_event));

        let vim_up_event = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        assert!(bindings.is_up(&vim_up_event));
    }
}
