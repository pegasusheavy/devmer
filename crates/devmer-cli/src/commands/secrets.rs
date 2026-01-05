//! Secrets management commands

use anyhow::{Context, Result};
use colored::Colorize;
use devmer_config::ConfigLoader;
use dialoguer::Password;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;

use crate::commands::stack::get_current_stack;
use crate::output;

/// Secrets file path
const SECRETS_FILE: &str = ".devmer/secrets.json";

/// Encrypted secrets storage
#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
struct SecretsStore {
    /// Encryption provider used
    provider: String,
    /// Salt for key derivation (if passphrase-based)
    #[serde(default)]
    salt: Option<String>,
    /// Encrypted secrets by stack
    #[serde(default)]
    stacks: std::collections::HashMap<String, std::collections::HashMap<String, EncryptedSecret>>,
}

/// An encrypted secret
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct EncryptedSecret {
    /// Encrypted value (base64)
    ciphertext: String,
    /// Nonce (base64)
    nonce: String,
    /// Created timestamp
    created_at: String,
    /// Last rotated timestamp
    #[serde(default)]
    rotated_at: Option<String>,
}

impl SecretsStore {
    fn load() -> Self {
        let path = Path::new(SECRETS_FILE);
        if path.exists() {
            fs::read_to_string(path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Self {
                provider: "passphrase".to_string(),
                ..Default::default()
            }
        }
    }

    fn save(&self) -> Result<()> {
        let path = Path::new(SECRETS_FILE);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

/// Set a secret
pub async fn set(name: &str, value: Option<String>) -> Result<()> {
    // Load configuration
    let config = ConfigLoader::current_dir()
        .context("Failed to get current directory")?
        .load()
        .context("Failed to load Devmer.toml. Run 'devmer init' to create one.")?;

    let stack_name = get_current_stack(&config);

    // Get the secret value
    let secret_value = match value {
        Some(v) => v,
        None => {
            // Check if stdin has data (piped input)
            if atty::isnt(atty::Stream::Stdin) {
                let stdin = io::stdin();
                let mut line = String::new();
                stdin.lock().read_line(&mut line)?;
                line.trim().to_string()
            } else {
                // Interactive password prompt
                Password::new()
                    .with_prompt("Enter secret value")
                    .interact()?
            }
        }
    };

    if secret_value.is_empty() {
        anyhow::bail!("Secret value cannot be empty");
    }

    // Load secrets store
    let mut store = SecretsStore::load();

    // Get or create passphrase for encryption
    let passphrase = get_or_set_passphrase()?;

    // Encrypt the secret
    let encrypted = encrypt_secret(&secret_value, &passphrase)?;

    // Store the secret
    let stack_secrets = store.stacks.entry(stack_name.clone()).or_default();
    stack_secrets.insert(name.to_string(), encrypted);
    store.save()?;

    output::success(&format!("Set secret '{}' for stack '{}'", name, stack_name));

    Ok(())
}

/// Get a secret
pub async fn get(name: &str) -> Result<()> {
    // Load configuration
    let config = ConfigLoader::current_dir()
        .context("Failed to get current directory")?
        .load()
        .context("Failed to load Devmer.toml. Run 'devmer init' to create one.")?;

    let stack_name = get_current_stack(&config);

    // Load secrets store
    let store = SecretsStore::load();

    // Find the secret
    let encrypted = store
        .stacks
        .get(&stack_name)
        .and_then(|s| s.get(name))
        .ok_or_else(|| anyhow::anyhow!("Secret '{}' not found in stack '{}'", name, stack_name))?;

    // Get passphrase for decryption
    let passphrase = get_passphrase()?;

    // Decrypt the secret
    let decrypted = decrypt_secret(encrypted, &passphrase)?;

    // Output the value (no newline for piping)
    print!("{}", decrypted);
    io::stdout().flush()?;

    // Add newline only if terminal
    if atty::is(atty::Stream::Stdout) {
        println!();
    }

    Ok(())
}

/// List secrets
pub async fn list() -> Result<()> {
    // Load configuration
    let config = ConfigLoader::current_dir()
        .context("Failed to get current directory")?
        .load()
        .context("Failed to load Devmer.toml. Run 'devmer init' to create one.")?;

    let stack_name = get_current_stack(&config);

    output::banner(&format!("Secrets for stack: {}", stack_name));

    // Load secrets store
    let store = SecretsStore::load();

    let secrets = store.stacks.get(&stack_name);

    if let Some(secrets) = secrets {
        if secrets.is_empty() {
            output::info("No secrets configured.");
        } else {
            println!("{:<30}  {}", "Name".bold(), "Created".bold());
            println!("{}", "─".repeat(60));

            for (name, secret) in secrets {
                let created = if secret.created_at.len() >= 19 {
                    &secret.created_at[..19]
                } else {
                    &secret.created_at
                };
                println!("  {:<28}  {}", name.cyan(), created.dimmed());
            }

            println!();
            output::info(&format!("{} secret(s) total", secrets.len()));
        }
    } else {
        output::info("No secrets configured.");
    }

    println!();
    output::info("Use 'devmer secrets set <name>' to add a secret");
    output::info("Use 'devmer secrets get <name>' to retrieve a secret");

    Ok(())
}

/// Rotate secrets encryption
pub async fn rotate() -> Result<()> {
    output::info("Rotating secrets encryption...");

    // Get current passphrase
    let current_passphrase = get_passphrase()?;

    // Load secrets store
    let mut store = SecretsStore::load();

    // Get new passphrase
    let new_passphrase = Password::new()
        .with_prompt("Enter new passphrase")
        .with_confirmation("Confirm new passphrase", "Passphrases do not match")
        .interact()?;

    if new_passphrase.is_empty() {
        anyhow::bail!("Passphrase cannot be empty");
    }

    // Re-encrypt all secrets
    let mut rotated_count = 0;
    let now = chrono::Utc::now().to_rfc3339();

    for (_stack_name, secrets) in &mut store.stacks {
        for (name, encrypted) in secrets.iter_mut() {
            // Decrypt with old passphrase
            match decrypt_secret(encrypted, &current_passphrase) {
                Ok(plaintext) => {
                    // Re-encrypt with new passphrase
                    match encrypt_secret(&plaintext, &new_passphrase) {
                        Ok(new_encrypted) => {
                            encrypted.ciphertext = new_encrypted.ciphertext;
                            encrypted.nonce = new_encrypted.nonce;
                            encrypted.rotated_at = Some(now.clone());
                            rotated_count += 1;
                        }
                        Err(e) => {
                            output::error(&format!("Failed to re-encrypt '{}': {}", name, e));
                        }
                    }
                }
                Err(e) => {
                    output::error(&format!("Failed to decrypt '{}': {}", name, e));
                }
            }
        }
    }

    // Save updated store
    store.save()?;

    // Update stored passphrase
    set_passphrase(&new_passphrase)?;

    output::success(&format!(
        "Rotated {} secret(s). New passphrase is now active.",
        rotated_count
    ));

    Ok(())
}

// ============================================================================
// Encryption helpers (simplified - in production use devmer-secrets crate)
// ============================================================================

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

/// Simple XOR-based encryption (for demonstration)
/// In production, use proper encryption from devmer-secrets
fn encrypt_secret(plaintext: &str, passphrase: &str) -> Result<EncryptedSecret> {
    // Generate a random nonce
    let nonce: [u8; 12] = rand::random();

    // Derive a key from passphrase (simplified - use PBKDF2 in production)
    let key = derive_key(passphrase, &nonce);

    // XOR encrypt (simplified - use AES-GCM in production)
    let plaintext_bytes = plaintext.as_bytes();
    let mut ciphertext = vec![0u8; plaintext_bytes.len()];
    for (i, byte) in plaintext_bytes.iter().enumerate() {
        ciphertext[i] = byte ^ key[i % key.len()];
    }

    Ok(EncryptedSecret {
        ciphertext: BASE64.encode(&ciphertext),
        nonce: BASE64.encode(&nonce),
        created_at: chrono::Utc::now().to_rfc3339(),
        rotated_at: None,
    })
}

/// Decrypt a secret
fn decrypt_secret(encrypted: &EncryptedSecret, passphrase: &str) -> Result<String> {
    let ciphertext = BASE64
        .decode(&encrypted.ciphertext)
        .context("Invalid ciphertext")?;
    let nonce = BASE64.decode(&encrypted.nonce).context("Invalid nonce")?;

    // Derive key from passphrase
    let key = derive_key(passphrase, &nonce);

    // XOR decrypt
    let mut plaintext = vec![0u8; ciphertext.len()];
    for (i, byte) in ciphertext.iter().enumerate() {
        plaintext[i] = byte ^ key[i % key.len()];
    }

    String::from_utf8(plaintext).context("Invalid UTF-8 in decrypted secret")
}

/// Simple key derivation (use PBKDF2/Argon2 in production)
fn derive_key(passphrase: &str, salt: &[u8]) -> Vec<u8> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    passphrase.hash(&mut hasher);
    salt.hash(&mut hasher);
    let hash = hasher.finish();

    // Extend to 32 bytes
    let mut key = Vec::with_capacity(32);
    for i in 0..4 {
        let mut h = DefaultHasher::new();
        hash.hash(&mut h);
        i.hash(&mut h);
        key.extend_from_slice(&h.finish().to_le_bytes());
    }
    key
}

/// Passphrase cache file
const PASSPHRASE_CACHE: &str = ".devmer/.passphrase";

/// Get or set passphrase
fn get_or_set_passphrase() -> Result<String> {
    // Try to get existing passphrase
    if let Ok(passphrase) = get_passphrase() {
        return Ok(passphrase);
    }

    // Prompt for new passphrase
    output::info("No passphrase set. Please create one for encrypting secrets.");
    let passphrase = Password::new()
        .with_prompt("Enter passphrase")
        .with_confirmation("Confirm passphrase", "Passphrases do not match")
        .interact()?;

    if passphrase.is_empty() {
        anyhow::bail!("Passphrase cannot be empty");
    }

    set_passphrase(&passphrase)?;
    Ok(passphrase)
}

/// Get passphrase from environment or cache
fn get_passphrase() -> Result<String> {
    // Check environment variable first
    if let Ok(passphrase) = std::env::var("DEVMER_PASSPHRASE") {
        return Ok(passphrase);
    }

    // Check cache file
    let path = Path::new(PASSPHRASE_CACHE);
    if path.exists() {
        let content = fs::read_to_string(path)?;
        if !content.is_empty() {
            return Ok(content.trim().to_string());
        }
    }

    // Prompt for passphrase
    let passphrase = Password::new()
        .with_prompt("Enter passphrase")
        .interact()?;

    Ok(passphrase)
}

/// Save passphrase to cache
fn set_passphrase(passphrase: &str) -> Result<()> {
    let path = Path::new(PASSPHRASE_CACHE);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Set restrictive permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(passphrase.as_bytes())?;
    }

    #[cfg(not(unix))]
    fs::write(path, passphrase)?;

    Ok(())
}
