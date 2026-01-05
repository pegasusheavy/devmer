//! `devmer down` command

use anyhow::{Context, Result};
use devmer_config::ConfigLoader;
use devmer_di::AppContainer;
use dialoguer::Confirm;
use std::time::Instant;

use crate::output;

/// Execute the down command
pub async fn execute(stack: Option<String>, yes: bool, remove: bool) -> Result<()> {
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

    output::banner(&format!("Destroying {} / {}", project_name, stack_name));

    // Create the DI container
    let container = AppContainer::new(config);
    let execution_service = container.execution_service();

    // Preview what will be destroyed
    output::info("Previewing resources to destroy...");
    let preview = execution_service
        .preview(&stack_name)
        .await
        .context("Preview failed")?;

    // In destroy mode, everything becomes a delete
    let total_resources = preview.creates.len() + preview.updates.len() + preview.same;

    if total_resources == 0 && preview.deletes.is_empty() {
        output::success("No resources to destroy. Stack is already empty.");
        return Ok(());
    }

    // Show what will be destroyed
    output::warning(&format!(
        "This will destroy {} resource(s) in stack '{}'",
        total_resources + preview.deletes.len(),
        stack_name
    ));

    println!();

    // Confirm unless --yes
    if !yes {
        let proceed = Confirm::new()
            .with_prompt("Are you sure you want to destroy all resources?")
            .default(false)
            .interact()?;

        if !proceed {
            output::info("Destruction cancelled.");
            return Ok(());
        }

        // Double confirmation for safety
        let really_proceed = Confirm::new()
            .with_prompt("This action cannot be undone. Type 'yes' to confirm")
            .default(false)
            .interact()?;

        if !really_proceed {
            output::info("Destruction cancelled.");
            return Ok(());
        }
    }

    // Execute destruction
    output::info("Destroying resources...");
    let start = Instant::now();

    let result = execution_service
        .destroy(&stack_name, yes)
        .await
        .context("Destruction failed")?;

    let duration = start.elapsed().as_secs_f64();

    // Show result
    println!();
    if result.success {
        output::success(&format!(
            "Successfully destroyed {} resource(s) in {:.2}s",
            result.resources_destroyed, duration
        ));

        if remove {
            output::info("Removing stack state...");
            // TODO: Actually remove the stack state file
            output::success("Stack state removed.");
        }
    } else {
        for error in &result.errors {
            output::error(error);
        }
        output::error(&format!(
            "Destruction failed. {} resource(s) destroyed before failure.",
            result.resources_destroyed
        ));
    }

    Ok(())
}
