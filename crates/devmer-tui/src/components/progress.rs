//! Progress components

use crate::theme::{Symbols, Theme};
use ratatui::{
    layout::Rect,
    style::Style,
    text::Span,
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

/// Progress bar component
pub struct ProgressBar<'a> {
    progress: f64,
    label: Option<&'a str>,
    theme: &'a Theme,
}

impl<'a> ProgressBar<'a> {
    /// Create a new progress bar
    pub fn new(progress: f64, theme: &'a Theme) -> Self {
        Self {
            progress: progress.clamp(0.0, 100.0),
            label: None,
            theme,
        }
    }

    /// Set label
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// Render the progress bar
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let label = self
            .label
            .map(|l| format!("{} ({:.0}%)", l, self.progress))
            .unwrap_or_else(|| format!("{:.0}%", self.progress));

        let gauge = Gauge::default()
            .gauge_style(self.theme.progress_filled())
            .label(label)
            .ratio(self.progress / 100.0);

        frame.render_widget(gauge, area);
    }
}

/// Spinner component
pub struct Spinner<'a> {
    frame: usize,
    message: Option<&'a str>,
    theme: &'a Theme,
}

impl<'a> Spinner<'a> {
    /// Create a new spinner
    pub fn new(frame: usize, theme: &'a Theme) -> Self {
        Self {
            frame,
            message: None,
            theme,
        }
    }

    /// Set message
    pub fn message(mut self, message: &'a str) -> Self {
        self.message = Some(message);
        self
    }

    /// Get current spinner character
    fn current_char(&self) -> &'static str {
        Symbols::SPINNER[self.frame % Symbols::SPINNER.len()]
    }

    /// Render the spinner
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let text = if let Some(msg) = self.message {
            format!("{} {}", self.current_char(), msg)
        } else {
            self.current_char().to_string()
        };

        let paragraph = Paragraph::new(text).style(self.theme.spinner());
        frame.render_widget(paragraph, area);
    }
}

/// Operation progress item
pub struct OperationProgress<'a> {
    name: &'a str,
    status: OperationProgressStatus,
    theme: &'a Theme,
}

/// Status of an operation in progress
pub enum OperationProgressStatus {
    Pending,
    InProgress(usize), // spinner frame
    Succeeded,
    Failed,
}

impl<'a> OperationProgress<'a> {
    /// Create a new operation progress
    pub fn new(name: &'a str, status: OperationProgressStatus, theme: &'a Theme) -> Self {
        Self { name, status, theme }
    }

    /// Render the operation progress
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let (symbol, style) = match &self.status {
            OperationProgressStatus::Pending => (Symbols::PENDING, self.theme.text_muted()),
            OperationProgressStatus::InProgress(f) => {
                (Symbols::SPINNER[*f % Symbols::SPINNER.len()], self.theme.spinner())
            }
            OperationProgressStatus::Succeeded => (Symbols::SUCCESS, self.theme.text_success()),
            OperationProgressStatus::Failed => (Symbols::FAILURE, self.theme.text_error()),
        };

        let text = format!("{} {}", symbol, self.name);
        let paragraph = Paragraph::new(text).style(style);
        frame.render_widget(paragraph, area);
    }
}
