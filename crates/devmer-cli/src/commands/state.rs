//! State management commands

use anyhow::{Context, Result};
use colored::Colorize;
use devmer_config::ConfigLoader;
use devmer_core::state::StackState;
use devmer_di::AppContainer;
use dialoguer::Confirm;
use std::fs;
use std::path::Path;

use crate::commands::stack::get_current_stack;
use crate::output;

/// Export state to file
pub async fn export(file: Option<String>, stack: Option<String>) -> Result<()> {
    // Load configuration
    let config = ConfigLoader::current_dir()
        .context("Failed to get current directory")?
        .load()
        .context("Failed to load Devmer.toml. Run 'devmer init' to create one.")?;

    let stack_name = stack.unwrap_or_else(|| get_current_stack(&config));
    let filename = file.unwrap_or_else(|| format!("{}-state.json", stack_name));

    output::info(&format!(
        "Exporting state for '{}' to '{}'",
        stack_name, filename
    ));

    // Create DI container
    let container = AppContainer::new(config);
    let state_service = container.state_service();

    // Get state
    let state = state_service
        .get_state(&stack_name)
        .await
        .context("Failed to read state")?;

    if let Some(s) = state {
        // Write to file
        let content = serde_json::to_string_pretty(&s)?;
        fs::write(&filename, content)?;

        output::success(&format!("Exported state to '{}'", filename));
        output::info(&format!("  {} resources", s.resource_count()));
    } else {
        output::warning(&format!("No state found for stack '{}'", stack_name));
    }

    Ok(())
}

/// Import state from file
pub async fn import(file: &str, stack: Option<String>) -> Result<()> {
    // Load configuration
    let config = ConfigLoader::current_dir()
        .context("Failed to get current directory")?
        .load()
        .context("Failed to load Devmer.toml. Run 'devmer init' to create one.")?;

    let stack_name = stack.unwrap_or_else(|| get_current_stack(&config));

    // Check if file exists
    if !Path::new(file).exists() {
        anyhow::bail!("State file not found: {}", file);
    }

    output::info(&format!(
        "Importing state for '{}' from '{}'",
        stack_name, file
    ));

    // Read and parse state file
    let content = fs::read_to_string(file).context("Failed to read state file")?;
    let imported_state: StackState =
        serde_json::from_str(&content).context("Failed to parse state file")?;

    // Create DI container
    let container = AppContainer::new(config);
    let state_service = container.state_service();

    // Check if stack already has state
    let existing = state_service.get_state(&stack_name).await.ok().flatten();
    if let Some(existing) = existing {
        let count = existing.resource_count();
        if count > 0 {
            output::warning(&format!(
                "Stack '{}' already has {} resource(s)",
                stack_name, count
            ));

            let proceed = Confirm::new()
                .with_prompt("Overwrite existing state?")
                .default(false)
                .interact()?;

            if !proceed {
                output::info("Import cancelled.");
                return Ok(());
            }
        }
    }

    // Save imported state
    state_service
        .save_state(&stack_name, &imported_state)
        .await
        .context("Failed to save state")?;

    output::success("State imported successfully.");
    output::info(&format!("  {} resources", imported_state.resource_count()));

    Ok(())
}

/// Unlock state (force-remove lock)
pub async fn unlock(stack: Option<String>) -> Result<()> {
    // Load configuration
    let config = ConfigLoader::current_dir()
        .context("Failed to get current directory")?
        .load()
        .context("Failed to load Devmer.toml. Run 'devmer init' to create one.")?;

    let stack_name = stack.unwrap_or_else(|| get_current_stack(&config));

    // Create DI container
    let container = AppContainer::new(config);
    let state_service = container.state_service();

    // Check lock status
    let lock_status = state_service
        .get_lock_status(&stack_name)
        .await
        .context("Failed to get lock status")?;

    match lock_status {
        devmer_state::locking::LockStatus::Locked(info)
        | devmer_state::locking::LockStatus::LockedByOther(info) => {
            output::warning(&format!(
                "Stack '{}' is locked by: {}",
                stack_name, info.owner
            ));
            output::warning(&format!(
                "Locked at: {}",
                info.created_at.format("%Y-%m-%d %H:%M:%S")
            ));
            output::warning(&format!("Operation: {}", info.operation));

            println!();
            output::warning(
                "Force-unlocking state can be dangerous if another process is still running!",
            );

            let proceed = Confirm::new()
                .with_prompt("Are you sure you want to force-unlock?")
                .default(false)
                .interact()?;

            if !proceed {
                output::info("Cancelled.");
                return Ok(());
            }

            // Force unlock using a dummy lock ID
            let dummy_lock_id = devmer_state::locking::LockId::new();
            state_service
                .unlock(&stack_name, &dummy_lock_id)
                .await
                .context("Failed to unlock state")?;

            output::success(&format!("State for '{}' unlocked.", stack_name));
        }
        devmer_state::locking::LockStatus::Unlocked => {
            output::info(&format!("Stack '{}' is not locked.", stack_name));
        }
        devmer_state::locking::LockStatus::LockedByUs(info) => {
            output::info(&format!(
                "Stack '{}' is locked by us (operation: {})",
                stack_name, info.operation
            ));
        }
        devmer_state::locking::LockStatus::Expired(info) => {
            output::warning(&format!(
                "Stack '{}' has an expired lock from: {}",
                stack_name, info.owner
            ));
            // Force unlock expired lock
            let dummy_lock_id = devmer_state::locking::LockId::new();
            let _ = state_service.unlock(&stack_name, &dummy_lock_id).await;
            output::success("Expired lock cleared.");
        }
    }

    Ok(())
}

/// Delete resource from state (without destroying the actual resource)
pub async fn delete(urn: &str, stack: Option<String>) -> Result<()> {
    // Load configuration
    let config = ConfigLoader::current_dir()
        .context("Failed to get current directory")?
        .load()
        .context("Failed to load Devmer.toml. Run 'devmer init' to create one.")?;

    let stack_name = stack.unwrap_or_else(|| get_current_stack(&config));

    output::warning(&format!(
        "Deleting resource '{}' from state (stack: {})",
        urn, stack_name
    ));
    output::warning("This will NOT delete the actual resource in the cloud.");
    output::warning("The resource will appear as 'new' on the next preview/deploy.");

    // Confirm
    let proceed = Confirm::new()
        .with_prompt("Are you sure you want to remove this resource from state?")
        .default(false)
        .interact()?;

    if !proceed {
        output::info("Cancelled.");
        return Ok(());
    }

    // Create DI container
    let container = AppContainer::new(config);
    let state_service = container.state_service();

    // Get current state
    let mut state = state_service
        .get_state(&stack_name)
        .await
        .context("Failed to read state")?
        .ok_or_else(|| anyhow::anyhow!("No state found for stack '{}'", stack_name))?;

    // Find and remove the resource
    let removed = state.remove_resource(urn);

    if removed.is_some() {
        // Save updated state
        state_service
            .save_state(&stack_name, &state)
            .await
            .context("Failed to save state")?;

        output::success("Resource removed from state.");
    } else {
        output::error(&format!("Resource '{}' not found in state", urn));

        // Show similar resources
        let similar: Vec<_> = state
            .resources()
            .filter(|r| {
                r.urn.as_str().contains(urn) || urn.contains(r.urn.as_str())
            })
            .take(5)
            .collect();

        if !similar.is_empty() {
            println!();
            output::info("Did you mean one of these?");
            for r in similar {
                println!("  {}", r.urn.as_str().cyan());
            }
        }
    }

    Ok(())
}
