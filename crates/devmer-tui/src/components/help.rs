//! Help overlay component

use crate::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Help overlay component
pub struct HelpOverlay<'a> {
    theme: &'a Theme,
}

impl<'a> HelpOverlay<'a> {
    /// Create a new help overlay
    pub fn new(theme: &'a Theme) -> Self {
        Self { theme }
    }

    /// Render the help overlay
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Calculate centered area
        let popup_area = centered_rect(60, 70, area);

        // Clear the background
        frame.render_widget(Clear, popup_area);

        // Build help content
        let sections = vec![
            ("Navigation", vec![
                ("↑/k", "Move up"),
                ("↓/j", "Move down"),
                ("←/h", "Move left"),
                ("→/l", "Move right"),
                ("Tab", "Next tab"),
                ("Shift+Tab", "Previous tab"),
                ("Enter/Space", "Select"),
                ("Esc/Backspace", "Go back"),
            ]),
            ("Actions", vec![
                ("p", "Preview changes"),
                ("u", "Deploy (up)"),
                ("d", "Destroy (down)"),
                ("r", "Refresh state"),
                ("s", "Select stack"),
            ]),
            ("Views", vec![
                ("1", "Dashboard"),
                ("2", "Resources"),
                ("3", "Stacks"),
                ("4", "State browser"),
            ]),
            ("General", vec![
                ("?/F1", "Toggle help"),
                ("/", "Search"),
                ("q/Ctrl+c", "Quit"),
            ]),
        ];

        let mut lines: Vec<Line> = Vec::new();

        for (section_name, bindings) in sections {
            // Section header
            lines.push(Line::from(Span::styled(
                section_name,
                self.theme.text_primary(),
            )));
            lines.push(Line::from(""));

            // Key bindings
            for (key, desc) in bindings {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(format!("{:>12}", key), self.theme.text_info()),
                    Span::raw("  "),
                    Span::styled(desc, self.theme.text()),
                ]));
            }

            lines.push(Line::from(""));
        }

        let text = Text::from(lines);
        let help_paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .title(Span::styled(" Help ", self.theme.title()))
                    .borders(Borders::ALL)
                    .border_style(self.theme.block_focused()),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(help_paragraph, popup_area);
    }
}

/// Create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
