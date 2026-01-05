//! Stack management commands

use anyhow::{Context, Result};
use chrono::Utc;
use colored::Colorize;
use devmer_config::ConfigLoader;
use devmer_di::AppContainer;
use dialoguer::Confirm;
use std::fs;
use std::path::Path;

use crate::output;

/// Workspace state file path
const WORKSPACE_FILE: &str = ".devmer/workspace.json";

/// Workspace state
#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
struct Workspace {
    /// Currently selected stack
    current_stack: Option<String>,
    /// Last updated
    updated_at: Option<String>,
}

impl Workspace {
    fn load() -> Self {
        let path = Path::new(WORKSPACE_FILE);
        if path.exists() {
            fs::read_to_string(path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    fn save(&self) -> Result<()> {
        let path = Path::new(WORKSPACE_FILE);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

/// List all stacks
pub async fn list() -> Result<()> {
    // Load configuration
    let config = ConfigLoader::current_dir()
        .context("Failed to get current directory")?
        .load()
        .context("Failed to load Devmer.toml. Run 'devmer init' to create one.")?;

    let project_name = &config.name;
    let workspace = Workspace::load();

    output::banner(&format!("Stacks for {}", project_name));

    // Get stacks from config
    let config_stacks: Vec<String> = config.stack_names().iter().map(|s| s.to_string()).collect();

    // Create DI container to check state backend
    let container = AppContainer::new(config);
    let state_service = container.state_service();

    // Get stacks from state backend
    let backend_stacks = state_service.list_stacks().await.unwrap_or_default();

    // Combine unique stacks
    let mut all_stacks: Vec<String> = config_stacks;
    for stack in backend_stacks {
        if !all_stacks.contains(&stack) {
            all_stacks.push(stack);
        }
    }

    if all_stacks.is_empty() {
        output::info("No stacks found. Create one with 'devmer stack new <name>'");
        return Ok(());
    }

    // Print stacks
    for stack in &all_stacks {
        let marker = if workspace.current_stack.as_deref() == Some(stack) {
            "*".green().bold()
        } else {
            " ".normal()
        };

        // Check if stack has state
        let state = state_service.get_state(stack).await.ok().flatten();
        let status = if let Some(s) = state {
            format!("{} resources", s.resource_count())
        } else {
            "no resources".dimmed().to_string()
        };

        println!("  {} {}  ({})", marker, stack, status);
    }

    if let Some(current) = &workspace.current_stack {
        println!();
        output::info(&format!("Current stack: {}", current.green()));
    }

    Ok(())
}

/// Create a new stack
pub async fn new_stack(name: &str) -> Result<()> {
    // Load configuration
    let config = ConfigLoader::current_dir()
        .context("Failed to get current directory")?
        .load()
        .context("Failed to load Devmer.toml. Run 'devmer init' to create one.")?;

    output::info(&format!("Creating stack: {}", name));

    // Check if stack already exists in config
    let existing_stacks = config.stack_names();
    if existing_stacks.contains(&name) {
        output::warning(&format!("Stack '{}' already exists in Devmer.toml", name));
    } else {
        // Add stack to Devmer.toml
        let config_path = Path::new("Devmer.toml");
        if config_path.exists() {
            let mut content = fs::read_to_string(config_path)?;

            // Append new stack section
            let stack_section = format!(
                "\n[stack.{}]\ndescription = \"{} environment\"\n",
                name,
                name.chars()
                    .next()
                    .map(|c| c.to_uppercase().collect::<String>())
                    .unwrap_or_default()
                    + &name[1..]
            );
            content.push_str(&stack_section);
            fs::write(config_path, content)?;
            output::info("Added stack to Devmer.toml");
        }
    }

    // Initialize empty state
    let container = AppContainer::new(config);
    let state_service = container.state_service();

    // Create empty state file
    let empty_state = devmer_core::state::StackState::new(name);
    state_service
        .save_state(name, &empty_state)
        .await
        .context("Failed to initialize stack state")?;

    // Update workspace to select new stack
    let mut workspace = Workspace::load();
    workspace.current_stack = Some(name.to_string());
    workspace.updated_at = Some(Utc::now().to_rfc3339());
    workspace.save()?;

    output::success(&format!("Created and selected stack '{}'", name));

    Ok(())
}

/// Select a stack
pub async fn select(name: &str) -> Result<()> {
    // Load configuration
    let config = ConfigLoader::current_dir()
        .context("Failed to get current directory")?
        .load()
        .context("Failed to load Devmer.toml. Run 'devmer init' to create one.")?;

    // Check if stack exists
    let existing_stacks = config.stack_names();
    if !existing_stacks.contains(&name) {
        output::warning(&format!(
            "Stack '{}' not found in Devmer.toml. It may not have configuration.",
            name
        ));
    }

    // Update workspace
    let mut workspace = Workspace::load();
    workspace.current_stack = Some(name.to_string());
    workspace.updated_at = Some(Utc::now().to_rfc3339());
    workspace.save()?;

    output::success(&format!("Selected stack '{}'", name));

    Ok(())
}

/// Remove a stack
pub async fn remove(name: &str, force: bool) -> Result<()> {
    // Load configuration
    let config = ConfigLoader::current_dir()
        .context("Failed to get current directory")?
        .load()
        .context("Failed to load Devmer.toml. Run 'devmer init' to create one.")?;

    // Create DI container
    let container = AppContainer::new(config.clone());
    let state_service = container.state_service();

    // Check if stack has resources
    let state = state_service.get_state(name).await.ok().flatten();
    if let Some(ref s) = state {
        let count = s.resource_count();
        if count > 0 && !force {
            output::error(&format!(
                "Stack '{}' still has {} resource(s). Use --force to remove anyway, or run 'devmer down' first.",
                name, count
            ));
            return Ok(());
        }
    }

    // Confirm removal
    if !force {
        let proceed = Confirm::new()
            .with_prompt(format!("Are you sure you want to remove stack '{}'?", name))
            .default(false)
            .interact()?;

        if !proceed {
            output::info("Cancelled.");
            return Ok(());
        }
    }

    output::info(&format!("Removing stack: {}", name));

    // Delete state
    state_service
        .delete_state(name)
        .await
        .context("Failed to delete stack state")?;

    // Update workspace if this was the current stack
    let mut workspace = Workspace::load();
    if workspace.current_stack.as_deref() == Some(name) {
        workspace.current_stack = config.stack_names().first().map(|s| s.to_string());
        workspace.updated_at = Some(Utc::now().to_rfc3339());
        workspace.save()?;

        if let Some(new_current) = &workspace.current_stack {
            output::info(&format!("Switched to stack '{}'", new_current));
        }
    }

    output::success(&format!("Removed stack '{}'", name));

    Ok(())
}

/// Show stack history
pub async fn history(stack: Option<String>) -> Result<()> {
    // Load configuration
    let config = ConfigLoader::current_dir()
        .context("Failed to get current directory")?
        .load()
        .context("Failed to load Devmer.toml. Run 'devmer init' to create one.")?;

    let workspace = Workspace::load();
    let stack_name = stack.or(workspace.current_stack).unwrap_or_else(|| {
        config
            .stack_names()
            .first()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "dev".to_string())
    });

    output::banner(&format!("History for stack: {}", stack_name));

    // Create DI container
    let container = AppContainer::new(config);
    let state_service = container.state_service();

    // Get state
    let state = state_service.get_state(&stack_name).await.ok().flatten();

    if let Some(s) = state {
        if s.history.is_empty() {
            output::info("No deployment history yet.");
        } else {
            println!(
                "{:<24}  {:<10}  {}",
                "Time".bold(),
                "Operation".bold(),
                "Resources".bold()
            );
            println!("{}", "─".repeat(60));

            for entry in s.history.iter().rev().take(10) {
                let time = entry.started_at.format("%Y-%m-%d %H:%M:%S");
                let op = format!("{:?}", entry.kind);
                let resources = entry.resources_created + entry.resources_updated + entry.resources_deleted;
                println!("  {:<22}  {:<10}  {}", time, op, resources);
            }

            if s.history.len() > 10 {
                output::info(&format!("... and {} more entries", s.history.len() - 10));
            }
        }
    } else {
        output::info("No state found for this stack.");
    }

    Ok(())
}

/// Show stack outputs
pub async fn output(stack: Option<String>, key: Option<String>) -> Result<()> {
    // Load configuration
    let config = ConfigLoader::current_dir()
        .context("Failed to get current directory")?
        .load()
        .context("Failed to load Devmer.toml. Run 'devmer init' to create one.")?;

    let workspace = Workspace::load();
    let stack_name = stack.or(workspace.current_stack).unwrap_or_else(|| {
        config
            .stack_names()
            .first()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "dev".to_string())
    });

    // Create DI container
    let container = AppContainer::new(config);
    let state_service = container.state_service();

    // Get state
    let state = state_service.get_state(&stack_name).await.ok().flatten();

    if let Some(_s) = state {
        if let Some(k) = key {
            // Get specific output - outputs not implemented in state yet
            output::info(&format!("Output '{}' for stack '{}':", k, stack_name));
            output::warning("Outputs are stored in the deployment result, not in state.");
        } else {
            // List all outputs
            output::banner(&format!("Outputs for stack: {}", stack_name));
            output::info("No outputs in state. Outputs are displayed after 'devmer up'.");
        }
    } else {
        output::info("No state found for this stack. Run 'devmer up' first.");
    }

    Ok(())
}

/// Get current stack name
pub fn get_current_stack(config: &devmer_config::DevmerConfig) -> String {
    let workspace = Workspace::load();
    workspace.current_stack.unwrap_or_else(|| {
        config
            .stack_names()
            .first()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "dev".to_string())
    })
}
