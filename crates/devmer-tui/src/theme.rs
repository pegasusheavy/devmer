//! Theme and color definitions for the TUI

use ratatui::style::{Color, Modifier, Style};

/// Theme configuration for the TUI
#[derive(Debug, Clone)]
pub struct Theme {
    /// Primary accent color
    pub primary: Color,
    /// Secondary accent color
    pub secondary: Color,
    /// Success/positive color
    pub success: Color,
    /// Warning color
    pub warning: Color,
    /// Error/danger color
    pub error: Color,
    /// Info color
    pub info: Color,
    /// Background color
    pub bg: Color,
    /// Foreground/text color
    pub fg: Color,
    /// Muted/dimmed text color
    pub muted: Color,
    /// Border color
    pub border: Color,
    /// Highlight/selection color
    pub highlight: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    /// Create a dark theme
    pub fn dark() -> Self {
        Self {
            primary: Color::Rgb(99, 102, 241),    // Indigo
            secondary: Color::Rgb(139, 92, 246),  // Purple
            success: Color::Rgb(34, 197, 94),     // Green
            warning: Color::Rgb(234, 179, 8),     // Yellow
            error: Color::Rgb(239, 68, 68),       // Red
            info: Color::Rgb(59, 130, 246),       // Blue
            bg: Color::Rgb(17, 24, 39),           // Dark gray
            fg: Color::Rgb(243, 244, 246),        // Light gray
            muted: Color::Rgb(107, 114, 128),     // Gray
            border: Color::Rgb(55, 65, 81),       // Medium gray
            highlight: Color::Rgb(31, 41, 55),    // Selection bg
        }
    }

    /// Create a light theme
    pub fn light() -> Self {
        Self {
            primary: Color::Rgb(79, 70, 229),     // Indigo
            secondary: Color::Rgb(124, 58, 237),  // Purple
            success: Color::Rgb(22, 163, 74),     // Green
            warning: Color::Rgb(202, 138, 4),     // Yellow
            error: Color::Rgb(220, 38, 38),       // Red
            info: Color::Rgb(37, 99, 235),        // Blue
            bg: Color::Rgb(255, 255, 255),        // White
            fg: Color::Rgb(17, 24, 39),           // Dark
            muted: Color::Rgb(107, 114, 128),     // Gray
            border: Color::Rgb(209, 213, 219),    // Light gray
            highlight: Color::Rgb(243, 244, 246), // Selection bg
        }
    }

    // =========================================================================
    // Style helpers
    // =========================================================================

    /// Default text style
    pub fn text(&self) -> Style {
        Style::default().fg(self.fg)
    }

    /// Muted/dimmed text style
    pub fn text_muted(&self) -> Style {
        Style::default().fg(self.muted)
    }

    /// Bold text style
    pub fn text_bold(&self) -> Style {
        Style::default().fg(self.fg).add_modifier(Modifier::BOLD)
    }

    /// Primary colored text
    pub fn text_primary(&self) -> Style {
        Style::default().fg(self.primary)
    }

    /// Success colored text
    pub fn text_success(&self) -> Style {
        Style::default().fg(self.success)
    }

    /// Warning colored text
    pub fn text_warning(&self) -> Style {
        Style::default().fg(self.warning)
    }

    /// Error colored text
    pub fn text_error(&self) -> Style {
        Style::default().fg(self.error)
    }

    /// Info colored text
    pub fn text_info(&self) -> Style {
        Style::default().fg(self.info)
    }

    /// Block/border style
    pub fn block(&self) -> Style {
        Style::default().fg(self.border)
    }

    /// Focused block/border style
    pub fn block_focused(&self) -> Style {
        Style::default().fg(self.primary)
    }

    /// Title style
    pub fn title(&self) -> Style {
        Style::default()
            .fg(self.fg)
            .add_modifier(Modifier::BOLD)
    }

    /// Title style (focused)
    pub fn title_focused(&self) -> Style {
        Style::default()
            .fg(self.primary)
            .add_modifier(Modifier::BOLD)
    }

    /// Highlight/selected item style
    pub fn highlight(&self) -> Style {
        Style::default()
            .bg(self.highlight)
            .fg(self.fg)
    }

    /// Selected item with primary color
    pub fn selected(&self) -> Style {
        Style::default()
            .bg(self.primary)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    }

    /// Status bar style
    pub fn status_bar(&self) -> Style {
        Style::default()
            .bg(self.border)
            .fg(self.fg)
    }

    /// Tab style (inactive)
    pub fn tab(&self) -> Style {
        Style::default().fg(self.muted)
    }

    /// Tab style (active)
    pub fn tab_active(&self) -> Style {
        Style::default()
            .fg(self.primary)
            .add_modifier(Modifier::BOLD)
    }

    // =========================================================================
    // Resource operation styles
    // =========================================================================

    /// Style for create operations
    pub fn op_create(&self) -> Style {
        Style::default()
            .fg(self.success)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for update operations
    pub fn op_update(&self) -> Style {
        Style::default()
            .fg(self.warning)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for delete operations
    pub fn op_delete(&self) -> Style {
        Style::default()
            .fg(self.error)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for replace operations
    pub fn op_replace(&self) -> Style {
        Style::default()
            .fg(self.secondary)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for unchanged resources
    pub fn op_unchanged(&self) -> Style {
        Style::default().fg(self.muted)
    }

    // =========================================================================
    // Diff styles
    // =========================================================================

    /// Style for added lines in diff
    pub fn diff_add(&self) -> Style {
        Style::default()
            .fg(self.success)
            .bg(Color::Rgb(22, 101, 52)) // Dark green bg
    }

    /// Style for removed lines in diff
    pub fn diff_remove(&self) -> Style {
        Style::default()
            .fg(self.error)
            .bg(Color::Rgb(127, 29, 29)) // Dark red bg
    }

    /// Style for context lines in diff
    pub fn diff_context(&self) -> Style {
        Style::default().fg(self.muted)
    }

    /// Style for diff header
    pub fn diff_header(&self) -> Style {
        Style::default()
            .fg(self.info)
            .add_modifier(Modifier::BOLD)
    }

    // =========================================================================
    // Progress styles
    // =========================================================================

    /// Progress bar filled style
    pub fn progress_filled(&self) -> Style {
        Style::default().fg(self.primary)
    }

    /// Progress bar unfilled style
    pub fn progress_unfilled(&self) -> Style {
        Style::default().fg(self.border)
    }

    /// Spinner style
    pub fn spinner(&self) -> Style {
        Style::default().fg(self.primary)
    }
}

/// Symbols used in the TUI
pub struct Symbols;

impl Symbols {
    // Resource operations
    pub const CREATE: &'static str = "+";
    pub const UPDATE: &'static str = "~";
    pub const DELETE: &'static str = "-";
    pub const REPLACE: &'static str = "±";
    pub const UNCHANGED: &'static str = " ";

    // Tree structure
    pub const TREE_BRANCH: &'static str = "├──";
    pub const TREE_LAST: &'static str = "└──";
    pub const TREE_VERTICAL: &'static str = "│  ";
    pub const TREE_SPACE: &'static str = "   ";

    // Status indicators
    pub const SUCCESS: &'static str = "✓";
    pub const FAILURE: &'static str = "✗";
    pub const PENDING: &'static str = "○";
    pub const RUNNING: &'static str = "●";
    pub const WARNING: &'static str = "⚠";

    // Navigation
    pub const ARROW_RIGHT: &'static str = "→";
    pub const ARROW_LEFT: &'static str = "←";
    pub const ARROW_UP: &'static str = "↑";
    pub const ARROW_DOWN: &'static str = "↓";

    // Misc
    pub const BULLET: &'static str = "•";
    pub const ELLIPSIS: &'static str = "…";
    pub const SEPARATOR: &'static str = "│";

    // Spinner frames
    pub const SPINNER: &'static [&'static str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dark_theme() {
        let theme = Theme::dark();
        assert_eq!(theme.bg, Color::Rgb(17, 24, 39));
    }

    #[test]
    fn test_light_theme() {
        let theme = Theme::light();
        assert_eq!(theme.bg, Color::Rgb(255, 255, 255));
    }
}
