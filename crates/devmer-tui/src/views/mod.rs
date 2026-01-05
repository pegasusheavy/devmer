//! Application views

mod dashboard;
mod deployment;
mod preview;
mod resources;
mod state_browser;

pub use dashboard::DashboardView;
pub use deployment::DeploymentView;
pub use preview::PreviewView;
pub use resources::{ResourceDetailsView, ResourcesView};
pub use state_browser::StateBrowserView;
