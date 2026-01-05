//! `devmer init` command

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::output;

/// Execute the init command
pub async fn execute(name: Option<String>, runtime: Option<String>) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let project_name = name.unwrap_or_else(|| {
        cwd.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("my-project")
            .to_string()
    });

    output::info(&format!("Initializing Devmer project: {}", project_name));

    // Check if already initialized
    if Path::new("Devmer.toml").exists() {
        anyhow::bail!("Devmer.toml already exists in this directory");
    }

    // Detect runtime from existing files
    let detected_runtime = runtime.unwrap_or_else(|| {
        if Path::new("package.json").exists() {
            "typescript".to_string()
        } else if Path::new("requirements.txt").exists() || Path::new("pyproject.toml").exists() {
            "python".to_string()
        } else if Path::new("go.mod").exists() {
            "go".to_string()
        } else {
            "typescript".to_string()
        }
    });

    output::info(&format!("Detected runtime: {}", detected_runtime));

    // Create Devmer.toml
    let config_content = generate_config(&project_name, &detected_runtime);
    fs::write("Devmer.toml", config_content).context("Failed to create Devmer.toml")?;

    // Update .gitignore if it exists
    if Path::new(".gitignore").exists() {
        let gitignore = fs::read_to_string(".gitignore")?;
        if !gitignore.contains(".devmer/") {
            let additions = "\n# Devmer\n.devmer/\n*.local.toml\n";
            fs::write(".gitignore", format!("{}{}", gitignore, additions))?;
            output::info("Updated .gitignore");
        }
    }

    output::success("Initialized Devmer project");
    output::info("Next steps:");
    println!("  devmer stack new dev");
    println!("  devmer preview");

    Ok(())
}

fn generate_config(name: &str, runtime: &str) -> String {
    let main_file = match runtime {
        "typescript" => "index.ts",
        "python" => "__main__.py",
        "go" => "main.go",
        "rhai" => "main.rhai",
        _ => "index.ts",
    };

    format!(
        r#"# Devmer Configuration
name = "{}"
description = "Infrastructure managed by Devmer"

[runtime]
name = "{}"
main = "{}"

[backend]
type = "local"

[secrets]
provider = "passphrase"

[stack.dev]
description = "Development environment"
"#,
        name, runtime, main_file
    )
}
