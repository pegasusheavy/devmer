//! Header component

use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    style::Stylize,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Header component for the TUI
pub struct Header<'a> {
    project_name: Option<&'a str>,
    stack_name: Option<&'a str>,
    theme: &'a Theme,
}

impl<'a> Header<'a> {
    /// Create a new header
    pub fn new(theme: &'a Theme) -> Self {
        Self {
            project_name: None,
            stack_name: None,
            theme,
        }
    }

    /// Set project name
    pub fn project(mut self, name: &'a str) -> Self {
        self.project_name = Some(name);
        self
    }

    /// Set stack name
    pub fn stack(mut self, name: &'a str) -> Self {
        self.stack_name = Some(name);
        self
    }

    /// Render the header
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let mut spans = vec![
            Span::styled("Devmer", self.theme.title()),
            Span::raw(" "),
        ];

        if let Some(project) = self.project_name {
            spans.push(Span::styled(project, self.theme.text_primary()));
        }

        if let Some(stack) = self.stack_name {
            spans.push(Span::styled(" / ", self.theme.text_muted()));
            spans.push(Span::styled(stack, self.theme.text_info()));
        }

        let title = Line::from(spans);

        let block = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(self.theme.block());

        let paragraph = Paragraph::new(title).block(block);

        frame.render_widget(paragraph, area);
    }
}
