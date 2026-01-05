//! `devmer preview` command

use anyhow::{Context, Result};
use devmer_config::ConfigLoader;
use devmer_di::{AppContainer, ChangeType};

use crate::output;

/// Execute the preview command
pub async fn execute(stack: Option<String>, diff: bool, json: bool) -> Result<()> {
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

    if !json {
        output::banner(&format!("Previewing {} / {}", project_name, stack_name));
    }

    // Create the DI container
    let container = AppContainer::new(config);

    // Get the execution service
    let execution_service = container.execution_service();

    // Run preview
    let result = execution_service
        .preview(&stack_name)
        .await
        .context("Preview failed")?;

    // Output results
    if json {
        let creates: Vec<serde_json::Value> = result
            .creates
            .iter()
            .map(|c| {
                serde_json::json!({
                    "urn": c.urn,
                    "type": c.resource_type,
                    "name": c.name
                })
            })
            .collect();

        let updates: Vec<serde_json::Value> = result
            .updates
            .iter()
            .map(|c| {
                serde_json::json!({
                    "urn": c.urn,
                    "type": c.resource_type,
                    "name": c.name
                })
            })
            .collect();

        let deletes: Vec<serde_json::Value> = result
            .deletes
            .iter()
            .map(|c| {
                serde_json::json!({
                    "urn": c.urn,
                    "type": c.resource_type,
                    "name": c.name
                })
            })
            .collect();

        let output = serde_json::json!({
            "stack": stack_name,
            "creates": creates,
            "updates": updates,
            "deletes": deletes,
            "same": result.same
        });

        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        let total_changes = result.creates.len() + result.updates.len() + result.deletes.len();

        if total_changes == 0 && result.same == 0 {
            output::info("No resources defined yet.");
            output::info("Add resources to your program and run preview again.");
        } else {
            // Show creates
            for change in &result.creates {
                output::resource_change("create", &change.resource_type, &change.name);
                if diff {
                    for d in &change.diffs {
                        output::property_diff(&d.path, d.old_value.as_deref(), d.new_value.as_deref());
                    }
                }
            }

            // Show updates
            for change in &result.updates {
                let change_type = match change.change_type {
                    ChangeType::Replace => "replace",
                    _ => "update",
                };
                output::resource_change(change_type, &change.resource_type, &change.name);
                if diff {
                    for d in &change.diffs {
                        output::property_diff(&d.path, d.old_value.as_deref(), d.new_value.as_deref());
                    }
                }
            }

            // Show deletes
            for change in &result.deletes {
                output::resource_change("delete", &change.resource_type, &change.name);
            }
        }

        println!();
        output::summary(
            result.creates.len(),
            result.updates.len(),
            result.deletes.len(),
            result.same,
        );
    }

    Ok(())
}
