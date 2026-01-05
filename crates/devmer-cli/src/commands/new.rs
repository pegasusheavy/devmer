//! `devmer new` command

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::output;

/// Execute the new command
pub async fn execute(
    name: &str,
    template: &str,
    runtime: &str,
    js_runtime: &str,
    generate_sample: bool,
) -> Result<()> {
    output::info(&format!("Creating new Devmer project: {}", name));

    // Create project directory
    let project_dir = Path::new(name);
    if project_dir.exists() {
        anyhow::bail!("Directory '{}' already exists", name);
    }

    fs::create_dir_all(project_dir).context("Failed to create project directory")?;

    // Create Devmer.toml
    let config_content = generate_config(name, runtime, js_runtime);
    fs::write(project_dir.join("Devmer.toml"), config_content)
        .context("Failed to create Devmer.toml")?;

    // Create .gitignore
    let gitignore = r#"# Devmer
.devmer/
*.local.toml

# Environment
.env
.env.local
.env.*.local

# IDE
.idea/
.vscode/
*.swp
*.swo

# OS
.DS_Store
Thumbs.db
"#;
    fs::write(project_dir.join(".gitignore"), gitignore).context("Failed to create .gitignore")?;

    // Create runtime-specific files
    match runtime {
        "typescript" => create_typescript_project(project_dir, js_runtime, generate_sample)?,
        "python" => create_python_project(project_dir, generate_sample)?,
        "go" => create_go_project(project_dir, name, generate_sample)?,
        "rhai" => create_rhai_project(project_dir, generate_sample)?,
        _ => anyhow::bail!("Unknown runtime: {}", runtime),
    }

    // Create template-specific resources
    if template != "default" && generate_sample {
        create_template_resources(project_dir, template, runtime)?;
    }

    output::success(&format!("Created project '{}'", name));
    output::info("Next steps:");
    println!("  cd {}", name);
    println!("  devmer stack new dev");
    println!("  devmer preview");

    Ok(())
}

fn generate_config(name: &str, runtime: &str, js_runtime: &str) -> String {
    let mut config = format!(
        r#"# Devmer Configuration
name = "{}"
description = "Infrastructure managed by Devmer"

[runtime]
name = "{}"
"#,
        name, runtime
    );

    if runtime == "typescript" {
        config.push_str(&format!("js_runtime = \"{}\"\n", js_runtime));
        config.push_str("main = \"index.ts\"\n");
    } else if runtime == "python" {
        config.push_str("main = \"__main__.py\"\n");
    } else if runtime == "go" {
        config.push_str("main = \"main.go\"\n");
    } else if runtime == "rhai" {
        config.push_str("main = \"main.rhai\"\n");
    }

    config.push_str(
        r#"
[backend]
type = "local"
# For S3 backend:
# type = "s3"
# bucket = "${DEVMER_STATE_BUCKET}"
# region = "${AWS_REGION:-us-east-1}"
# lock_table = "devmer-locks"

[secrets]
provider = "passphrase"
# For AWS KMS:
# provider = "awskms"
# kms_key_id = "alias/devmer"

[stack.dev]
description = "Development environment"

[stack.prod]
description = "Production environment"
"#,
    );

    config
}

fn create_typescript_project(project_dir: &Path, js_runtime: &str, sample: bool) -> Result<()> {
    // package.json
    let package_json = format!(
        r#"{{
  "name": "{}",
  "version": "0.1.0",
  "type": "module",
  "scripts": {{
    "build": "tsc",
    "preview": "devmer preview",
    "deploy": "devmer up"
  }},
  "devDependencies": {{
    "typescript": "^5.0.0",
    "@types/node": "^20.0.0"
  }},
  "dependencies": {{
    "@devmer/sdk": "^0.1.0"
  }}
}}
"#,
        project_dir.file_name().unwrap().to_str().unwrap()
    );
    fs::write(project_dir.join("package.json"), package_json)?;

    // tsconfig.json
    let tsconfig = r#"{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "outDir": "./dist"
  },
  "include": ["*.ts"],
  "exclude": ["node_modules"]
}
"#;
    fs::write(project_dir.join("tsconfig.json"), tsconfig)?;

    // index.ts
    let index_ts = if sample {
        r#"import * as devmer from "@devmer/sdk";

// Example: Create an S3 bucket
const bucket = new devmer.aws.s3.Bucket("my-bucket", {
    bucket: "my-unique-bucket-name",
    tags: {
        Environment: devmer.getStack(),
        ManagedBy: "Devmer",
    },
});

// Export the bucket name
export const bucketName = bucket.bucket;
export const bucketArn = bucket.arn;
"#
    } else {
        r#"import * as devmer from "@devmer/sdk";

// Define your infrastructure here
"#
    };
    fs::write(project_dir.join("index.ts"), index_ts)?;

    // Runtime-specific config
    if js_runtime == "deno" {
        let deno_json = r#"{
  "compilerOptions": {
    "strict": true
  },
  "imports": {
    "@devmer/sdk": "jsr:@devmer/sdk@^0.1.0"
  }
}
"#;
        fs::write(project_dir.join("deno.json"), deno_json)?;
    } else if js_runtime == "bun" {
        // Bun uses package.json, no extra config needed
    }

    Ok(())
}

fn create_python_project(project_dir: &Path, sample: bool) -> Result<()> {
    // requirements.txt
    let requirements = "devmer>=0.1.0\n";
    fs::write(project_dir.join("requirements.txt"), requirements)?;

    // pyproject.toml
    let pyproject = r#"[project]
name = "devmer-project"
version = "0.1.0"
requires-python = ">=3.10"
dependencies = [
    "devmer>=0.1.0",
]

[tool.ruff]
line-length = 100
"#;
    fs::write(project_dir.join("pyproject.toml"), pyproject)?;

    // __main__.py
    let main_py = if sample {
        r#"import devmer
from devmer import aws

# Example: Create an S3 bucket
bucket = aws.s3.Bucket("my-bucket",
    bucket="my-unique-bucket-name",
    tags={
        "Environment": devmer.get_stack(),
        "ManagedBy": "Devmer",
    },
)

# Export the bucket name
devmer.export("bucket_name", bucket.bucket)
devmer.export("bucket_arn", bucket.arn)
"#
    } else {
        r#"import devmer

# Define your infrastructure here
"#
    };
    fs::write(project_dir.join("__main__.py"), main_py)?;

    Ok(())
}

fn create_go_project(project_dir: &Path, name: &str, sample: bool) -> Result<()> {
    // go.mod
    let go_mod = format!(
        r#"module {}

go 1.21

require github.com/devmer/sdk-go v0.1.0
"#,
        name
    );
    fs::write(project_dir.join("go.mod"), go_mod)?;

    // main.go
    let main_go = if sample {
        r#"package main

import (
    "github.com/devmer/sdk-go/devmer"
    "github.com/devmer/sdk-go/devmer/aws/s3"
)

func main() {
    devmer.Run(func(ctx *devmer.Context) error {
        // Example: Create an S3 bucket
        bucket, err := s3.NewBucket(ctx, "my-bucket", &s3.BucketArgs{
            Bucket: devmer.String("my-unique-bucket-name"),
            Tags: devmer.StringMap{
                "Environment": devmer.String(ctx.Stack()),
                "ManagedBy":   devmer.String("Devmer"),
            },
        })
        if err != nil {
            return err
        }

        // Export the bucket name
        ctx.Export("bucketName", bucket.Bucket)
        ctx.Export("bucketArn", bucket.Arn)

        return nil
    })
}
"#
    } else {
        r#"package main

import (
    "github.com/devmer/sdk-go/devmer"
)

func main() {
    devmer.Run(func(ctx *devmer.Context) error {
        // Define your infrastructure here
        return nil
    })
}
"#
    };
    fs::write(project_dir.join("main.go"), main_go)?;

    Ok(())
}

fn create_rhai_project(project_dir: &Path, sample: bool) -> Result<()> {
    // main.rhai
    let main_rhai = if sample {
        r#"// Devmer Rhai Script

// Example: Create an S3 bucket
let bucket = aws::s3::Bucket("my-bucket", #{
    bucket: "my-unique-bucket-name",
    tags: #{
        Environment: stack(),
        ManagedBy: "Devmer",
    },
});

// Export the bucket name
export("bucket_name", bucket.bucket);
export("bucket_arn", bucket.arn);
"#
    } else {
        r#"// Devmer Rhai Script
// Define your infrastructure here
"#
    };
    fs::write(project_dir.join("main.rhai"), main_rhai)?;

    Ok(())
}

fn create_template_resources(_project_dir: &Path, _template: &str, _runtime: &str) -> Result<()> {
    // Template-specific resource files would be added here
    Ok(())
}
