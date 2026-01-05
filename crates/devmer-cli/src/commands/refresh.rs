//! `devmer refresh` command

use anyhow::{Context, Result};
use devmer_config::ConfigLoader;
use devmer_di::AppContainer;
use std::time::Instant;

use crate::output;

/// Execute the refresh command
pub async fn execute(stack: Option<String>) -> Result<()> {
    // Load configuration
    let config = ConfigLoader::current_dir()
        .context("Failed to get current directory")?
        .load()
        .context("Failed to load Devmer.toml. Run 'devmer init' to create one.")?;

    let project_name = config.name.clone();

    // Get stack name
    let stack_name = stack.unwrap_or_else(|| {
        config
            .stack_names()
            .first()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "dev".to_string())
    });

    output::banner(&format!("Refreshing {} / {}", project_name, stack_name));

    // Create the DI container
    let container = AppContainer::new(config);
    let execution_service = container.execution_service();

    // Execute refresh
    output::info("Reading resource state from cloud providers...");
    let start = Instant::now();

    let result = execution_service
        .refresh(&stack_name)
        .await
        .context("Refresh failed")?;

    let duration = start.elapsed().as_secs_f64();

    // Show result
    println!();
    if result.success {
        output::success(&format!(
            "Refreshed {} resource(s) in {:.2}s",
            result.resources_refreshed, duration
        ));

        if result.drift_detected > 0 {
            println!();
            output::warning(&format!(
                "Drift detected in {} resource(s)",
                result.drift_detected
            ));
            output::info("Run 'devmer preview' to see the differences.");
            output::info("Run 'devmer up' to reconcile state with your program.");
        } else if result.resources_refreshed > 0 {
            output::info("No drift detected. State is in sync with cloud.");
        }
    } else {
        output::error("Refresh failed. Some resources could not be read.");
    }

    Ok(())
}
