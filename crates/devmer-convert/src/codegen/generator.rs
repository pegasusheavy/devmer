//! Code generator implementation

use crate::codegen::{go, python, rhai, typescript};
use crate::error::Result;
use crate::ir::IrModule;
use crate::ConvertOptions;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Target language for code generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    /// TypeScript
    TypeScript,
    /// Python
    Python,
    /// Go
    Go,
    /// Rhai script
    Rhai,
}

impl Language {
    /// Get file extension for this language
    pub fn extension(&self) -> &'static str {
        match self {
            Language::TypeScript => "ts",
            Language::Python => "py",
            Language::Go => "go",
            Language::Rhai => "rhai",
        }
    }

    /// Get main file name
    pub fn main_file(&self) -> &'static str {
        match self {
            Language::TypeScript => "index.ts",
            Language::Python => "__main__.py",
            Language::Go => "main.go",
            Language::Rhai => "main.rhai",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "typescript" | "ts" => Some(Language::TypeScript),
            "python" | "py" => Some(Language::Python),
            "go" | "golang" => Some(Language::Go),
            "rhai" => Some(Language::Rhai),
            _ => None,
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Language::TypeScript => write!(f, "typescript"),
            Language::Python => write!(f, "python"),
            Language::Go => write!(f, "go"),
            Language::Rhai => write!(f, "rhai"),
        }
    }
}

/// A generated file
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    /// File path (relative to output directory)
    pub path: PathBuf,

    /// File content
    pub content: String,

    /// Whether this is the main entry point
    pub is_main: bool,
}

/// Code generator
pub struct CodeGenerator {
    /// Target language
    language: Language,

    /// Options
    options: ConvertOptions,
}

impl CodeGenerator {
    /// Create a new code generator
    pub fn new(language: Language, options: ConvertOptions) -> Self {
        Self { language, options }
    }

    /// Generate code from IR module
    pub fn generate(&self, module: &IrModule) -> Result<Vec<GeneratedFile>> {
        match self.language {
            Language::TypeScript => typescript::generate(module, &self.options),
            Language::Python => python::generate(module, &self.options),
            Language::Go => go::generate(module, &self.options),
            Language::Rhai => rhai::generate(module, &self.options),
        }
    }

    /// Generate Devmer.toml configuration
    pub fn generate_config(&self, module: &IrModule, project_name: &str) -> Result<GeneratedFile> {
        let mut config = format!(
            r#"# Devmer Configuration
# Converted from Terraform/OpenTofu

name = "{}"
description = "Infrastructure managed by Devmer"

[runtime]
name = "{}"
"#,
            project_name, self.language
        );

        if self.language == Language::TypeScript {
            config.push_str(&format!(
                "js_runtime = \"{}\"\n",
                self.options
                    .provider_mappings
                    .get("js_runtime")
                    .map(|s| s.as_str())
                    .unwrap_or("node")
            ));
        }

        config.push_str(&format!("main = \"{}\"\n", self.language.main_file()));

        // Backend configuration
        if let Some(ref settings) = module.terraform_settings {
            if let Some(ref backend) = settings.backend {
                config.push_str("\n[backend]\n");
                config.push_str(&format!("type = \"{}\"\n", map_backend_type(&backend.backend_type)));
            }
        }

        // Secrets configuration
        config.push_str(
            r#"
[secrets]
provider = "passphrase"

[stack.dev]
description = "Development environment"

[stack.prod]
description = "Production environment"
"#,
        );

        Ok(GeneratedFile {
            path: PathBuf::from("Devmer.toml"),
            content: config,
            is_main: false,
        })
    }
}

/// Map Terraform backend type to Devmer backend type
fn map_backend_type(tf_type: &str) -> &str {
    match tf_type {
        "s3" => "s3",
        "gcs" => "gcs",
        "azurerm" => "azure",
        "pg" | "postgres" => "postgresql",
        "consul" => "consul",
        "local" => "local",
        _ => "local",
    }
}
