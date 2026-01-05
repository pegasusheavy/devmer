//! `devmer convert` command - Convert HCL projects to scripting languages

use anyhow::{Context, Result};
use devmer_convert::{convert_project, ConvertOptions, Language};
use std::fs;
use std::path::Path;

use crate::output;

/// Execute the convert command
pub async fn execute(
    source: &str,
    language: &str,
    output_dir: Option<String>,
    project_name: Option<String>,
    generate_config: bool,
) -> Result<()> {
    let source_path = Path::new(source);

    if !source_path.exists() {
        anyhow::bail!("Source directory not found: {}", source);
    }

    let target_lang = Language::from_str(language)
        .ok_or_else(|| anyhow::anyhow!(
            "Unknown language: {}. Supported: typescript, python, go, rhai",
            language
        ))?;

    output::banner(&format!("Converting HCL to {}", target_lang));
    output::info(&format!("Source: {}", source_path.display()));

    let options = ConvertOptions {
        output_dir: output_dir.map(|s| s.into()),
        project_name: project_name.clone(),
        generate_config,
        preserve_comments: true,
        generate_tests: false,
        provider_mappings: std::collections::HashMap::new(),
        use_async: true,
        format_output: true,
        generate_types: true,
        js_runtime: None,
        verbose: false,
    };

    // Convert the project
    let converted = convert_project(source_path, target_lang, options.clone())
        .await
        .context("Failed to convert HCL project")?;

    // Determine output directory
    let out_dir = options
        .output_dir
        .clone()
        .unwrap_or_else(|| {
            let name = project_name
                .clone()
                .or_else(|| {
                    source_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| format!("{}-devmer", s))
                })
                .unwrap_or_else(|| "devmer-project".to_string());
            Path::new(".").join(&name)
        });

    output::info(&format!("Output: {}", out_dir.display()));

    // Create output directory
    fs::create_dir_all(&out_dir).context("Failed to create output directory")?;

    // Write generated files
    let mut file_count = 0;
    for file in &converted.generated_files {
        let file_path = out_dir.join(&file.path);

        // Create parent directories
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&file_path, &file.content)
            .with_context(|| format!("Failed to write {}", file_path.display()))?;

        output::info(&format!("  Created: {}", file.path.display()));
        file_count += 1;
    }

    // Generate Devmer.toml if requested
    if generate_config {
        let generator = devmer_convert::CodeGenerator::new(target_lang, options);
        let config_file = generator.generate_config(
            &converted.module,
            project_name.as_deref().unwrap_or("devmer-project"),
        )?;

        let config_path = out_dir.join(&config_file.path);
        fs::write(&config_path, &config_file.content)?;
        output::info(&format!("  Created: {}", config_file.path.display()));
        file_count += 1;
    }

    // Summary
    println!();
    output::success(&format!(
        "Converted {} resources, {} data sources, {} outputs",
        converted.module.resources.len(),
        converted.module.data_sources.len(),
        converted.module.outputs.len()
    ));
    output::success(&format!("Generated {} files", file_count));

    // Next steps
    println!();
    output::info("Next steps:");
    println!("  cd {}", out_dir.display());

    match target_lang {
        Language::TypeScript => {
            println!("  npm install");
            println!("  devmer preview");
        }
        Language::Python => {
            println!("  pip install -r requirements.txt");
            println!("  devmer preview");
        }
        Language::Go => {
            println!("  go mod tidy");
            println!("  devmer preview");
        }
        Language::Rhai => {
            println!("  devmer preview");
        }
    }

    Ok(())
}

/// List supported source formats and target languages
pub fn list_formats() {
    output::banner("Supported Conversions");

    println!("Source formats:");
    println!("  - Terraform/OpenTofu HCL (.tf files)");
    println!("  - Terraform JSON (.tf.json files)");
    println!();

    println!("Target languages:");
    println!("  - typescript (ts)  - TypeScript/JavaScript");
    println!("  - python (py)      - Python 3.10+");
    println!("  - go               - Go 1.21+");
    println!("  - rhai             - Rhai embedded script");
}

/// Analyze an HCL project without converting
pub async fn analyze(source: &str) -> Result<()> {
    let source_path = Path::new(source);

    if !source_path.exists() {
        anyhow::bail!("Source directory not found: {}", source);
    }

    output::banner("Analyzing HCL Project");
    output::info(&format!("Source: {}", source_path.display()));

    let parser = devmer_convert::HclParser::new();
    let module = parser
        .parse_directory(source_path)
        .context("Failed to parse HCL files")?;

    println!();
    println!("Project summary:");
    println!("  Files parsed: {}", module.source_files.len());
    println!("  Providers:    {}", module.required_providers.len());
    println!("  Variables:    {}", module.variables.len());
    println!("  Locals:       {}", module.locals.len());
    println!("  Resources:    {}", module.resources.len());
    println!("  Data sources: {}", module.data_sources.len());
    println!("  Outputs:      {}", module.outputs.len());
    println!("  Modules:      {}", module.modules.len());

    // List providers
    if !module.required_providers.is_empty() {
        println!();
        println!("Required providers:");
        for (name, req) in &module.required_providers {
            let version = req.version.as_deref().unwrap_or("*");
            let source = req.source.as_deref().unwrap_or("unknown");
            println!("  {} ({}) - {}", name, version, source);
        }
    }

    // List resources by type
    if !module.resources.is_empty() {
        println!();
        println!("Resources by type:");

        let mut by_type: std::collections::HashMap<&str, Vec<&str>> = std::collections::HashMap::new();
        for resource in &module.resources {
            by_type
                .entry(&resource.resource_type)
                .or_default()
                .push(&resource.name);
        }

        let mut types: Vec<_> = by_type.keys().collect();
        types.sort();

        for resource_type in types {
            let names = &by_type[resource_type];
            println!("  {} ({})", resource_type, names.len());
            for name in names.iter().take(3) {
                println!("    - {}", name);
            }
            if names.len() > 3 {
                println!("    ... and {} more", names.len() - 3);
            }
        }
    }

    // Warnings about unsupported features
    if !module.modules.is_empty() {
        println!();
        output::warning(&format!(
            "{} module calls found - these require manual conversion",
            module.modules.len()
        ));
    }

    Ok(())
}
