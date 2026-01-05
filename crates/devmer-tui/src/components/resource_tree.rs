//! Resource tree component

use crate::state::ResourceInfo;
use crate::theme::{Symbols, Theme};
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use std::collections::HashMap;

/// Resource tree component
pub struct ResourceTree<'a> {
    resources: &'a [ResourceInfo],
    selected: usize,
    theme: &'a Theme,
    title: Option<&'a str>,
    focused: bool,
}

impl<'a> ResourceTree<'a> {
    /// Create a new resource tree
    pub fn new(resources: &'a [ResourceInfo], selected: usize, theme: &'a Theme) -> Self {
        Self {
            resources,
            selected,
            theme,
            title: None,
            focused: false,
        }
    }

    /// Set title
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    /// Set focused state
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Build tree structure from resources
    fn build_tree(&self) -> Vec<TreeNode<'a>> {
        // Build parent-child relationships
        let mut children_map: HashMap<Option<&str>, Vec<&ResourceInfo>> = HashMap::new();

        for resource in self.resources {
            let parent = resource.parent.as_deref();
            children_map.entry(parent).or_default().push(resource);
        }

        // Build tree starting from root (no parent)
        let mut nodes = Vec::new();
        if let Some(roots) = children_map.get(&None) {
            for (i, root) in roots.iter().enumerate() {
                let is_last = i == roots.len() - 1;
                self.build_node(root, &children_map, &mut nodes, "", is_last);
            }
        }

        nodes
    }

    fn build_node(
        &self,
        resource: &'a ResourceInfo,
        children_map: &HashMap<Option<&str>, Vec<&ResourceInfo>>,
        nodes: &mut Vec<TreeNode<'a>>,
        prefix: &str,
        is_last: bool,
    ) {
        let branch = if is_last {
            Symbols::TREE_LAST
        } else {
            Symbols::TREE_BRANCH
        };

        nodes.push(TreeNode {
            resource,
            prefix: format!("{}{}", prefix, branch),
        });

        // Add children
        let children_prefix = if is_last {
            format!("{}{}", prefix, Symbols::TREE_SPACE)
        } else {
            format!("{}{}", prefix, Symbols::TREE_VERTICAL)
        };

        if let Some(children) = children_map.get(&Some(resource.urn.as_str())) {
            for (i, child) in children.iter().enumerate() {
                let child_is_last = i == children.len() - 1;
                self.build_node(child, children_map, nodes, &children_prefix, child_is_last);
            }
        }
    }

    /// Render the resource tree
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let tree_nodes = self.build_tree();

        let items: Vec<ListItem> = tree_nodes
            .iter()
            .enumerate()
            .map(|(i, node)| {
                let is_selected = i == self.selected;
                self.render_node(node, is_selected)
            })
            .collect();

        let mut block = Block::default().borders(Borders::ALL);

        if let Some(title) = self.title {
            let title_style = if self.focused {
                self.theme.title_focused()
            } else {
                self.theme.title()
            };
            block = block.title(Span::styled(title, title_style));
        }

        let border_style = if self.focused {
            self.theme.block_focused()
        } else {
            self.theme.block()
        };
        block = block.border_style(border_style);

        let list = List::new(items)
            .block(block)
            .highlight_style(self.theme.selected());

        let mut state = ListState::default();
        state.select(Some(self.selected));

        frame.render_stateful_widget(list, area, &mut state);
    }

    fn render_node(&self, node: &TreeNode, is_selected: bool) -> ListItem {
        let resource = node.resource;

        let status_style = match resource.status {
            crate::state::ResourceStatus::Ok => self.theme.text_success(),
            crate::state::ResourceStatus::Pending => self.theme.text_muted(),
            crate::state::ResourceStatus::Creating => self.theme.text_info(),
            crate::state::ResourceStatus::Updating => self.theme.text_warning(),
            crate::state::ResourceStatus::Deleting => self.theme.text_error(),
            crate::state::ResourceStatus::Failed => self.theme.text_error(),
            crate::state::ResourceStatus::Drifted => self.theme.text_warning(),
        };

        let status_symbol = match resource.status {
            crate::state::ResourceStatus::Ok => Symbols::SUCCESS,
            crate::state::ResourceStatus::Pending => Symbols::PENDING,
            crate::state::ResourceStatus::Creating | crate::state::ResourceStatus::Updating => {
                Symbols::RUNNING
            }
            crate::state::ResourceStatus::Deleting => Symbols::RUNNING,
            crate::state::ResourceStatus::Failed => Symbols::FAILURE,
            crate::state::ResourceStatus::Drifted => Symbols::WARNING,
        };

        let base_style = if is_selected {
            self.theme.selected()
        } else {
            self.theme.text()
        };

        let spans = vec![
            Span::styled(&node.prefix, self.theme.text_muted()),
            Span::styled(status_symbol, status_style),
            Span::raw(" "),
            Span::styled(&resource.name, base_style),
            Span::raw(" "),
            Span::styled(
                format!("({})", &resource.resource_type),
                self.theme.text_muted(),
            ),
        ];

        ListItem::new(Line::from(spans))
    }
}

struct TreeNode<'a> {
    resource: &'a ResourceInfo,
    prefix: String,
}
