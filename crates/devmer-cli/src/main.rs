//! Devmer CLI - Infrastructure as Code
//!
//! A self-hosted, Rust-based IaC tool inspired by Pulumi.

mod commands;
mod output;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Devmer - Infrastructure as Code
#[derive(Parser)]
#[command(name = "devmer")]
#[command(author = "Joseph R. Quinn")]
#[command(version)]
#[command(about = "A self-hosted Infrastructure as Code tool", long_about = None)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Output format (text, json)
    #[arg(short, long, global = true, default_value = "text")]
    format: String,

    /// Working directory
    #[arg(short = 'C', long, global = true)]
    cwd: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new Devmer project
    New {
        /// Project name
        name: String,

        /// Template to use (default, aws, gcp, azure, kubernetes)
        #[arg(short, long, default_value = "default")]
        template: String,

        /// Runtime (python, typescript, go, rhai)
        #[arg(short, long, default_value = "typescript")]
        runtime: String,

        /// JavaScript runtime for TypeScript (node, deno, bun)
        #[arg(long, default_value = "node")]
        js_runtime: String,

        /// Generate sample code
        #[arg(long)]
        generate_sample: bool,
    },

    /// Initialize Devmer in an existing directory
    Init {
        /// Project name (defaults to directory name)
        #[arg(short, long)]
        name: Option<String>,

        /// Runtime (python, typescript, go, rhai)
        #[arg(short, long)]
        runtime: Option<String>,
    },

    /// Preview changes to be made
    Preview {
        /// Stack name
        #[arg(short, long)]
        stack: Option<String>,

        /// Show detailed diff
        #[arg(long)]
        diff: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Deploy infrastructure changes
    Up {
        /// Stack name
        #[arg(short, long)]
        stack: Option<String>,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,

        /// Refresh state before deploying
        #[arg(long)]
        refresh: bool,

        /// Number of concurrent operations
        #[arg(long, default_value = "10")]
        parallel: usize,
    },

    /// Destroy deployed infrastructure
    Down {
        /// Stack name
        #[arg(short, long)]
        stack: Option<String>,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,

        /// Remove the stack completely
        #[arg(long)]
        remove: bool,
    },

    /// Refresh state from cloud providers
    Refresh {
        /// Stack name
        #[arg(short, long)]
        stack: Option<String>,
    },

    /// Stack management commands
    Stack {
        #[command(subcommand)]
        command: StackCommands,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Secrets management
    Secrets {
        #[command(subcommand)]
        command: SecretsCommands,
    },

    /// State management commands
    State {
        #[command(subcommand)]
        command: StateCommands,
    },

    /// Login to cloud providers
    Login {
        /// Provider name (aws, gcp, azure)
        provider: Option<String>,
    },

    /// Convert HCL (Terraform/OpenTofu) projects to Devmer
    Convert {
        #[command(subcommand)]
        command: ConvertCommands,
    },

    /// Show version information
    Version,
}

#[derive(Subcommand)]
enum ConvertCommands {
    /// Convert an HCL project to a scripting language
    From {
        /// Source directory containing .tf files
        source: String,

        /// Target language (typescript, python, go, rhai)
        #[arg(short, long, default_value = "typescript")]
        language: String,

        /// Output directory
        #[arg(short, long)]
        output: Option<String>,

        /// Project name
        #[arg(short, long)]
        name: Option<String>,

        /// Generate Devmer.toml configuration
        #[arg(long, default_value = "true")]
        config: bool,
    },

    /// Analyze an HCL project without converting
    Analyze {
        /// Source directory containing .tf files
        source: String,
    },

    /// List supported formats and languages
    Formats,
}

#[derive(Subcommand)]
enum StackCommands {
    /// List all stacks
    Ls,

    /// Create a new stack
    New {
        /// Stack name
        name: String,
    },

    /// Select a stack
    Select {
        /// Stack name
        name: String,
    },

    /// Remove a stack
    Rm {
        /// Stack name
        name: String,

        /// Force removal
        #[arg(short, long)]
        force: bool,
    },

    /// Show stack history
    History {
        /// Stack name
        #[arg(short, long)]
        stack: Option<String>,
    },

    /// Export stack output
    Output {
        /// Stack name
        #[arg(short, long)]
        stack: Option<String>,

        /// Output key
        key: Option<String>,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Get a configuration value
    Get {
        /// Configuration key
        key: String,

        /// Stack name
        #[arg(short, long)]
        stack: Option<String>,
    },

    /// Set a configuration value
    Set {
        /// Configuration key
        key: String,

        /// Configuration value
        value: String,

        /// Stack name
        #[arg(short, long)]
        stack: Option<String>,

        /// Mark as secret
        #[arg(long)]
        secret: bool,
    },

    /// Remove a configuration value
    Rm {
        /// Configuration key
        key: String,

        /// Stack name
        #[arg(short, long)]
        stack: Option<String>,
    },
}

#[derive(Subcommand)]
enum SecretsCommands {
    /// Set a secret
    Set {
        /// Secret name
        name: String,

        /// Secret value (reads from stdin if not provided)
        value: Option<String>,
    },

    /// Get a secret
    Get {
        /// Secret name
        name: String,
    },

    /// List secrets
    Ls,

    /// Rotate secrets encryption
    Rotate,
}

#[derive(Subcommand)]
enum StateCommands {
    /// Export state to file
    Export {
        /// Output file
        #[arg(short, long)]
        file: Option<String>,

        /// Stack name
        #[arg(short, long)]
        stack: Option<String>,
    },

    /// Import state from file
    Import {
        /// Input file
        file: String,

        /// Stack name
        #[arg(short, long)]
        stack: Option<String>,
    },

    /// Unlock state
    Unlock {
        /// Stack name
        #[arg(short, long)]
        stack: Option<String>,
    },

    /// Delete resource from state
    Delete {
        /// Resource URN
        urn: String,

        /// Stack name
        #[arg(short, long)]
        stack: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(false))
        .with(filter)
        .init();

    // Change directory if specified
    if let Some(cwd) = &cli.cwd {
        std::env::set_current_dir(cwd)?;
    }

    // Execute command
    match cli.command {
        Commands::New {
            name,
            template,
            runtime,
            js_runtime,
            generate_sample,
        } => {
            commands::new::execute(&name, &template, &runtime, &js_runtime, generate_sample).await
        }

        Commands::Init { name, runtime } => commands::init::execute(name, runtime).await,

        Commands::Preview { stack, diff, json } => {
            commands::preview::execute(stack, diff, json).await
        }

        Commands::Up {
            stack,
            yes,
            refresh,
            parallel,
        } => commands::up::execute(stack, yes, refresh, parallel).await,

        Commands::Down { stack, yes, remove } => commands::down::execute(stack, yes, remove).await,

        Commands::Refresh { stack } => commands::refresh::execute(stack).await,

        Commands::Stack { command } => match command {
            StackCommands::Ls => commands::stack::list().await,
            StackCommands::New { name } => commands::stack::new_stack(&name).await,
            StackCommands::Select { name } => commands::stack::select(&name).await,
            StackCommands::Rm { name, force } => commands::stack::remove(&name, force).await,
            StackCommands::History { stack } => commands::stack::history(stack).await,
            StackCommands::Output { stack, key } => commands::stack::output(stack, key).await,
        },

        Commands::Config { command } => match command {
            ConfigCommands::Get { key, stack } => commands::config::get(&key, stack).await,
            ConfigCommands::Set {
                key,
                value,
                stack,
                secret,
            } => commands::config::set(&key, &value, stack, secret).await,
            ConfigCommands::Rm { key, stack } => commands::config::remove(&key, stack).await,
        },

        Commands::Secrets { command } => match command {
            SecretsCommands::Set { name, value } => commands::secrets::set(&name, value).await,
            SecretsCommands::Get { name } => commands::secrets::get(&name).await,
            SecretsCommands::Ls => commands::secrets::list().await,
            SecretsCommands::Rotate => commands::secrets::rotate().await,
        },

        Commands::State { command } => match command {
            StateCommands::Export { file, stack } => commands::state::export(file, stack).await,
            StateCommands::Import { file, stack } => commands::state::import(&file, stack).await,
            StateCommands::Unlock { stack } => commands::state::unlock(stack).await,
            StateCommands::Delete { urn, stack } => commands::state::delete(&urn, stack).await,
        },

        Commands::Login { provider } => commands::login::execute(provider).await,

        Commands::Convert { command } => match command {
            ConvertCommands::From {
                source,
                language,
                output,
                name,
                config,
            } => commands::convert::execute(&source, &language, output, name, config).await,
            ConvertCommands::Analyze { source } => commands::convert::analyze(&source).await,
            ConvertCommands::Formats => {
                commands::convert::list_formats();
                Ok(())
            }
        },

        Commands::Version => {
            println!("devmer {}", env!("CARGO_PKG_VERSION"));
            println!("rust {}", env!("CARGO_PKG_RUST_VERSION"));
            Ok(())
        }
    }
}
