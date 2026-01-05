//! Configuration file loading

use crate::error::{ConfigError, Result};
use crate::interpolation::Interpolator;
use crate::schema::DevmerConfig;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Configuration file names to search for
const CONFIG_FILES: &[&str] = &["Devmer.toml", "devmer.toml"];

/// Configuration loader
pub struct ConfigLoader {
    /// Base directory to search from
    base_dir: PathBuf,

    /// Interpolator for environment variables
    interpolator: Interpolator,

    /// Whether to load .env files
    load_dotenv: bool,
}

impl ConfigLoader {
    /// Create a new config loader for the given directory
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            interpolator: Interpolator::new(),
            load_dotenv: true,
        }
    }

    /// Create a loader for the current directory
    pub fn current_dir() -> Result<Self> {
        let dir = std::env::current_dir()?;
        Ok(Self::new(dir))
    }

    /// Disable .env file loading
    pub fn without_dotenv(mut self) -> Self {
        self.load_dotenv = false;
        self
    }

    /// Set custom interpolator
    pub fn with_interpolator(mut self, interpolator: Interpolator) -> Self {
        self.interpolator = interpolator;
        self
    }

    /// Load configuration
    pub fn load(&self) -> Result<DevmerConfig> {
        // Load .env files first
        if self.load_dotenv {
            self.load_dotenv_files();
        }

        // Find config file
        let config_path = self.find_config_file()?;
        info!("Loading configuration from: {}", config_path.display());

        // Read and parse
        let content = std::fs::read_to_string(&config_path).map_err(|e| {
            ConfigError::FileReadError {
                path: config_path.display().to_string(),
                message: e.to_string(),
            }
        })?;

        // Interpolate environment variables
        let interpolated = self.interpolator.interpolate(&content)?;

        // Parse TOML
        let config: DevmerConfig = toml::from_str(&interpolated)?;

        debug!("Loaded configuration for project: {}", config.name);

        Ok(config)
    }

    /// Find the configuration file
    fn find_config_file(&self) -> Result<PathBuf> {
        // First, check the base directory
        for name in CONFIG_FILES {
            let path = self.base_dir.join(name);
            if path.exists() {
                return Ok(path);
            }
        }

        // Walk up the directory tree
        let mut current = self.base_dir.clone();
        while let Some(parent) = current.parent() {
            for name in CONFIG_FILES {
                let path = parent.join(name);
                if path.exists() {
                    return Ok(path);
                }
            }
            current = parent.to_path_buf();
        }

        Err(ConfigError::FileNotFound(
            "Devmer.toml not found in current directory or any parent".to_string(),
        ))
    }

    /// Load .env files
    fn load_dotenv_files(&self) {
        // Load in order of precedence (later overrides earlier)
        let env_files = [".env", ".env.local"];

        for name in env_files {
            let path = self.base_dir.join(name);
            if path.exists() {
                debug!("Loading environment from: {}", path.display());
                if let Err(e) = dotenvy::from_path(&path) {
                    debug!("Failed to load {}: {}", path.display(), e);
                }
            }
        }

        // Also check for stack-specific .env files
        // e.g., .env.dev, .env.prod
    }

    /// Load configuration from a specific path
    pub fn load_from(path: impl AsRef<Path>) -> Result<DevmerConfig> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(ConfigError::FileNotFound(path.display().to_string()));
        }

        let content = std::fs::read_to_string(path).map_err(|e| ConfigError::FileReadError {
            path: path.display().to_string(),
            message: e.to_string(),
        })?;

        let interpolator = Interpolator::new();
        let interpolated = interpolator.interpolate(&content)?;

        Ok(toml::from_str(&interpolated)?)
    }

    /// Get the project root directory (where config file is located)
    pub fn project_root(&self) -> Result<PathBuf> {
        let config_path = self.find_config_file()?;
        Ok(config_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.base_dir.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config(dir: &Path, content: &str) {
        std::fs::write(dir.join("Devmer.toml"), content).unwrap();
    }

    #[test]
    fn test_load_simple_config() {
        let temp = TempDir::new().unwrap();
        create_test_config(
            temp.path(),
            r#"
            name = "test-project"
            description = "A test project"

            [backend]
            type = "local"
        "#,
        );

        let loader = ConfigLoader::new(temp.path()).without_dotenv();
        let config = loader.load().unwrap();

        assert_eq!(config.name, "test-project");
        assert_eq!(config.description, Some("A test project".to_string()));
        assert_eq!(config.backend.backend_type, "local");
    }

    #[test]
    fn test_load_with_interpolation() {
        let temp = TempDir::new().unwrap();
        create_test_config(
            temp.path(),
            r#"
            name = "${PROJECT_NAME:-default-project}"

            [backend]
            type = "s3"
            bucket = "${DEVMER_BUCKET:-test-bucket}"
        "#,
        );

        let loader = ConfigLoader::new(temp.path()).without_dotenv();
        let config = loader.load().unwrap();

        assert_eq!(config.name, "default-project");
        assert_eq!(config.backend.bucket, Some("test-bucket".to_string()));
    }

    #[test]
    fn test_config_not_found() {
        let temp = TempDir::new().unwrap();
        let loader = ConfigLoader::new(temp.path());
        let result = loader.load();

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::FileNotFound(_)));
    }
}
