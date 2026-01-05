//! Environment variable interpolation

use crate::error::{ConfigError, Result};
use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::sync::LazyLock;

/// Regex for ${VAR} syntax
static ENV_VAR_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$\{([^}:]+)(?::-([^}]*))?\}").unwrap());

/// Regex for ${VAR:-default} syntax (already captured above)
/// Regex for ${file:/path} syntax
static FILE_REF_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$\{file:([^}]+)\}").unwrap());

/// Regex for ${secret:name} syntax
static SECRET_REF_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$\{secret:([^}]+)\}").unwrap());

/// Fast check for interpolation markers - avoids regex when possible
#[inline]
fn contains_interpolation_marker(s: &str) -> bool {
    s.contains("${")
}

/// Environment variable interpolator
pub struct Interpolator {
    /// Additional variables to use (overrides env)
    extra_vars: HashMap<String, String>,

    /// Whether to error on missing variables
    strict: bool,
}

impl Interpolator {
    /// Create a new interpolator
    pub fn new() -> Self {
        Self {
            extra_vars: HashMap::new(),
            strict: true,
        }
    }

    /// Set strict mode (error on missing variables)
    pub fn strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }

    /// Add extra variables
    pub fn with_vars(mut self, vars: HashMap<String, String>) -> Self {
        self.extra_vars.extend(vars);
        self
    }

    /// Add a single variable
    pub fn with_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra_vars.insert(key.into(), value.into());
        self
    }

    /// Interpolate a string value
    pub fn interpolate(&self, input: &str) -> Result<String> {
        // Fast path: no interpolation markers at all
        if !contains_interpolation_marker(input) {
            return Ok(input.to_string());
        }

        let mut result = input.to_string();

        // First, handle file references
        result = self.interpolate_file_refs(&result)?;

        // Then handle environment variables
        result = self.interpolate_env_vars(&result)?;

        Ok(result)
    }

    /// Interpolate environment variable references using efficient single-pass replacement
    fn interpolate_env_vars(&self, input: &str) -> Result<String> {
        // Fast path: no env var markers
        if !contains_interpolation_marker(input) {
            return Ok(input.to_string());
        }

        // Process line by line to skip comments
        let mut result = String::with_capacity(input.len() + 64);
        let mut first_line = true;

        for line in input.lines() {
            if !first_line {
                result.push('\n');
            }
            first_line = false;

            let trimmed = line.trim_start();

            // Skip interpolation on comment lines
            if trimmed.starts_with('#') {
                result.push_str(line);
                continue;
            }

            // Fast path: no interpolation in this line
            if !contains_interpolation_marker(line) {
                result.push_str(line);
                continue;
            }

            // For non-comment lines, split at first # to preserve inline comments
            let (code_part, comment_part) = match line.find('#') {
                Some(pos) => (&line[..pos], Some(&line[pos..])),
                None => (line, None),
            };

            // Interpolate the code part using efficient replacement
            let interpolated = self.replace_env_vars(code_part)?;

            result.push_str(&interpolated);
            if let Some(comment) = comment_part {
                result.push_str(comment);
            }
        }

        Ok(result)
    }

    /// Replace environment variables in a string segment (no comments)
    /// Uses iterative replacement to handle nested variables
    fn replace_env_vars(&self, input: &str) -> Result<String> {
        let mut current = input.to_string();
        let max_iterations = 10; // Prevent infinite loops from circular references

        for _ in 0..max_iterations {
            // Fast path check
            if !contains_interpolation_marker(&current) {
                break;
            }

            let mut new_result = String::with_capacity(current.len() + 32);
            let mut last_end = 0;
            let mut any_replacement = false;

            for cap in ENV_VAR_REGEX.captures_iter(&current) {
                let m = cap.get(0).unwrap();
                let var_name = cap.get(1).unwrap().as_str();
                let default_value = cap.get(2).map(|m| m.as_str());

                // Copy text before this match
                new_result.push_str(&current[last_end..m.start()]);

                // Get replacement value
                let value = self.get_var(var_name).or_else(|| default_value.map(String::from));

                match value {
                    Some(v) => {
                        new_result.push_str(&v);
                        any_replacement = true;
                    }
                    None if self.strict => {
                        return Err(ConfigError::EnvVarNotFound(var_name.to_string()));
                    }
                    None => {
                        // Leave as-is in non-strict mode
                        new_result.push_str(m.as_str());
                    }
                }

                last_end = m.end();
            }

            // Copy remaining text
            new_result.push_str(&current[last_end..]);

            if !any_replacement {
                return Ok(new_result);
            }

            current = new_result;
        }

        Ok(current)
    }

    /// Interpolate file references
    fn interpolate_file_refs(&self, input: &str) -> Result<String> {
        let mut result = input.to_string();

        for cap in FILE_REF_REGEX.captures_iter(input) {
            let full_match = cap.get(0).unwrap().as_str();
            let file_path = cap.get(1).unwrap().as_str();

            // Expand any env vars in the path first
            let expanded_path = shellexpand::env(file_path)
                .map_err(|e| ConfigError::interpolation_error(file_path, e.to_string()))?;

            let content = std::fs::read_to_string(expanded_path.as_ref()).map_err(|e| {
                ConfigError::FileReadError {
                    path: file_path.to_string(),
                    message: e.to_string(),
                }
            })?;

            result = result.replace(full_match, content.trim());
        }

        Ok(result)
    }

    /// Get a variable value (extra vars take precedence over env)
    fn get_var(&self, name: &str) -> Option<String> {
        self.extra_vars
            .get(name)
            .cloned()
            .or_else(|| env::var(name).ok())
    }

    /// Check if a string contains secret references
    #[inline]
    pub fn has_secret_refs(input: &str) -> bool {
        // Fast path: check for marker before running regex
        input.contains("${secret:") && SECRET_REF_REGEX.is_match(input)
    }

    /// Extract secret reference names from a string
    pub fn extract_secret_refs(input: &str) -> Vec<String> {
        // Fast path: no secret markers
        if !input.contains("${secret:") {
            return Vec::new();
        }
        SECRET_REF_REGEX
            .captures_iter(input)
            .map(|cap| cap.get(1).unwrap().as_str().to_string())
            .collect()
    }

    /// Check if a string contains any interpolation syntax
    #[inline]
    pub fn needs_interpolation(input: &str) -> bool {
        // Fast path: check for marker before running regex
        if !contains_interpolation_marker(input) {
            return false;
        }
        ENV_VAR_REGEX.is_match(input)
            || FILE_REF_REGEX.is_match(input)
            || SECRET_REF_REGEX.is_match(input)
    }
}

impl Default for Interpolator {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to interpolate a string with default settings
pub fn interpolate(input: &str) -> Result<String> {
    Interpolator::new().interpolate(input)
}

/// Convenience function to interpolate a string in non-strict mode
pub fn interpolate_optional(input: &str) -> Result<String> {
    Interpolator::new().strict(false).interpolate(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_interpolation() {
        let interp = Interpolator::new().with_var("MY_VAR", "hello");
        let result = interp.interpolate("value: ${MY_VAR}").unwrap();
        assert_eq!(result, "value: hello");
    }

    #[test]
    fn test_default_value() {
        let interp = Interpolator::new();
        let result = interp
            .interpolate("value: ${NONEXISTENT:-default_value}")
            .unwrap();
        assert_eq!(result, "value: default_value");
    }

    #[test]
    fn test_missing_var_strict() {
        let interp = Interpolator::new().strict(true);
        let result = interp.interpolate("value: ${NONEXISTENT_VAR_12345}");
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_var_non_strict() {
        let interp = Interpolator::new().strict(false);
        let result = interp
            .interpolate("value: ${NONEXISTENT_VAR_12345}")
            .unwrap();
        assert_eq!(result, "value: ${NONEXISTENT_VAR_12345}");
    }

    #[test]
    fn test_multiple_vars() {
        let interp = Interpolator::new()
            .with_var("VAR1", "one")
            .with_var("VAR2", "two");
        let result = interp.interpolate("${VAR1} and ${VAR2}").unwrap();
        assert_eq!(result, "one and two");
    }

    #[test]
    fn test_secret_ref_detection() {
        assert!(Interpolator::has_secret_refs("password: ${secret:db_password}"));
        assert!(!Interpolator::has_secret_refs("password: ${DB_PASSWORD}"));
    }

    #[test]
    fn test_extract_secret_refs() {
        let refs = Interpolator::extract_secret_refs(
            "db: ${secret:db_password}, api: ${secret:api_key}",
        );
        assert_eq!(refs, vec!["db_password", "api_key"]);
    }

    #[test]
    fn test_needs_interpolation() {
        assert!(Interpolator::needs_interpolation("${VAR}"));
        assert!(Interpolator::needs_interpolation("${file:/path}"));
        assert!(Interpolator::needs_interpolation("${secret:name}"));
        assert!(!Interpolator::needs_interpolation("plain text"));
    }
}
