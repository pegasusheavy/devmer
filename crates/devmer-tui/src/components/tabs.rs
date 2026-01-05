//! Tabs component

use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Paragraph, Tabs as RataTabs},
    Frame,
};

/// Tabs component
pub struct Tabs<'a> {
    titles: Vec<&'a str>,
    selected: usize,
    theme: &'a Theme,
}

impl<'a> Tabs<'a> {
    /// Create new tabs
    pub fn new(titles: Vec<&'a str>, selected: usize, theme: &'a Theme) -> Self {
        Self {
            titles,
            selected,
            theme,
        }
    }

    /// Render the tabs
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let titles: Vec<Line> = self
            .titles
            .iter()
            .enumerate()
            .map(|(i, t)| {
                if i == self.selected {
                    Line::from(Span::styled(*t, self.theme.tab_active()))
                } else {
                    Line::from(Span::styled(*t, self.theme.tab()))
                }
            })
            .collect();

        let tabs = RataTabs::new(titles)
            .select(self.selected)
            .divider(" │ ")
            .highlight_style(self.theme.tab_active());

        frame.render_widget(tabs, area);
    }
}
