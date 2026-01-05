//! Status bar component

use crate::state::{StatusLevel, StatusMessage};
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Status bar component
pub struct StatusBar<'a> {
    message: Option<&'a StatusMessage>,
    hints: Vec<(&'a str, &'a str)>,
    theme: &'a Theme,
}

impl<'a> StatusBar<'a> {
    /// Create a new status bar
    pub fn new(theme: &'a Theme) -> Self {
        Self {
            message: None,
            hints: Vec::new(),
            theme,
        }
    }

    /// Set status message
    pub fn message(mut self, message: &'a StatusMessage) -> Self {
        self.message = Some(message);
        self
    }

    /// Add key hint
    pub fn hint(mut self, key: &'a str, description: &'a str) -> Self {
        self.hints.push((key, description));
        self
    }

    /// Render the status bar
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let mut spans = Vec::new();

        // Add status message if present
        if let Some(msg) = self.message {
            let style = match msg.level {
                StatusLevel::Info => self.theme.text_info(),
                StatusLevel::Success => self.theme.text_success(),
                StatusLevel::Warning => self.theme.text_warning(),
                StatusLevel::Error => self.theme.text_error(),
            };
            spans.push(Span::styled(&msg.text, style));
            spans.push(Span::raw("  "));
        }

        // Add hints
        if !self.hints.is_empty() {
            for (i, (key, desc)) in self.hints.iter().enumerate() {
                if i > 0 {
                    spans.push(Span::styled(" │ ", self.theme.text_muted()));
                }
                spans.push(Span::styled(*key, self.theme.text_primary()));
                spans.push(Span::raw(" "));
                spans.push(Span::styled(*desc, self.theme.text_muted()));
            }
        }

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line).style(self.theme.status_bar());

        frame.render_widget(paragraph, area);
    }
}
