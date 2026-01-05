//! Configuration management commands

use anyhow::{Context, Result};
use devmer_config::ConfigLoader;
use std::fs;
use std::path::Path;

use crate::commands::stack::get_current_stack;
use crate::output;

/// Stack-specific configuration file
fn stack_config_path(stack: &str) -> String {
    format!("Devmer.{}.toml", stack)
}

/// Configuration entry
#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
struct StackConfig {
    #[serde(default)]
    config: std::collections::HashMap<String, toml::Value>,
    #[serde(default)]
    secrets: std::collections::HashMap<String, String>,
}

impl StackConfig {
    fn load(stack: &str) -> Self {
        let path = stack_config_path(stack);
        if Path::new(&path).exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|s| toml::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    fn save(&self, stack: &str) -> Result<()> {
        let path = stack_config_path(stack);
        let content = toml::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }
}

/// Get a configuration value
pub async fn get(key: &str, stack: Option<String>) -> Result<()> {
    // Load main configuration
    let config = ConfigLoader::current_dir()
        .context("Failed to get current directory")?
        .load()
        .context("Failed to load Devmer.toml. Run 'devmer init' to create one.")?;

    let stack_name = stack.unwrap_or_else(|| get_current_stack(&config));

    // Load stack-specific config
    let stack_config = StackConfig::load(&stack_name);

    // Check stack config first
    if let Some(value) = stack_config.config.get(key) {
        println!("{}", toml_value_to_string(value));
        return Ok(());
    }

    // Check if it's a secret
    if stack_config.secrets.contains_key(key) {
        output::info(&format!("{}: [secret]", key));
        return Ok(());
    }

    // Config not found
    output::warning(&format!(
        "Configuration '{}' not found for stack '{}'",
        key, stack_name
    ));
    println!();
    output::info("Set a value with: devmer config set <key> <value>");

    Ok(())
}

/// Set a configuration value
pub async fn set(key: &str, value: &str, stack: Option<String>, secret: bool) -> Result<()> {
    // Load main configuration
    let config = ConfigLoader::current_dir()
        .context("Failed to get current directory")?
        .load()
        .context("Failed to load Devmer.toml. Run 'devmer init' to create one.")?;

    let stack_name = stack.unwrap_or_else(|| get_current_stack(&config));

    // Load stack-specific config
    let mut stack_config = StackConfig::load(&stack_name);

    if secret {
        // Store as a secret (marked but not encrypted in config file)
        stack_config.secrets.insert(key.to_string(), value.to_string());
        stack_config.save(&stack_name)?;

        output::success(&format!(
            "Set secret '{}' for stack '{}'",
            key, stack_name
        ));
        output::warning(
            "Note: Use 'devmer secrets set' for proper encryption, or configure a secrets provider.",
        );
    } else {
        // Parse value as appropriate type
        let toml_value = parse_toml_value(value);
        stack_config.config.insert(key.to_string(), toml_value);
        stack_config.save(&stack_name)?;

        output::success(&format!(
            "Set '{}' = '{}' for stack '{}'",
            key, value, stack_name
        ));
    }

    Ok(())
}

/// Remove a configuration value
pub async fn remove(key: &str, stack: Option<String>) -> Result<()> {
    // Load main configuration
    let config = ConfigLoader::current_dir()
        .context("Failed to get current directory")?
        .load()
        .context("Failed to load Devmer.toml. Run 'devmer init' to create one.")?;

    let stack_name = stack.unwrap_or_else(|| get_current_stack(&config));

    // Load stack-specific config
    let mut stack_config = StackConfig::load(&stack_name);

    let removed_config = stack_config.config.remove(key).is_some();
    let removed_secret = stack_config.secrets.remove(key).is_some();

    if removed_config || removed_secret {
        stack_config.save(&stack_name)?;
        output::success(&format!("Removed '{}' from stack '{}'", key, stack_name));
    } else {
        output::warning(&format!(
            "Configuration '{}' not found in stack '{}'",
            key, stack_name
        ));
    }

    Ok(())
}

/// Parse a string value to appropriate TOML type
fn parse_toml_value(value: &str) -> toml::Value {
    // Try to parse as number
    if let Ok(n) = value.parse::<i64>() {
        return toml::Value::Integer(n);
    }
    if let Ok(n) = value.parse::<f64>() {
        return toml::Value::Float(n);
    }

    // Try to parse as boolean
    match value.to_lowercase().as_str() {
        "true" => return toml::Value::Boolean(true),
        "false" => return toml::Value::Boolean(false),
        _ => {}
    }

    // Try to parse as array
    if value.starts_with('[') && value.ends_with(']') {
        if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(value) {
            let toml_arr: Vec<toml::Value> = arr.into_iter().map(json_to_toml).collect();
            return toml::Value::Array(toml_arr);
        }
    }

    // Try to parse as JSON object -> TOML table
    if value.starts_with('{') && value.ends_with('}') {
        if let Ok(obj) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(value) {
            let mut table = toml::map::Map::new();
            for (k, v) in obj {
                table.insert(k, json_to_toml(v));
            }
            return toml::Value::Table(table);
        }
    }

    // Default to string
    toml::Value::String(value.to_string())
}

/// Convert JSON value to TOML value
fn json_to_toml(value: serde_json::Value) -> toml::Value {
    match value {
        serde_json::Value::Null => toml::Value::String("null".to_string()),
        serde_json::Value::Bool(b) => toml::Value::Boolean(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                toml::Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                toml::Value::Float(f)
            } else {
                toml::Value::String(n.to_string())
            }
        }
        serde_json::Value::String(s) => toml::Value::String(s),
        serde_json::Value::Array(arr) => {
            toml::Value::Array(arr.into_iter().map(json_to_toml).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut table = toml::map::Map::new();
            for (k, v) in obj {
                table.insert(k, json_to_toml(v));
            }
            toml::Value::Table(table)
        }
    }
}

/// Convert TOML value to display string
fn toml_value_to_string(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => s.clone(),
        toml::Value::Integer(n) => n.to_string(),
        toml::Value::Float(n) => n.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(toml_value_to_string).collect();
            format!("[{}]", items.join(", "))
        }
        toml::Value::Table(t) => {
            let items: Vec<String> = t
                .iter()
                .map(|(k, v)| format!("{}: {}", k, toml_value_to_string(v)))
                .collect();
            format!("{{{}}}", items.join(", "))
        }
        toml::Value::Datetime(dt) => dt.to_string(),
    }
}
