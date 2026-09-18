<!-- converted from Cursor rules -->

## Cursor rule: `.cursor/rules/cli.mdc`

_CLI design patterns and conventions_

Applies to: `["**/devmer-cli/**/*.rs", "**/cli/**/*.rs"]`

# CLI Design Guidelines

## Command Structure

Use `clap` with derive macros:

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "devmer")]
#[command(about = "Infrastructure as Code for the modern cloud")]
#[command(version, author)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,
    
    /// Configuration file path
    #[arg(short, long, global = true, default_value = "Devmer.toml")]
    pub config: PathBuf,
    
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create or update infrastructure
    Up {
        /// Stack to deploy
        #[arg(short, long)]
        stack: Option<String>,
        
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
    
    /// Preview changes without deploying
    Preview {
        #[arg(short, long)]
        stack: Option<String>,
    },
    
    /// Destroy infrastructure
    Down {
        #[arg(short, long)]
        stack: Option<String>,
        
        #[arg(short = 'y', long)]
        yes: bool,
    },
    
    /// Stack management
    Stack {
        #[command(subcommand)]
        command: StackCommands,
    },
}
```

## Output Formatting

Support multiple output formats:

```rust
#[derive(Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
    Yaml,
}

pub fn print_resources(resources: &[Resource], format: OutputFormat) {
    match format {
        OutputFormat::Human => print_resources_table(resources),
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(resources).unwrap()),
        OutputFormat::Yaml => println!("{}", serde_yaml::to_string(resources).unwrap()),
    }
}
```

## Colored Output

Use `colored` crate for terminal colors:

```rust
use colored::*;

pub fn print_diff(diff: &ResourceDiff) {
    match diff.change_type {
        ChangeType::Create => {
            println!("{} {}", "+".green().bold(), diff.resource_name.green());
        }
        ChangeType::Update => {
            println!("{} {}", "~".yellow().bold(), diff.resource_name.yellow());
        }
        ChangeType::Delete => {
            println!("{} {}", "-".red().bold(), diff.resource_name.red());
        }
        ChangeType::NoOp => {
            println!("{} {}", " ".normal(), diff.resource_name.dimmed());
        }
    }
}
```

## Progress Indicators

Use `indicatif` for progress bars:

```rust
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};

pub async fn deploy_with_progress(resources: &[Resource]) -> Result<()> {
    let multi = MultiProgress::new();
    let style = ProgressStyle::default_bar()
        .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
        .unwrap();
    
    let pb = multi.add(ProgressBar::new(resources.len() as u64));
    pb.set_style(style);
    
    for resource in resources {
        pb.set_message(format!("Creating {}", resource.name));
        create_resource(resource).await?;
        pb.inc(1);
    }
    
    pb.finish_with_message("Deployment complete");
    Ok(())
}
```

## Interactive Prompts

Use `dialoguer` for user input:

```rust
use dialoguer::{Confirm, Select, Input};

pub fn confirm_deployment(changes: &[Change]) -> Result<bool> {
    println!("\nPlanned changes:");
    for change in changes {
        print_change(change);
    }
    
    Confirm::new()
        .with_prompt("Do you want to apply these changes?")
        .default(false)
        .interact()
        .map_err(Into::into)
}

pub fn select_stack(stacks: &[String]) -> Result<String> {
    let selection = Select::new()
        .with_prompt("Select a stack")
        .items(stacks)
        .interact()?;
    
    Ok(stacks[selection].clone())
}
```

## Error Display

Format errors nicely for users:

```rust
pub fn display_error(error: &Error) {
    eprintln!("{}: {}", "Error".red().bold(), error);
    
    if let Some(cause) = error.source() {
        eprintln!("\n{}: {}", "Caused by".yellow(), cause);
    }
    
    if let Some(hint) = error.hint() {
        eprintln!("\n{}: {}", "Hint".cyan(), hint);
    }
}
```

## Exit Codes

Use consistent exit codes:

```rust
pub enum ExitCode {
    Success = 0,
    GeneralError = 1,
    ConfigError = 2,
    StateError = 3,
    ProviderError = 4,
    UserAborted = 130,
}

pub fn main() {
    let result = run();
    
    std::process::exit(match result {
        Ok(_) => ExitCode::Success as i32,
        Err(e) => {
            display_error(&e);
            e.exit_code() as i32
        }
    });
}
```


## Cursor rule: `.cursor/rules/configuration.mdc`

_Configuration parsing and environment variable handling_

Applies to: `["**/devmer-config/**/*.rs", "**/config/**/*.rs", "**/*.toml"]`

# Configuration Guidelines

## Configuration Structure

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct DevmerConfig {
    pub project: ProjectConfig,
    
    #[serde(default)]
    pub backend: BackendConfig,
    
    #[serde(default)]
    pub secrets: SecretsConfig,
    
    #[serde(default)]
    pub stack: HashMap<String, StackConfig>,
    
    #[serde(default)]
    pub plugins: HashMap<String, String>,
    
    #[serde(default)]
    pub environment: EnvironmentConfig,
}
```

## Environment Variable Interpolation

Support multiple interpolation patterns:

```rust
pub fn interpolate(value: &str, env: &Environment) -> Result<String> {
    let re = Regex::new(r"\$\{([^}]+)\}").unwrap();
    
    let result = re.replace_all(value, |caps: &Captures| {
        let expr = &caps[1];
        
        // ${VAR} - required
        // ${VAR:-default} - with default
        // ${VAR:?error} - required with custom error
        // ${env:VAR} - explicit env prefix
        // ${file:/path} - read from file
        // ${secret:name} - from secrets provider
        
        parse_interpolation(expr, env)
            .unwrap_or_else(|e| format!("ERROR: {}", e))
    });
    
    Ok(result.into_owned())
}

fn parse_interpolation(expr: &str, env: &Environment) -> Result<String> {
    if let Some(rest) = expr.strip_prefix("env:") {
        return env.get(rest).ok_or_else(|| ConfigError::MissingEnv(rest.into()));
    }
    
    if let Some(path) = expr.strip_prefix("file:") {
        return std::fs::read_to_string(path)
            .map(|s| s.trim().to_string())
            .map_err(|e| ConfigError::FileRead(path.into(), e));
    }
    
    if let Some(name) = expr.strip_prefix("secret:") {
        return env.get_secret(name);
    }
    
    // Handle ${VAR:-default} and ${VAR:?error}
    if let Some((var, default)) = expr.split_once(":-") {
        return Ok(env.get(var).unwrap_or_else(|| default.to_string()));
    }
    
    if let Some((var, error)) = expr.split_once(":?") {
        return env.get(var).ok_or_else(|| ConfigError::Custom(error.into()));
    }
    
    // Simple ${VAR}
    env.get(expr).ok_or_else(|| ConfigError::MissingEnv(expr.into()))
}
```

## .env File Loading

```rust
pub fn load_env_files(stack: &str) -> Result<HashMap<String, String>> {
    let mut env = HashMap::new();
    
    // Load in order (later overrides earlier)
    let files = [
        ".env",
        ".env.local",
        &format!(".env.{}", stack),
        &format!(".env.{}.local", stack),
    ];
    
    for file in files {
        if Path::new(file).exists() {
            let contents = std::fs::read_to_string(file)?;
            for line in contents.lines() {
                if let Some((key, value)) = parse_env_line(line) {
                    env.insert(key, value);
                }
            }
        }
    }
    
    Ok(env)
}

fn parse_env_line(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    
    // Skip comments and empty lines
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    
    let (key, value) = line.split_once('=')?;
    let key = key.trim().to_string();
    let value = value.trim();
    
    // Remove quotes if present
    let value = if (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
    {
        value[1..value.len()-1].to_string()
    } else {
        value.to_string()
    };
    
    Some((key, value))
}
```

## Configuration Validation

```rust
use validator::Validate;

#[derive(Debug, Validate, Deserialize)]
pub struct BackendConfig {
    #[validate(length(min = 1))]
    pub backend_type: String,
    
    #[validate(url)]
    pub endpoint: Option<String>,
    
    #[validate(range(min = 1, max = 65535))]
    pub port: Option<u16>,
}

pub fn validate_config(config: &DevmerConfig) -> Result<()> {
    config.validate()?;
    
    // Custom validations
    if config.backend.backend_type == "s3" && config.backend.bucket.is_none() {
        return Err(ConfigError::MissingField("backend.bucket".into()));
    }
    
    Ok(())
}
```

## Sensitive Value Handling

Never log or expose sensitive configuration:

```rust
#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    
    // Use _env suffix pattern - value is env var name, not actual secret
    #[serde(default)]
    pub password_env: Option<String>,
}

impl DatabaseConfig {
    pub fn connection_string(&self) -> Result<Secret<String>> {
        let password = match &self.password_env {
            Some(var) => std::env::var(var)
                .map_err(|_| ConfigError::MissingEnv(var.clone()))?,
            None => return Err(ConfigError::MissingField("password_env".into())),
        };
        
        Ok(Secret::new(format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, password, self.host, self.port, self.database
        )))
    }
}
```

## Configuration Precedence

```rust
pub fn load_config(cli_args: &CliArgs) -> Result<DevmerConfig> {
    let mut builder = Config::builder();
    
    // 1. Default values
    builder = builder.add_source(Config::try_from(&DevmerConfig::default())?);
    
    // 2. Config file
    let config_path = &cli_args.config;
    if config_path.exists() {
        builder = builder.add_source(File::from(config_path.as_ref()));
    }
    
    // 3. Environment variables (DEVMER_ prefix)
    builder = builder.add_source(
        Environment::with_prefix("DEVMER")
            .separator("__")
            .try_parsing(true)
    );
    
    // 4. CLI overrides
    for (key, value) in &cli_args.config_overrides {
        builder = builder.set_override(key, value.clone())?;
    }
    
    let config: DevmerConfig = builder.build()?.try_deserialize()?;
    validate_config(&config)?;
    
    Ok(config)
}
```


## Cursor rule: `.cursor/rules/dependency-injection.mdc`

_Dependency injection patterns using dependency_injector crate_

Applies to: `["**/*.rs"]`

# Dependency Injection Guidelines

Devmer uses the `dependency_injector` crate (Armature DI) for dependency injection.

## Core Principles

1. **Program to Interfaces**: Always inject traits, not concrete types
2. **Constructor Injection**: Use `#[inject]` attribute for dependencies
3. **Single Responsibility**: Each service has one purpose
4. **Composition Root**: Resolve dependencies only at app entry point

## Service Interface Pattern

```rust
use async_trait::async_trait;

/// Define service interface as a trait
#[async_trait]
pub trait MyService: Send + Sync {
    async fn do_something(&self, input: &str) -> Result<Output>;
}
```

## Injectable Implementation

```rust
use dependency_injector::{Injectable, Inject};

#[derive(Injectable)]
pub struct MyServiceImpl {
    // Injected dependencies
    #[inject]
    config: Arc<dyn ConfigService>,
    
    #[inject]
    other_service: Arc<dyn OtherService>,
}

#[async_trait]
impl MyService for MyServiceImpl {
    async fn do_something(&self, input: &str) -> Result<Output> {
        let setting = self.config.get("my.setting")?;
        self.other_service.process(input, &setting).await
    }
}
```

## Container Registration

```rust
use dependency_injector::Container;

pub fn register_services(container: &mut Container) {
    // Singleton - one instance shared
    container.register_singleton::<dyn MyService, MyServiceImpl>();
    
    // Lazy singleton - created on first use
    container.register_lazy_singleton::<dyn ExpensiveService, ExpensiveServiceImpl>();
    
    // Factory - new instance each time
    container.register_factory::<dyn StateBackend, _>(|c| {
        let config: Arc<dyn ConfigService> = c.resolve();
        create_backend_from_config(&config)
    });
    
    // Instance - pre-created instance
    container.register_instance::<DevmerConfig>(config);
}
```

## Resolving Dependencies

```rust
// At composition root only
let service: Arc<dyn MyService> = container.resolve();

// Never pass container around - inject specific dependencies instead
```

## Testing with Mocks

```rust
use mockall::automock;

#[automock]
#[async_trait]
pub trait MyService: Send + Sync {
    async fn do_something(&self, input: &str) -> Result<Output>;
}

#[tokio::test]
async fn test_with_mock() {
    let mut mock = MockMyService::new();
    mock.expect_do_something()
        .returning(|_| Ok(Output::default()));
    
    let container = TestContainerBuilder::new()
        .with_mock::<dyn MyService>(mock)
        .build();
    
    // Test...
}
```

## Anti-Patterns to Avoid

```rust
// ❌ BAD: Injecting concrete type
#[inject]
service: Arc<MyServiceImpl>,

// ✅ GOOD: Inject trait
#[inject]
service: Arc<dyn MyService>,

// ❌ BAD: Passing container around
fn do_work(container: &Container) {
    let service = container.resolve();
}

// ✅ GOOD: Inject what you need
fn do_work(service: Arc<dyn MyService>) {
    service.do_something();
}

// ❌ BAD: Service locator in business logic
impl MyService {
    fn process(&self) {
        let other = GLOBAL_CONTAINER.resolve();
    }
}

// ✅ GOOD: Dependencies injected via constructor
#[derive(Injectable)]
struct MyServiceImpl {
    #[inject]
    other: Arc<dyn OtherService>,
}
```


## Cursor rule: `.cursor/rules/error-handling.mdc`

_Error handling patterns for Devmer_

Applies to: `["**/*.rs"]`

# Error Handling Patterns

## Crate-Level Error Types

Each crate should define its own error type in `error.rs`:

```rust
// crates/devmer-state/src/error.rs
use thiserror::Error;

pub type Result<T> = std::result::Result<T, StateError>;

#[derive(Error, Debug)]
pub enum StateError {
    #[error("backend error: {0}")]
    Backend(#[from] BackendError),
    
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("state is locked")]
    Locked(LockInfo),
    
    #[error("state not found for stack: {0}")]
    NotFound(String),
    
    #[error("{0}")]
    Custom(String),
}
```

## Error Context

Always add context to errors:

```rust
// ✅ Good - provides context
fs::read_to_string(&path)
    .with_context(|| format!("failed to read state file: {}", path.display()))?;

// ❌ Bad - loses context
fs::read_to_string(&path)?;
```

## Error Conversion

Use `#[from]` for automatic conversion, but be intentional:

```rust
#[derive(Error, Debug)]
pub enum ProviderError {
    // Automatic conversion from AWS SDK errors
    #[error("AWS API error: {0}")]
    AwsSdk(#[from] aws_sdk_s3::Error),
    
    // Manual conversion for more control
    #[error("configuration error: {0}")]
    Config(String),
}

impl From<ConfigError> for ProviderError {
    fn from(err: ConfigError) -> Self {
        ProviderError::Config(err.to_string())
    }
}
```

## User-Facing Errors

For CLI errors, provide helpful messages:

```rust
#[derive(Error, Debug)]
pub enum CliError {
    #[error("Stack '{name}' not found.\n\nAvailable stacks:\n{}", available.join("\n  - "))]
    StackNotFound {
        name: String,
        available: Vec<String>,
    },
    
    #[error("Configuration error: {message}\n\nHint: {hint}")]
    Config {
        message: String,
        hint: String,
    },
}
```

## Never Panic in Libraries

```rust
// ✅ Good - returns Result
pub fn parse_urn(s: &str) -> Result<Urn> {
    let parts: Vec<&str> = s.split("::").collect();
    if parts.len() != 5 {
        return Err(ParseError::InvalidUrn(s.to_string()));
    }
    // ...
}

// ❌ Bad - panics on invalid input
pub fn parse_urn(s: &str) -> Urn {
    let parts: Vec<&str> = s.split("::").collect();
    assert!(parts.len() == 5, "invalid URN");
    // ...
}
```

## Recoverable vs Unrecoverable

Use `expect()` only for truly unrecoverable situations:

```rust
// ✅ OK - programmer error if this fails
let regex = Regex::new(r"^\d+$").expect("invalid regex pattern");

// ❌ Bad - user input should not panic
let value = input.parse::<i32>().expect("invalid number");
```


## Cursor rule: `.cursor/rules/git-workflow.mdc`

_Git workflow and commit conventions_

Applies to: `["**/*"]`

# Git Workflow

## Branch Naming

- `main` - Production-ready code
- `develop` - Integration branch
- `feature/{ticket}-{description}` - New features
- `fix/{ticket}-{description}` - Bug fixes
- `refactor/{description}` - Code refactoring
- `docs/{description}` - Documentation updates
- `release/v{version}` - Release preparation

## Commit Messages

Follow Conventional Commits:

```
<type>(<scope>): <subject>

[optional body]

[optional footer(s)]
```

### Types
- `feat` - New feature
- `fix` - Bug fix
- `docs` - Documentation
- `style` - Formatting (no code change)
- `refactor` - Code restructuring
- `perf` - Performance improvement
- `test` - Adding tests
- `chore` - Maintenance tasks
- `ci` - CI/CD changes

### Scopes
- `core` - devmer-core crate
- `state` - devmer-state crate
- `cli` - devmer-cli crate
- `tui` - devmer-tui crate
- `aws` - AWS provider
- `gcp` - GCP provider
- `azure` - Azure provider
- `sdk` - SDK components
- `config` - Configuration

### Examples

```
feat(aws): add support for S3 bucket lifecycle rules

Add ability to configure lifecycle rules for S3 buckets including
transitions to different storage classes and expiration policies.

Closes #123
```

```
fix(state): handle concurrent state access race condition

Use optimistic locking with version checking to prevent
lost updates when multiple users access the same stack.

Fixes #456
```

```
refactor(core)!: rename ResourceInputs to ResourceArgs

BREAKING CHANGE: All provider implementations need to update
their type references from ResourceInputs to ResourceArgs.
```

## Pull Requests

### PR Title Format
Same as commit message: `<type>(<scope>): <description>`

### PR Description Template

```markdown
## Summary
Brief description of changes

## Changes
- Change 1
- Change 2

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Manual testing performed

## Documentation
- [ ] Code comments added
- [ ] README updated (if applicable)
- [ ] API docs updated (if applicable)

## Breaking Changes
None / List breaking changes

## Related Issues
Closes #123
```

## Pre-commit Checks

Run before committing:

```bash
# Format code
cargo fmt --all

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test

# Check for secrets
./scripts/check-secrets.sh
```

## Release Process

1. Create release branch: `release/v1.2.0`
2. Update version in all Cargo.toml files
3. Update CHANGELOG.md
4. Create PR to main
5. After merge, tag: `git tag v1.2.0`
6. Push tag: `git push origin v1.2.0`
7. CI publishes crates and creates GitHub release


## Cursor rule: `.cursor/rules/project-overview.mdc`

_Overview of Devmer project architecture and conventions_

Applies to: `["**/*.rs", "**/*.toml", "**/*.md"]`

# Devmer Project Overview

Devmer is a Rust-based Infrastructure as Code (IaC) tool similar to Pulumi/Terraform with the following key characteristics:

## Project Goals
- Self-hosted state management (no proprietary cloud service required)
- Multi-language SDK support (Python, TypeScript/Deno/Bun, Go, Rust)
- Modular provider architecture (separate crates per cloud provider)
- Enterprise features (SOC2 compliance, multi-organization support)

## Crate Organization
The project uses a workspace with multiple crates:
- `devmer-cli` - CLI application
- `devmer-core` - Core engine, resource graph, execution
- `devmer-state` - State management backends
- `devmer-secrets` - Secrets encryption engine
- `devmer-audit` - Audit logging & compliance
- `devmer-config` - Configuration parsing & environment interpolation
- `devmer-migrate` - State import from Terraform/Pulumi
- `devmer-org` - Multi-organization management
- `devmer-tui` - Terminal User Interface
- `devmer-sdk` - Base SDK traits and types
- `devmer-rpc` - gRPC/IPC for language host communication
- `providers/devmer-aws`, `devmer-gcp`, `devmer-azure`, etc.

## Key Design Principles
1. **Async-first**: Use `tokio` runtime, all I/O operations are async
2. **Type-safe**: Leverage Rust's type system for compile-time guarantees
3. **Modular**: Each feature in its own crate with clear boundaries
4. **Testable**: Design for unit and integration testing
5. **Secure**: Never log secrets, use secure memory handling

## Naming Conventions
- Crates: `devmer-{feature}` (kebab-case)
- Modules: `snake_case`
- Types: `PascalCase`
- Functions: `snake_case`
- Constants: `SCREAMING_SNAKE_CASE`
- Resource types: `{provider}:{service}:{Resource}` (e.g., `aws:s3:Bucket`)


## Cursor rule: `.cursor/rules/providers.mdc`

_Guidelines for implementing cloud providers_

Applies to: `["**/providers/**/*.rs", "**/devmer-aws/**", "**/devmer-gcp/**", "**/devmer-azure/**"]`

# Provider Implementation Guidelines

## Provider Trait

All providers must implement the `Provider` trait:

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    /// Provider identifier (e.g., "aws", "gcp", "azure")
    fn name(&self) -> &str;
    
    /// Provider version
    fn version(&self) -> &str;
    
    /// Configure the provider with settings
    async fn configure(&mut self, config: ProviderConfig) -> Result<()>;
    
    /// Get the schema for all resources
    fn get_schema(&self) -> &ProviderSchema;
    
    /// Check provider health/connectivity
    async fn health_check(&self) -> Result<HealthStatus>;
}
```

## Resource Trait

Each resource type implements the `Resource` trait:

```rust
#[async_trait]
pub trait Resource: Send + Sync {
    /// Resource type name (e.g., "aws:s3:Bucket")
    fn type_name(&self) -> &str;
    
    /// Validate inputs before operations
    async fn check(&self, inputs: &ResourceInputs) -> Result<CheckResult>;
    
    /// Compare desired vs actual state
    async fn diff(
        &self,
        id: &str,
        olds: &ResourceInputs,
        news: &ResourceInputs,
    ) -> Result<DiffResult>;
    
    /// Create a new resource
    async fn create(&self, inputs: &ResourceInputs) -> Result<CreateResult>;
    
    /// Read current resource state
    async fn read(&self, id: &str) -> Result<ReadResult>;
    
    /// Update an existing resource
    async fn update(
        &self,
        id: &str,
        olds: &ResourceInputs,
        news: &ResourceInputs,
    ) -> Result<UpdateResult>;
    
    /// Delete a resource
    async fn delete(&self, id: &str) -> Result<()>;
}
```

## Resource Naming

```rust
// Resource type format: {provider}:{service}:{Resource}
pub const TYPE_NAME: &str = "aws:s3:Bucket";

// URN format: urn:devmer:{stack}::{project}::{type}::{name}
pub fn create_urn(stack: &str, project: &str, type_name: &str, name: &str) -> String {
    format!("urn:devmer:{}::{}::{}::{}", stack, project, type_name, name)
}
```

## Input/Output Schemas

Define schemas for IDE support and validation:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BucketArgs {
    /// The name of the bucket
    #[serde(default)]
    pub bucket: Option<String>,
    
    /// Enable versioning
    #[serde(default)]
    pub versioning: Option<bool>,
    
    /// Tags to apply to the bucket
    #[serde(default)]
    pub tags: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketOutputs {
    /// The ARN of the bucket
    pub arn: String,
    
    /// The regional domain name
    pub regional_domain_name: String,
    
    /// The bucket name (may differ from input if auto-generated)
    pub bucket: String,
}
```

## Error Handling

Use provider-specific error types:

```rust
#[derive(Error, Debug)]
pub enum AwsError {
    #[error("AWS API error: {message}")]
    Api { 
        message: String, 
        code: Option<String>,
        request_id: Option<String>,
    },
    
    #[error("resource not found: {resource_type} {id}")]
    NotFound { resource_type: String, id: String },
    
    #[error("permission denied: {action} on {resource}")]
    PermissionDenied { action: String, resource: String },
    
    #[error("rate limited, retry after {retry_after:?}")]
    RateLimited { retry_after: Option<Duration> },
}
```

## Retry Logic

Implement exponential backoff for transient failures:

```rust
pub async fn with_retry<T, F, Fut>(
    operation: F,
    max_retries: u32,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut attempt = 0;
    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if e.is_retryable() && attempt < max_retries => {
                let delay = Duration::from_millis(100 * 2u64.pow(attempt));
                tokio::time::sleep(delay).await;
                attempt += 1;
            }
            Err(e) => return Err(e),
        }
    }
}
```

## Testing Providers

Use mocking for unit tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    
    #[tokio::test]
    async fn test_create_bucket() {
        let mut mock_client = MockS3Client::new();
        mock_client
            .expect_create_bucket()
            .with(eq("test-bucket"))
            .returning(|_| Ok(CreateBucketOutput::default()));
        
        let provider = AwsProvider::with_client(mock_client);
        let bucket = Bucket::new(&provider);
        
        let result = bucket.create(&BucketArgs {
            bucket: Some("test-bucket".into()),
            ..Default::default()
        }).await;
        
        assert!(result.is_ok());
    }
}
```


## Cursor rule: `.cursor/rules/rust-conventions.mdc`

_Rust coding conventions and best practices for Devmer_

Applies to: `["**/*.rs"]`

# Rust Coding Conventions

## General Style
- Follow the official Rust style guide (rustfmt)
- Use `cargo fmt` before committing
- Use `cargo clippy` and address all warnings
- Maximum line length: 100 characters

## Imports
- Group imports in this order, separated by blank lines:
  1. Standard library (`std::`)
  2. External crates
  3. Internal crates (`devmer_*`)
  4. Current crate modules (`crate::`, `super::`)
- Prefer explicit imports over glob imports (`use module::*`)

```rust
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use devmer_core::Resource;

use crate::error::Result;
```

## Error Handling
- Define crate-specific error types using `thiserror`
- Use `anyhow` for application-level error handling in CLI
- Always provide context with errors using `.context()` or `.with_context()`
- Never use `.unwrap()` in library code; use `.expect()` only with clear messages

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StateError {
    #[error("failed to read state from backend: {0}")]
    ReadError(#[from] std::io::Error),
    
    #[error("state is locked by {holder} since {since}")]
    Locked { holder: String, since: DateTime<Utc> },
    
    #[error("state version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u64, actual: u64 },
}
```

## Async Patterns
- Use `#[async_trait]` for async trait methods
- Prefer `tokio::spawn` for concurrent tasks
- Use `tokio::select!` for concurrent operations with cancellation
- Always handle cancellation gracefully

```rust
#[async_trait]
pub trait StateBackend: Send + Sync {
    async fn get_state(&self, stack: &str) -> Result<Option<StackState>>;
    async fn save_state(&self, stack: &str, state: &StackState) -> Result<()>;
}
```

## Documentation
- All public items must have doc comments
- Use `///` for item documentation
- Use `//!` for module-level documentation
- Include examples in doc comments using ```` ```rust ````
- Document panics, errors, and safety considerations

```rust
/// Creates a new S3 state backend.
///
/// # Arguments
/// * `bucket` - The S3 bucket name for state storage
/// * `region` - AWS region for the bucket
///
/// # Errors
/// Returns `StateError::ConfigError` if the bucket doesn't exist.
///
/// # Example
/// ```rust
/// let backend = S3Backend::new("my-state-bucket", "us-east-1").await?;
/// ```
pub async fn new(bucket: &str, region: &str) -> Result<Self> {
    // ...
}
```

## Testing
- Unit tests go in the same file using `#[cfg(test)]` module
- Integration tests go in `tests/` directory
- Use descriptive test names: `test_{function}_{scenario}_{expected_result}`
- Use `#[tokio::test]` for async tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_save_state_creates_new_state_file() {
        // Arrange
        let backend = LocalBackend::new(temp_dir()).await.unwrap();
        let state = StackState::default();
        
        // Act
        backend.save_state("test-stack", &state).await.unwrap();
        
        // Assert
        assert!(backend.get_state("test-stack").await.unwrap().is_some());
    }
}
```


## Cursor rule: `.cursor/rules/sdk.mdc`

_SDK design patterns for multi-language support_

Applies to: `["**/devmer-sdk/**/*.rs", "**/devmer-rpc/**/*.rs", "**/sdks/**"]`

# SDK Development Guidelines

## Core SDK Traits

Define language-agnostic traits:

```rust
/// Base trait for all resources
#[async_trait]
pub trait ResourceBase: Send + Sync {
    /// Get the resource URN
    fn urn(&self) -> &str;
    
    /// Get the resource type
    fn resource_type(&self) -> &str;
    
    /// Get all outputs
    fn outputs(&self) -> &ResourceOutputs;
}

/// Output represents an asynchronously-computed value
pub struct Output<T> {
    inner: Arc<OutputInner<T>>,
}

impl<T: Clone + Send + Sync + 'static> Output<T> {
    pub fn new(value: T) -> Self { /* ... */ }
    
    pub fn from_future<F: Future<Output = T> + Send + 'static>(fut: F) -> Self { /* ... */ }
    
    pub fn apply<U, F>(&self, f: F) -> Output<U>
    where
        F: Fn(T) -> U + Send + Sync + 'static,
        U: Clone + Send + Sync + 'static,
    { /* ... */ }
}
```

## Resource Options

Standard options for all resources:

```rust
#[derive(Default, Clone)]
pub struct ResourceOptions {
    /// Parent resource for hierarchical organization
    pub parent: Option<Arc<dyn ResourceBase>>,
    
    /// Explicit dependencies
    pub depends_on: Vec<Arc<dyn ResourceBase>>,
    
    /// Protect from deletion
    pub protect: bool,
    
    /// Provider to use (overrides default)
    pub provider: Option<Arc<dyn Provider>>,
    
    /// Aliases for state migration
    pub aliases: Vec<Alias>,
    
    /// Transformations to apply
    pub transformations: Vec<Box<dyn Transformation>>,
    
    /// Custom timeouts
    pub timeouts: Option<Timeouts>,
    
    /// Import existing resource
    pub import_id: Option<String>,
}
```

## Component Resources

Base for logical groupings:

```rust
pub trait ComponentResource: ResourceBase {
    fn component_type(&self) -> &str;
    
    fn register_outputs(&self, outputs: HashMap<String, Output<Value>>);
}

// Macro for defining components
#[macro_export]
macro_rules! component {
    ($name:ident, $type:expr, { $($field:ident: $ty:ty),* }) => {
        pub struct $name {
            urn: String,
            $(pub $field: Output<$ty>,)*
        }
        
        impl ComponentResource for $name {
            fn component_type(&self) -> &str { $type }
            // ...
        }
    };
}
```

## gRPC Protocol

Define protobuf messages for language host communication:

```protobuf
// proto/engine.proto

syntax = "proto3";
package devmer.engine.v1;

service Engine {
    rpc RegisterResource(RegisterResourceRequest) returns (RegisterResourceResponse);
    rpc ReadResource(ReadResourceRequest) returns (ReadResourceResponse);
    rpc Invoke(InvokeRequest) returns (InvokeResponse);
    rpc Log(LogRequest) returns (LogResponse);
}

message RegisterResourceRequest {
    string type = 1;
    string name = 2;
    google.protobuf.Struct inputs = 3;
    ResourceOptions options = 4;
}

message RegisterResourceResponse {
    string urn = 1;
    string id = 2;
    google.protobuf.Struct outputs = 3;
}
```

## Language Host Interface

Interface for SDK implementations:

```rust
#[async_trait]
pub trait LanguageHost: Send + Sync {
    /// Start the language runtime
    async fn start(&mut self, program: &Path) -> Result<()>;
    
    /// Wait for program completion
    async fn wait(&mut self) -> Result<ExitStatus>;
    
    /// Send resource registration to engine
    async fn on_resource_registered(&self, resource: &RegisteredResource) -> Result<()>;
    
    /// Handle engine callbacks
    async fn handle_callback(&self, callback: Callback) -> Result<Value>;
}
```

## Python SDK Patterns

```python
# sdks/python/devmer/resource.py

from typing import TypeVar, Generic, Optional, Any
from dataclasses import dataclass

T = TypeVar('T')

class Output(Generic[T]):
    """Represents an asynchronously-computed value."""
    
    def __init__(self, value: T | None = None):
        self._value = value
        self._resolved = value is not None
    
    def apply(self, func: Callable[[T], U]) -> 'Output[U]':
        """Transform the output value."""
        if self._resolved:
            return Output(func(self._value))
        return Output()  # Will be resolved later
    
    def __getattr__(self, name: str) -> 'Output[Any]':
        """Allow property access on outputs."""
        return self.apply(lambda v: getattr(v, name))


class Resource:
    """Base class for all resources."""
    
    def __init__(
        self,
        resource_type: str,
        name: str,
        props: dict,
        opts: Optional[ResourceOptions] = None
    ):
        self._type = resource_type
        self._name = name
        self._urn = register_resource(resource_type, name, props, opts)
```

## TypeScript SDK Patterns

```typescript
// sdks/typescript/src/resource.ts

export class Output<T> {
    private readonly promise: Promise<T>;
    
    constructor(valueOrPromise: T | Promise<T>) {
        this.promise = Promise.resolve(valueOrPromise);
    }
    
    apply<U>(func: (value: T) => U | Promise<U>): Output<U> {
        return new Output(this.promise.then(func));
    }
    
    get<K extends keyof T>(key: K): Output<T[K]> {
        return this.apply(v => v[key]);
    }
}

export abstract class Resource {
    public readonly urn: Output<string>;
    
    constructor(
        type: string,
        name: string,
        props: Record<string, any>,
        opts?: ResourceOptions
    ) {
        const registration = registerResource(type, name, props, opts);
        this.urn = new Output(registration.then(r => r.urn));
    }
}
```


## Cursor rule: `.cursor/rules/security.mdc`

_Security best practices for Devmer_

Applies to: `["**/*.rs", "**/*.toml"]`

# Security Best Practices

## Secret Handling

### Never Log Secrets

```rust
// ✅ Good - mask secrets in logs
tracing::info!(
    backend = %backend_type,
    bucket = %bucket,
    "connected to state backend"
);

// ❌ Bad - logging credentials
tracing::debug!("connecting with key: {}", api_key);
```

### Use Secure Memory

```rust
use secrecy::{Secret, ExposeSecret};
use zeroize::Zeroize;

pub struct Credentials {
    pub access_key: String,
    pub secret_key: Secret<String>,  // Wrapped in Secret<>
}

impl Credentials {
    pub fn authenticate(&self) -> Result<()> {
        // Only expose when absolutely necessary
        let secret = self.secret_key.expose_secret();
        // Use secret...
    }
}

// For sensitive byte arrays
let mut key_bytes = derive_key(password);
// ... use key_bytes ...
key_bytes.zeroize();  // Zero memory when done
```

### Encryption Patterns

```rust
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use aes_gcm::aead::Aead;

pub fn encrypt_secret(plaintext: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key)?;
    let nonce = generate_random_nonce();  // 12 bytes
    
    let ciphertext = cipher.encrypt(&nonce, plaintext)?;
    
    // Prepend nonce to ciphertext
    let mut result = nonce.to_vec();
    result.extend(ciphertext);
    Ok(result)
}
```

## Input Validation

### Validate All User Input

```rust
pub fn validate_stack_name(name: &str) -> Result<()> {
    // Length check
    if name.is_empty() || name.len() > 100 {
        return Err(ValidationError::InvalidLength);
    }
    
    // Character whitelist
    let valid_pattern = Regex::new(r"^[a-zA-Z][a-zA-Z0-9-_]*$").unwrap();
    if !valid_pattern.is_match(name) {
        return Err(ValidationError::InvalidCharacters);
    }
    
    Ok(())
}
```

### Sanitize Path Inputs

```rust
use std::path::{Path, PathBuf};

pub fn safe_join(base: &Path, user_input: &str) -> Result<PathBuf> {
    let path = base.join(user_input);
    let canonical = path.canonicalize()?;
    
    // Ensure path is still under base directory
    if !canonical.starts_with(base.canonicalize()?) {
        return Err(SecurityError::PathTraversal);
    }
    
    Ok(canonical)
}
```

## Authentication & Authorization

### Role Assumption

```rust
pub async fn assume_role(role_arn: &str) -> Result<Credentials> {
    let sts_client = aws_sdk_sts::Client::new(&config);
    
    let assumed = sts_client
        .assume_role()
        .role_arn(role_arn)
        .role_session_name("devmer-session")
        .duration_seconds(3600)  // 1 hour max
        .send()
        .await?;
    
    // Credentials auto-expire
    Ok(Credentials::from(assumed.credentials))
}
```

## Audit Logging

### Log Security-Relevant Events

```rust
pub async fn access_secret(&self, name: &str, actor: &Actor) -> Result<Secret<String>> {
    // Log the access (but not the value!)
    audit::log(AuditEvent {
        event_type: AuditEventType::SecretAccessed,
        actor: actor.clone(),
        resource: Some(format!("secret:{}", name)),
        outcome: Outcome::Success,
        ..Default::default()
    }).await?;
    
    self.secrets_provider.get(name).await
}
```

## Dependency Security

- Run `cargo audit` regularly
- Pin dependencies with exact versions in production
- Review security advisories for dependencies
- Use `cargo deny` for license and vulnerability checks


## Cursor rule: `.cursor/rules/state-management.mdc`

_State management patterns and backends_

Applies to: `["**/devmer-state/**/*.rs", "**/state/**/*.rs"]`

# State Management Guidelines

## State Backend Trait

All backends implement this interface:

```rust
#[async_trait]
pub trait StateBackend: Send + Sync {
    /// Get state for a stack
    async fn get_state(&self, stack: &str) -> Result<Option<StackState>>;
    
    /// Save state for a stack
    async fn save_state(&self, stack: &str, state: &StackState) -> Result<()>;
    
    /// Delete state for a stack
    async fn delete_state(&self, stack: &str) -> Result<()>;
    
    /// Acquire a lock on the state
    async fn lock(&self, stack: &str, info: &LockInfo) -> Result<LockId>;
    
    /// Release a lock
    async fn unlock(&self, stack: &str, lock_id: &LockId) -> Result<()>;
    
    /// List all stacks
    async fn list_stacks(&self) -> Result<Vec<String>>;
    
    /// Get state history (for backends that support versioning)
    async fn get_history(&self, stack: &str, limit: usize) -> Result<Vec<StateVersion>> {
        Ok(vec![])  // Default: no history support
    }
}
```

## State Format

Use a versioned JSON format:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackState {
    /// State format version
    pub version: u32,
    
    /// Stack identifier
    pub stack: String,
    
    /// Project name
    pub project: String,
    
    /// Resources in this stack
    pub resources: Vec<ResourceState>,
    
    /// Pending operations (for crash recovery)
    pub pending_operations: Vec<PendingOperation>,
    
    /// Secrets provider configuration
    pub secrets_provider: Option<SecretsProviderConfig>,
    
    /// Metadata
    pub metadata: StateMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceState {
    /// Unique resource identifier
    pub urn: String,
    
    /// Resource type
    #[serde(rename = "type")]
    pub resource_type: String,
    
    /// Cloud provider ID
    pub id: Option<String>,
    
    /// Input properties
    pub inputs: serde_json::Value,
    
    /// Output properties
    pub outputs: serde_json::Value,
    
    /// Parent resource URN
    pub parent: Option<String>,
    
    /// Dependencies
    pub dependencies: Vec<String>,
    
    /// Timestamps
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
}
```

## Locking

Implement distributed locking for concurrent access:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockInfo {
    pub id: String,
    pub holder: String,       // Who holds the lock
    pub operation: String,    // What operation is running
    pub created: DateTime<Utc>,
    pub expires: Option<DateTime<Utc>>,
}

impl StateBackend for S3Backend {
    async fn lock(&self, stack: &str, info: &LockInfo) -> Result<LockId> {
        // Use DynamoDB for distributed locking
        let lock_key = format!("devmer-lock-{}", stack);
        
        // Try to acquire lock with conditional write
        self.dynamodb
            .put_item()
            .table_name(&self.lock_table)
            .item("LockID", AttributeValue::S(lock_key.clone()))
            .item("Info", AttributeValue::S(serde_json::to_string(info)?))
            .condition_expression("attribute_not_exists(LockID)")
            .send()
            .await
            .map_err(|e| StateError::LockFailed(e.to_string()))?;
        
        Ok(LockId(lock_key))
    }
}
```

## State Encryption

Encrypt sensitive data in state:

```rust
pub struct EncryptedState {
    /// Encryption envelope
    pub envelope: EncryptionEnvelope,
    
    /// Encrypted state blob
    pub ciphertext: Vec<u8>,
}

impl StateBackend for EncryptedBackend {
    async fn save_state(&self, stack: &str, state: &StackState) -> Result<()> {
        // Serialize state
        let plaintext = serde_json::to_vec(state)?;
        
        // Encrypt
        let encrypted = self.encryptor.encrypt(&plaintext).await?;
        
        // Save encrypted state
        self.inner.save_state(stack, &encrypted).await
    }
    
    async fn get_state(&self, stack: &str) -> Result<Option<StackState>> {
        // Get encrypted state
        let encrypted = match self.inner.get_state(stack).await? {
            Some(e) => e,
            None => return Ok(None),
        };
        
        // Decrypt
        let plaintext = self.encryptor.decrypt(&encrypted).await?;
        
        // Deserialize
        let state: StackState = serde_json::from_slice(&plaintext)?;
        Ok(Some(state))
    }
}
```

## Crash Recovery

Handle interrupted operations:

```rust
pub async fn recover_state(&self, stack: &str) -> Result<()> {
    let state = self.get_state(stack).await?.ok_or(StateError::NotFound)?;
    
    for pending in &state.pending_operations {
        match pending.operation_type {
            OperationType::Create => {
                // Check if resource was actually created
                if self.provider.resource_exists(&pending.resource_id).await? {
                    // Update state to reflect creation
                    self.mark_operation_complete(stack, pending).await?;
                } else {
                    // Remove pending operation
                    self.remove_pending_operation(stack, pending).await?;
                }
            }
            OperationType::Delete => {
                // Check if resource still exists
                if !self.provider.resource_exists(&pending.resource_id).await? {
                    // Update state to reflect deletion
                    self.mark_operation_complete(stack, pending).await?;
                }
                // If still exists, retry deletion on next apply
            }
            // ...
        }
    }
    
    Ok(())
}
```

## State Migration

Support migrating between state versions:

```rust
pub fn migrate_state(state: serde_json::Value) -> Result<StackState> {
    let version = state["version"].as_u64().unwrap_or(1);
    
    match version {
        1 => migrate_v1_to_current(state),
        2 => migrate_v2_to_current(state),
        CURRENT_VERSION => serde_json::from_value(state).map_err(Into::into),
        v => Err(StateError::UnsupportedVersion(v)),
    }
}
```


## Cursor rule: `.cursor/rules/testing.mdc`

_Testing patterns and conventions for Devmer_

Applies to: `["**/*.rs", "**/tests/**"]`

# Testing Guidelines

## Test Organization

```
crates/devmer-state/
├── src/
│   ├── lib.rs
│   └── backends/
│       └── s3.rs          # Contains unit tests in #[cfg(test)] mod
├── tests/
│   ├── integration_s3.rs  # Integration tests
│   └── fixtures/          # Test data files
└── benches/
    └── state_ops.rs       # Benchmarks
```

## Unit Tests

Place unit tests in the same file as the code:

```rust
// src/backends/s3.rs

pub struct S3Backend { /* ... */ }

impl S3Backend {
    pub async fn get_state(&self, stack: &str) -> Result<Option<State>> {
        // implementation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    fn create_test_state() -> State {
        State {
            version: 1,
            resources: vec![],
            ..Default::default()
        }
    }
    
    #[tokio::test]
    async fn test_get_state_returns_none_for_missing_stack() {
        let backend = S3Backend::new_mock();
        
        let result = backend.get_state("nonexistent").await.unwrap();
        
        assert!(result.is_none());
    }
    
    #[tokio::test]
    async fn test_get_state_returns_saved_state() {
        let backend = S3Backend::new_mock();
        let state = create_test_state();
        
        backend.save_state("my-stack", &state).await.unwrap();
        let result = backend.get_state("my-stack").await.unwrap();
        
        assert!(result.is_some());
        assert_eq!(result.unwrap().version, 1);
    }
}
```

## Integration Tests

Use `tests/` directory for integration tests:

```rust
// tests/integration_s3.rs

use devmer_state::{S3Backend, StateBackend};
use testcontainers::{clients, images::minio};

#[tokio::test]
#[ignore] // Run with: cargo test -- --ignored
async fn test_s3_backend_with_real_minio() {
    let docker = clients::Cli::default();
    let minio = docker.run(minio::MinIO::default());
    
    let endpoint = format!("http://localhost:{}", minio.get_host_port_ipv4(9000));
    let backend = S3Backend::new(&endpoint, "test-bucket").await.unwrap();
    
    // Test operations...
}
```

## Test Naming Convention

Use descriptive names: `test_{function}_{scenario}_{expected}`

```rust
#[test]
fn test_parse_urn_valid_urn_returns_parsed_components() { }

#[test]
fn test_parse_urn_missing_stack_returns_error() { }

#[test]
fn test_parse_urn_empty_string_returns_error() { }
```

## Async Testing

Use `#[tokio::test]` for async tests:

```rust
#[tokio::test]
async fn test_concurrent_state_access() {
    let backend = Arc::new(LocalBackend::new().await.unwrap());
    
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let backend = Arc::clone(&backend);
            tokio::spawn(async move {
                backend.get_state(&format!("stack-{}", i)).await
            })
        })
        .collect();
    
    for handle in handles {
        assert!(handle.await.unwrap().is_ok());
    }
}
```

## Mocking

Use `mockall` for mocking traits:

```rust
use mockall::{automock, predicate::*};

#[automock]
#[async_trait]
pub trait StateBackend {
    async fn get_state(&self, stack: &str) -> Result<Option<State>>;
    async fn save_state(&self, stack: &str, state: &State) -> Result<()>;
}

#[tokio::test]
async fn test_engine_uses_backend() {
    let mut mock = MockStateBackend::new();
    mock.expect_get_state()
        .with(eq("production"))
        .returning(|_| Ok(Some(State::default())));
    
    let engine = Engine::new(Box::new(mock));
    let result = engine.load_stack("production").await;
    
    assert!(result.is_ok());
}
```

## Test Fixtures

Use `include_str!` or fixtures directory:

```rust
const SAMPLE_STATE: &str = include_str!("../fixtures/sample_state.json");

#[test]
fn test_parse_state_from_json() {
    let state: State = serde_json::from_str(SAMPLE_STATE).unwrap();
    assert_eq!(state.resources.len(), 3);
}
```

## Property-Based Testing

Use `proptest` for property-based tests:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_urn_roundtrip(
        stack in "[a-z][a-z0-9-]{0,20}",
        project in "[a-z][a-z0-9-]{0,20}",
        name in "[a-z][a-z0-9-]{0,20}"
    ) {
        let urn = Urn::new(&stack, &project, "aws:s3:Bucket", &name);
        let parsed = Urn::parse(&urn.to_string()).unwrap();
        
        assert_eq!(urn.stack, parsed.stack);
        assert_eq!(urn.project, parsed.project);
        assert_eq!(urn.name, parsed.name);
    }
}
```

## Test Coverage

Run coverage with `cargo-tarpaulin`:

```bash
cargo tarpaulin --out Html --output-dir coverage/
```

Aim for >80% coverage on core logic.


## Cursor rule: `.cursor/rules/tui.mdc`

_TUI development patterns using Ratatui_

Applies to: `["**/devmer-tui/**/*.rs", "**/tui/**/*.rs"]`

# TUI Development Guidelines

## Architecture

Use the Elm-inspired architecture:

```rust
// Model - Application state
pub struct App {
    pub stacks: Vec<Stack>,
    pub selected_stack: usize,
    pub mode: AppMode,
    pub logs: Vec<LogEntry>,
}

// Messages - User actions
pub enum Message {
    SelectStack(usize),
    DeployStack,
    RefreshState,
    Quit,
}

// Update - State transitions
impl App {
    pub fn update(&mut self, msg: Message) -> Option<Command> {
        match msg {
            Message::SelectStack(idx) => {
                self.selected_stack = idx;
                None
            }
            Message::DeployStack => {
                self.mode = AppMode::Deploying;
                Some(Command::Deploy(self.current_stack().clone()))
            }
            Message::Quit => {
                Some(Command::Quit)
            }
            _ => None,
        }
    }
}
```

## Ratatui Widgets

Create reusable widget components:

```rust
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub struct ResourceTree<'a> {
    resources: &'a [Resource],
    selected: usize,
}

impl<'a> Widget for ResourceTree<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let items: Vec<ListItem> = self.resources
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let style = if i == self.selected {
                    Style::default().bg(Color::Blue)
                } else {
                    Style::default()
                };
                ListItem::new(format_resource(r)).style(style)
            })
            .collect();
        
        let list = List::new(items)
            .block(Block::default()
                .title("Resources")
                .borders(Borders::ALL));
        
        list.render(area, buf);
    }
}
```

## Layout

Use constraint-based layouts:

```rust
pub fn ui(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Main content
            Constraint::Length(5),  // Logs
            Constraint::Length(1),  // Status bar
        ])
        .split(frame.size());
    
    render_header(frame, chunks[0], app);
    render_main(frame, chunks[1], app);
    render_logs(frame, chunks[2], app);
    render_status_bar(frame, chunks[3], app);
}
```

## Event Handling

Handle keyboard input:

```rust
use crossterm::event::{self, Event, KeyCode, KeyModifiers};

pub async fn handle_events(tx: mpsc::Sender<Message>) -> Result<()> {
    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                let msg = match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), _) => Message::Quit,
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => Message::Quit,
                    (KeyCode::Up, _) | (KeyCode::Char('k'), _) => Message::Up,
                    (KeyCode::Down, _) | (KeyCode::Char('j'), _) => Message::Down,
                    (KeyCode::Enter, _) => Message::Select,
                    (KeyCode::Char('d'), _) => Message::Deploy,
                    (KeyCode::Char('r'), _) => Message::Refresh,
                    _ => continue,
                };
                tx.send(msg).await?;
            }
        }
    }
}
```

## Async Operations

Handle async operations without blocking UI:

```rust
pub enum Command {
    Deploy(Stack),
    Refresh,
    Quit,
}

pub async fn run_app() -> Result<()> {
    let mut app = App::new();
    let (cmd_tx, mut cmd_rx) = mpsc::channel(32);
    let (event_tx, mut event_rx) = mpsc::channel(32);
    
    // Spawn event handler
    tokio::spawn(handle_events(event_tx));
    
    loop {
        terminal.draw(|f| ui(f, &app))?;
        
        tokio::select! {
            Some(msg) = event_rx.recv() => {
                if let Some(cmd) = app.update(msg) {
                    match cmd {
                        Command::Quit => break,
                        Command::Deploy(stack) => {
                            tokio::spawn(deploy_stack(stack, cmd_tx.clone()));
                        }
                        _ => {}
                    }
                }
            }
            Some(result) = cmd_rx.recv() => {
                app.handle_command_result(result);
            }
        }
    }
    
    Ok(())
}
```

## Themes

Support customizable themes:

```rust
pub struct Theme {
    pub background: Color,
    pub foreground: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    pub selected_bg: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            background: Color::Rgb(30, 30, 30),
            foreground: Color::Rgb(220, 220, 220),
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::Cyan,
            selected_bg: Color::Rgb(60, 60, 100),
        }
    }
    
    pub fn light() -> Self {
        Self {
            background: Color::Rgb(250, 250, 250),
            foreground: Color::Rgb(30, 30, 30),
            // ...
        }
    }
}
```

