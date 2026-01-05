//! `devmer up` command

use anyhow::{Context, Result};
use devmer_config::ConfigLoader;
use devmer_di::AppContainer;
use dialoguer::Confirm;
use std::time::Instant;

use crate::output;

/// Execute the up command
pub async fn execute(
    stack: Option<String>,
    yes: bool,
    refresh: bool,
    _parallel: usize,
) -> Result<()> {
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

    output::banner(&format!("Deploying {} / {}", project_name, stack_name));

    // Create the DI container
    let container = AppContainer::new(config);
    let execution_service = container.execution_service();

    // Refresh state if requested
    if refresh {
        output::info("Refreshing state from cloud...");
        let refresh_result = execution_service
            .refresh(&stack_name)
            .await
            .context("Refresh failed")?;

        if refresh_result.drift_detected > 0 {
            output::warning(&format!(
                "Drift detected in {} resource(s)",
                refresh_result.drift_detected
            ));
        }
    }

    // Preview changes first
    output::info("Previewing changes...");
    let preview = execution_service
        .preview(&stack_name)
        .await
        .context("Preview failed")?;

    let total_changes = preview.creates.len() + preview.updates.len() + preview.deletes.len();

    if total_changes == 0 {
        // Even with no changes, ensure state file is initialized
        let _ = execution_service
            .deploy(&stack_name, true)
            .await
            .context("Failed to initialize state")?;
        output::success("No changes to deploy. Infrastructure is up to date.");
        return Ok(());
    }

    // Show summary
    println!();
    output::info(&format!(
        "Changes: {} to create, {} to update, {} to delete",
        preview.creates.len(),
        preview.updates.len(),
        preview.deletes.len()
    ));

    // Show resources to be changed
    for change in &preview.creates {
        output::resource_change("create", &change.resource_type, &change.name);
    }
    for change in &preview.updates {
        output::resource_change("update", &change.resource_type, &change.name);
    }
    for change in &preview.deletes {
        output::resource_change("delete", &change.resource_type, &change.name);
    }

    println!();

    // Confirm unless --yes
    if !yes {
        let proceed = Confirm::new()
            .with_prompt("Do you want to perform this deployment?")
            .default(false)
            .interact()?;

        if !proceed {
            output::info("Deployment cancelled.");
            return Ok(());
        }
    }

    // Execute deployment
    output::info("Deploying...");
    let start = Instant::now();

    let result = execution_service
        .deploy(&stack_name, yes)
        .await
        .context("Deployment failed")?;

    let duration = start.elapsed().as_secs_f64();

    // Show result
    println!();
    if result.success {
        output::deploy_result(
            true,
            result.resources_created,
            result.resources_updated,
            result.resources_deleted,
            duration,
        );
    } else {
        for error in &result.errors {
            output::error(error);
        }
        output::deploy_result(
            false,
            result.resources_created,
            result.resources_updated,
            result.resources_deleted,
            duration,
        );
    }

    Ok(())
}
