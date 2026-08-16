//! Config encryption — AES-256-GCM protection for strategy parameters.
//! Set `RTP_CONFIG_KEY` (64 hex chars) for encryption; plaintext otherwise.

use aes_gcm::{
    AeadCore, Aes256Gcm, Nonce,
    aead::{Aead, Generate, KeyInit},
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Environment variable name for the encryption key.
const CONFIG_KEY_ENV: &str = "RTP_CONFIG_KEY";

/// Default directory for encrypted configs (relative to repo root).
const CONFIGS_DIR: &str = "configs";

/// Errors from config encryption/decryption operations.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Key error: {0}")]
    KeyError(String),
    #[error("Encryption failed: {0}")]
    EncryptionError(String),
    #[error("Decryption failed: {0}")]
    DecryptionError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Config not found: {0}")]
    NotFound(String),
}

/// A config entry — encrypted bytes with associated metadata.
#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptedConfig {
    /// Hex-encoded ciphertext (AES-256-GCM encrypted).
    pub ciphertext_hex: String,
    /// Hex-encoded 96-bit nonce.
    pub nonce_hex: String,
    /// Config name for identification.
    pub name: String,
    /// ISO 8601 timestamp of when this config was encrypted.
    pub encrypted_at: String,
}

/// Plaintext config wrapper with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigEntry {
    /// Config name.
    pub name: String,
    /// Config value as JSON.
    pub value: serde_json::Value,
    /// ISO 8601 timestamp.
    pub created_at: String,
}

/// Config encryption manager.
///
/// Loads the AES-256 key from `RTP_CONFIG_KEY` env var (hex-encoded).
/// Falls back to plaintext mode if the env var is not set (development).
pub struct ConfigEncryption {
    /// Cipher instance (None in plaintext mode).
    cipher: Option<Aes256Gcm>,
    /// Directory where configs are stored.
    configs_dir: PathBuf,
}

impl std::fmt::Debug for ConfigEncryption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigEncryption")
            .field("encrypted", &self.is_encrypted())
            .field("configs_dir", &self.configs_dir)
            .finish()
    }
}

impl ConfigEncryption {
    /// Create a new ConfigEncryption instance.
    ///
    /// If `RTP_CONFIG_KEY` is set, enables AES-256-GCM encryption.
    /// Otherwise, operates in plaintext mode (for development).
    pub fn new() -> Result<Self, ConfigError> {
        let cipher = Self::load_cipher()?;
        let configs_dir = Self::default_configs_dir();
        Ok(Self {
            cipher,
            configs_dir,
        })
    }

    /// Create with a custom configs directory (for testing).
    pub fn with_dir(dir: PathBuf) -> Result<Self, ConfigError> {
        let cipher = Self::load_cipher()?;
        Ok(Self {
            cipher,
            configs_dir: dir,
        })
    }

    /// Create with a custom configs directory and an explicit key (for testing).
    /// Pass `None` for plaintext mode, `Some(key_bytes)` for encrypted mode.
    pub fn with_dir_and_key(dir: PathBuf, key: Option<&[u8; 32]>) -> Self {
        let cipher = key.map(|k| {
            let key: aes_gcm::Key<Aes256Gcm> = (*k).into();
            Aes256Gcm::new(&key)
        });
        Self {
            cipher,
            configs_dir: dir,
        }
    }

    /// Whether encryption is enabled.
    pub fn is_encrypted(&self) -> bool {
        self.cipher.is_some()
    }

    /// Save a config entry (encrypted if key is set, plaintext otherwise).
    pub fn save_config(&self, name: &str, value: &serde_json::Value) -> Result<(), ConfigError> {
        std::fs::create_dir_all(&self.configs_dir)?;

        let entry = ConfigEntry {
            name: name.to_string(),
            value: value.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let json_bytes = serde_json::to_vec(&entry)
            .map_err(|e| ConfigError::EncryptionError(format!("Serialize error: {}", e)))?;

        if let Some(ref cipher) = self.cipher {
            let nonce = Nonce::<<Aes256Gcm as AeadCore>::NonceSize>::generate();
            let ciphertext = cipher
                .encrypt(&nonce, json_bytes.as_ref())
                .map_err(|e| ConfigError::EncryptionError(format!("AES-GCM encrypt: {}", e)))?;

            let encrypted = EncryptedConfig {
                ciphertext_hex: hex::encode(&ciphertext),
                nonce_hex: hex::encode(nonce),
                name: name.to_string(),
                encrypted_at: chrono::Utc::now().to_rfc3339(),
            };

            let path = self.config_path(name);
            let file = std::fs::File::create(&path)?;
            serde_json::to_writer_pretty(file, &encrypted)
                .map_err(|e| ConfigError::IoError(std::io::Error::other(e)))?;
        } else {
            // Plaintext mode.
            let path = self.config_path(name);
            let file = std::fs::File::create(&path)?;
            serde_json::to_writer_pretty(file, &entry)
                .map_err(|e| ConfigError::IoError(std::io::Error::other(e)))?;
        }

        tracing::info!(name, encrypted = self.is_encrypted(), "Config saved");
        Ok(())
    }

    /// Load a config entry (decrypts if encrypted, reads plaintext otherwise).
    pub fn load_config(&self, name: &str) -> Result<ConfigEntry, ConfigError> {
        let path = self.config_path(name);
        if !path.exists() {
            return Err(ConfigError::NotFound(name.to_string()));
        }

        let file_bytes = std::fs::read(&path)?;

        // Try to parse as encrypted config first.
        if let Ok(encrypted) = serde_json::from_slice::<EncryptedConfig>(&file_bytes)
            && !encrypted.ciphertext_hex.is_empty()
        {
            return self.decrypt_config(&encrypted);
        }

        // Fall back to plaintext parsing.
        let entry: ConfigEntry = serde_json::from_slice(&file_bytes)
            .map_err(|e| ConfigError::DecryptionError(format!("Parse error: {}", e)))?;
        Ok(entry)
    }

    /// List all saved config names.
    pub fn list_configs(&self) -> Vec<String> {
        if !self.configs_dir.exists() {
            return vec![];
        }

        std::fs::read_dir(&self.configs_dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter_map(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        if name.ends_with(".json") {
                            Some(name.replace(".json", ""))
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Delete a config entry.
    pub fn delete_config(&self, name: &str) -> Result<(), ConfigError> {
        let path = self.config_path(name);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    // Private helpers

    fn config_path(&self, name: &str) -> PathBuf {
        self.configs_dir.join(format!("{}.json", name))
    }

    fn default_configs_dir() -> PathBuf {
        // Try to find repo root from CARGO_MANIFEST_DIR.
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(manifest)
            .parent()
            .unwrap_or(Path::new("."))
            .parent()
            .unwrap_or(Path::new("."))
            .join(CONFIGS_DIR)
    }

    fn load_cipher() -> Result<Option<Aes256Gcm>, ConfigError> {
        let key_hex = match std::env::var(CONFIG_KEY_ENV) {
            Ok(k) => k,
            Err(_) => {
                tracing::warn!(
                    "{} not set — running in plaintext mode (development only)",
                    CONFIG_KEY_ENV
                );
                return Ok(None);
            }
        };

        let key_bytes = hex::decode(&key_hex)
            .map_err(|e| ConfigError::KeyError(format!("Invalid hex key: {}", e)))?;

        if key_bytes.len() != 32 {
            return Err(ConfigError::KeyError(format!(
                "Key must be 256 bits (32 bytes), got {} bytes",
                key_bytes.len()
            )));
        }

        let key: aes_gcm::Key<Aes256Gcm> =
            aes_gcm::Key::<Aes256Gcm>::try_from(key_bytes.as_slice())
                .map_err(|e| ConfigError::KeyError(format!("Invalid key length: {}", e)))?;
        Ok(Some(Aes256Gcm::new(&key)))
    }

    fn decrypt_config(&self, encrypted: &EncryptedConfig) -> Result<ConfigEntry, ConfigError> {
        let cipher = self
            .cipher
            .as_ref()
            .ok_or_else(|| ConfigError::DecryptionError("No encryption key loaded".to_string()))?;

        let ciphertext = hex::decode(&encrypted.ciphertext_hex)
            .map_err(|e| ConfigError::DecryptionError(format!("Invalid ciphertext hex: {}", e)))?;

        let nonce_bytes = hex::decode(&encrypted.nonce_hex)
            .map_err(|e| ConfigError::DecryptionError(format!("Invalid nonce hex: {}", e)))?;

        let nonce: Nonce<<Aes256Gcm as AeadCore>::NonceSize> =
            Nonce::<<Aes256Gcm as AeadCore>::NonceSize>::try_from(nonce_bytes.as_slice()).map_err(
                |e| ConfigError::DecryptionError(format!("Invalid nonce length: {}", e)),
            )?;

        let plaintext = cipher
            .decrypt(&nonce, ciphertext.as_ref())
            .map_err(|e| ConfigError::DecryptionError(format!("AES-GCM decrypt: {}", e)))?;

        let entry: ConfigEntry = serde_json::from_slice(&plaintext)
            .map_err(|e| ConfigError::DecryptionError(format!("Deserialize error: {}", e)))?;
        Ok(entry)
    }
}

impl Default for ConfigEncryption {
    fn default() -> Self {
        Self::new().expect("Failed to initialize ConfigEncryption")
    }
}

/// Helper: safely remove env var (unsafe in Rust 2024 edition).
#[cfg(test)]
fn test_remove_var(key: &str) {
    unsafe { std::env::remove_var(key) };
}

/// Helper: safely set env var (unsafe in Rust 2024 edition).
#[cfg(test)]
fn test_set_var(key: &str, val: &str) {
    unsafe { std::env::set_var(key, val) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rtp_config_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn make_key() -> [u8; 32] {
        // Generate a random 256-bit key for testing.
        // Use three 12-byte nonces to fill 32 bytes.
        let n1 = Nonce::<<Aes256Gcm as AeadCore>::NonceSize>::generate();
        let n2 = Nonce::<<Aes256Gcm as AeadCore>::NonceSize>::generate();
        let n3 = Nonce::<<Aes256Gcm as AeadCore>::NonceSize>::generate();
        let mut key_bytes = [0u8; 32];
        key_bytes[..12].copy_from_slice(&n1);
        key_bytes[12..24].copy_from_slice(&n2);
        key_bytes[24..32].copy_from_slice(&n3[..8]);
        key_bytes
    }

    #[test]
    fn plaintext_mode_when_no_key() {
        let dir = temp_dir();
        let enc = ConfigEncryption::with_dir_and_key(dir, None);
        assert!(!enc.is_encrypted());
    }

    #[test]
    fn save_and_load_plaintext() {
        let dir = temp_dir();
        let enc = ConfigEncryption::with_dir_and_key(dir.clone(), None);

        let value = serde_json::json!({"rsi_entry": 28, "stop_loss": 0.03});
        enc.save_config("test_strategy", &value).unwrap();

        let loaded = enc.load_config("test_strategy").unwrap();
        assert_eq!(loaded.name, "test_strategy");
        assert_eq!(loaded.value["rsi_entry"], 28);
        assert_eq!(loaded.value["stop_loss"], 0.03);
    }

    #[test]
    fn save_and_load_encrypted() {
        let key = make_key();
        let dir = temp_dir();
        let enc = ConfigEncryption::with_dir_and_key(dir.clone(), Some(&key));

        assert!(enc.is_encrypted());

        let value = serde_json::json!({
            "strategy": "mr_rsi_bb",
            "rsi_entry": 28,
            "stop_loss": 0.03,
            "api_key": "secret_key_12345"
        });
        enc.save_config("encrypted_strategy", &value).unwrap();

        // Verify the file contains hex-encoded data (not plaintext).
        let raw = fs::read_to_string(dir.join("encrypted_strategy.json")).unwrap();
        assert!(raw.contains("ciphertext_hex"), "Expected encrypted output");
        assert!(
            !raw.contains("secret_key_12345"),
            "Secret should not be in plaintext"
        );

        // Load and verify (same key).
        let enc2 = ConfigEncryption::with_dir_and_key(dir.clone(), Some(&key));
        let loaded = enc2.load_config("encrypted_strategy").unwrap();
        assert_eq!(loaded.name, "encrypted_strategy");
        assert_eq!(loaded.value["strategy"], "mr_rsi_bb");
        assert_eq!(loaded.value["api_key"], "secret_key_12345");
    }

    #[test]
    fn load_encrypted_with_wrong_key_fails() {
        let key = make_key();
        let dir = temp_dir();
        let enc = ConfigEncryption::with_dir_and_key(dir.clone(), Some(&key));

        enc.save_config("wrong_key_test", &serde_json::json!({"x": 1}))
            .unwrap();

        // Try to load with a different key.
        let wrong_key = make_key();
        let enc2 = ConfigEncryption::with_dir_and_key(dir, Some(&wrong_key));
        let result = enc2.load_config("wrong_key_test");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("decrypt"));
    }

    #[test]
    fn load_nonexistent_config_returns_not_found() {
        let dir = temp_dir();
        let enc = ConfigEncryption::with_dir_and_key(dir, None);
        let result = enc.load_config("does_not_exist");
        assert!(matches!(result, Err(ConfigError::NotFound(_))));
    }

    #[test]
    fn list_configs_works() {
        let dir = temp_dir();
        let enc = ConfigEncryption::with_dir_and_key(dir.clone(), None);

        enc.save_config("list_a", &serde_json::json!({"a": 1}))
            .unwrap();
        enc.save_config("list_b", &serde_json::json!({"b": 2}))
            .unwrap();

        let mut names = enc.list_configs();
        names.sort();
        assert_eq!(names, vec!["list_a", "list_b"]);
    }

    #[test]
    fn delete_config_works() {
        let dir = temp_dir();
        let enc = ConfigEncryption::with_dir_and_key(dir.clone(), None);

        enc.save_config("to_delete", &serde_json::json!({"x": 1}))
            .unwrap();
        assert!(dir.join("to_delete.json").exists());

        enc.delete_config("to_delete").unwrap();
        assert!(!dir.join("to_delete.json").exists());

        // Delete non-existent is ok.
        enc.delete_config("nonexistent").unwrap();
    }

    #[test]
    fn invalid_key_hex_rejected() {
        test_set_var(CONFIG_KEY_ENV, "not-valid-hex");
        let dir = temp_dir();
        let result = ConfigEncryption::with_dir(dir);
        assert!(result.is_err());
        test_remove_var(CONFIG_KEY_ENV);
    }

    #[test]
    fn wrong_key_length_rejected() {
        // 16 bytes = 128 bits, not 256.
        let short_key = hex::encode([0u8; 16]);
        test_set_var(CONFIG_KEY_ENV, &short_key);
        let dir = temp_dir();
        let result = ConfigEncryption::with_dir(dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("32 bytes"));
        test_remove_var(CONFIG_KEY_ENV);
    }

    #[test]
    fn encrypted_config_roundtrip_complex_value() {
        let key = make_key();
        let dir = temp_dir();
        let enc = ConfigEncryption::with_dir_and_key(dir.clone(), Some(&key));

        let complex = serde_json::json!({
            "strategy": "multi_tf",
            "params": {
                "rsi_entry": 28,
                "rsi_exit": 72,
                "bb_period": 20,
                "stop_loss": 0.03,
                "hold_hours": 48,
            },
            "metadata": {
                "folds_validated": 9,
                "consistency": 0.78,
                "optimization_date": "2026-04-08"
            }
        });
        enc.save_config("complex", &complex).unwrap();

        let enc2 = ConfigEncryption::with_dir_and_key(dir, Some(&key));
        let loaded = enc2.load_config("complex").unwrap();
        assert_eq!(loaded.value["params"]["rsi_entry"], 28);
        assert_eq!(loaded.value["metadata"]["folds_validated"], 9);
    }
}
