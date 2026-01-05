//! Language host for external runtimes

use crate::error::{Result, RuntimeError};
use crate::registry::ResourceRegistry;
use crate::runtime::{LanguageRuntime, RunResult, RuntimeConfig, RuntimeKind};
use async_trait::async_trait;
use std::process::Stdio;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Language host for external process-based runtimes
pub struct LanguageHost {
    kind: RuntimeKind,
}

impl LanguageHost {
    /// Create a new language host
    pub fn new(kind: RuntimeKind) -> Self {
        Self { kind }
    }

    /// Find the executable for this runtime
    async fn find_executable(&self) -> Result<String> {
        let exe_name = self.kind.executable();

        if exe_name.is_empty() {
            return Err(RuntimeError::RuntimeNotFound(format!("{:?}", self.kind)));
        }

        match which::which(exe_name) {
            Ok(path) => Ok(path.to_string_lossy().to_string()),
            Err(_) => Err(RuntimeError::RuntimeNotFound(exe_name.to_string())),
        }
    }

    /// Build the command to run the program
    fn build_command(
        &self,
        config: &RuntimeConfig,
        executable: &str,
    ) -> Command {
        let mut cmd = Command::new(executable);
        cmd.current_dir(&config.working_dir);

        // Set environment variables
        for (key, value) in &config.env_vars {
            cmd.env(key, value);
        }

        // Set Devmer-specific environment
        cmd.env("DEVMER_STACK", &config.stack);
        cmd.env("DEVMER_PREVIEW", if config.preview { "1" } else { "0" });
        cmd.env("DEVMER_GRPC_ADDRESS", &config.grpc_address);

        // Configure based on runtime
        match self.kind {
            RuntimeKind::Node => {
                // Use ts-node or tsx for TypeScript
                let entry = config.entry_point.to_string_lossy();
                if entry.ends_with(".ts") {
                    cmd.arg("--import").arg("tsx");
                }
                cmd.arg(&config.entry_point);
            }
            RuntimeKind::Deno => {
                cmd.arg("run");
                cmd.arg("--allow-all"); // Full permissions for IaC
                cmd.arg(&config.entry_point);
            }
            RuntimeKind::Bun => {
                cmd.arg("run");
                cmd.arg(&config.entry_point);
            }
            RuntimeKind::Python => {
                cmd.arg(&config.entry_point);
            }
            RuntimeKind::Go => {
                cmd.arg("run");
                cmd.arg(&config.entry_point);
            }
            RuntimeKind::Rhai => {
                // Rhai is embedded, shouldn't reach here
            }
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        cmd
    }
}

#[async_trait]
impl LanguageRuntime for LanguageHost {
    fn kind(&self) -> RuntimeKind {
        self.kind
    }

    async fn is_available(&self) -> bool {
        self.find_executable().await.is_ok()
    }

    async fn version(&self) -> Result<String> {
        let executable = self.find_executable().await?;

        let version_arg = match self.kind {
            RuntimeKind::Node | RuntimeKind::Deno | RuntimeKind::Bun => "--version",
            RuntimeKind::Python => "--version",
            RuntimeKind::Go => "version",
            RuntimeKind::Rhai => return Ok("embedded".to_string()),
        };

        let output = Command::new(&executable)
            .arg(version_arg)
            .output()
            .await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(RuntimeError::execution_failed("Failed to get version"))
        }
    }

    async fn run(&self, config: &RuntimeConfig) -> Result<RunResult> {
        let start = Instant::now();
        let executable = self.find_executable().await?;

        info!(
            runtime = %self.kind,
            entry = %config.entry_point.display(),
            "Running program"
        );

        let mut cmd = self.build_command(config, &executable);
        let mut child = cmd.spawn()?;

        let registry = ResourceRegistry::new();

        // Collect stdout
        let stdout_handle = child.stdout.take();
        let stderr_handle = child.stderr.take();

        let mut stdout_lines = Vec::new();
        let mut stderr_lines = Vec::new();

        // Read stdout
        if let Some(stdout) = stdout_handle {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                debug!(line = %line, "stdout");
                stdout_lines.push(line);
            }
        }

        // Read stderr
        if let Some(stderr) = stderr_handle {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                warn!(line = %line, "stderr");
                stderr_lines.push(line);
            }
        }

        // Wait for process with timeout
        let timeout = config.timeout;
        let status = tokio::time::timeout(timeout, child.wait()).await;

        let duration = start.elapsed();

        match status {
            Ok(Ok(exit_status)) => {
                let code = exit_status.code().unwrap_or(-1);

                if exit_status.success() {
                    Ok(RunResult {
                        success: true,
                        exit_code: Some(code),
                        resources: registry,
                        stdout: stdout_lines.join("\n"),
                        stderr: stderr_lines.join("\n"),
                        duration,
                        errors: vec![],
                    })
                } else {
                    Ok(RunResult {
                        success: false,
                        exit_code: Some(code),
                        resources: registry,
                        stdout: stdout_lines.join("\n"),
                        stderr: stderr_lines.join("\n"),
                        duration,
                        errors: vec![format!("Process exited with code {}", code)],
                    })
                }
            }
            Ok(Err(e)) => Err(RuntimeError::execution_failed(e.to_string())),
            Err(_) => {
                // Timeout - kill the process
                let _ = child.kill().await;
                Err(RuntimeError::Timeout(timeout.as_secs()))
            }
        }
    }

    async fn install_dependencies(&self, config: &RuntimeConfig) -> Result<()> {
        let (executable, args): (&str, &[&str]) = match self.kind {
            RuntimeKind::Node => ("npm", &["install"]),
            RuntimeKind::Deno => return Ok(()), // Deno downloads on first run
            RuntimeKind::Bun => ("bun", &["install"]),
            RuntimeKind::Python => ("pip", &["install", "-r", "requirements.txt"]),
            RuntimeKind::Go => ("go", &["mod", "download"]),
            RuntimeKind::Rhai => return Ok(()), // No dependencies for Rhai
        };

        info!(runtime = %self.kind, "Installing dependencies");

        let status = Command::new(executable)
            .args(args)
            .current_dir(&config.working_dir)
            .status()
            .await?;

        if status.success() {
            Ok(())
        } else {
            Err(RuntimeError::execution_failed("Failed to install dependencies"))
        }
    }
}
