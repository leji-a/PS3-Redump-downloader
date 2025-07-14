use crate::{config::Config, key_manager::KeyManager};
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use tokio::process::Command;

/// Decryptor handles PS3 ISO decryption using the C binary and keys.
pub struct Decryptor {
    config: Config,
    key_manager: KeyManager,
}

impl Decryptor {
    /// Create a new Decryptor with the given configuration.
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
            key_manager: KeyManager::new(config),
        }
    }

    /// Decrypts a PS3 ISO file using the C decryption binary and key.
    pub async fn decrypt_iso(&self, encrypted_path: &Path, decrypted_path: &Path, key: &str) -> Result<()> {
        let decryptor_path = self.config.decryptor_path();
        
        // Check if decryption binary exists
        if !decryptor_path.exists() {
            anyhow::bail!(
                "PS3 decryption binary not found at: {}. Please ensure the C decryption binary is compiled and available.",
                decryptor_path.display()
            );
        }

        println!("Decrypting PS3 ISO file with key...");

        // Create progress bar for decryption
        let progress_bar = ProgressBar::new_spinner();
        progress_bar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} Decrypting PS3 ISO... {elapsed_precise}")
                .unwrap()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
        );

        // Build command for decryption with key
        let mut command = Command::new(&decryptor_path);
        command.arg(encrypted_path);
        command.arg(decrypted_path);
        command.arg(key); // Add the key as a third argument

        // Set timeout for decryption process
        let timeout_duration = std::time::Duration::from_secs(self.config.decryption_timeout);

        // Execute decryption with timeout
        let result = tokio::time::timeout(timeout_duration, async {
            let output = command.output().await?;
            
            if output.status.success() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("Decryption failed: {}", stderr);
            }
        }).await;

        progress_bar.finish_with_message("Decryption completed");

        match result {
            Ok(Ok(())) => {
                println!("PS3 ISO decryption completed successfully");
                Ok(())
            }
            Ok(Err(e)) => {
                anyhow::bail!("Decryption error: {}", e);
            }
            Err(_) => {
                anyhow::bail!("Decryption timed out after {} seconds", self.config.decryption_timeout);
            }
        }
    }

    /// Validates that the decryption binary is available and executable.
    pub fn validate_decryptor(&self) -> Result<()> {
        let decryptor_path = self.config.decryptor_path();
        
        if !decryptor_path.exists() {
            anyhow::bail!(
                "PS3 decryption binary not found at: {}. Please compile the C decryption binary first.",
                decryptor_path.display()
            );
        }

        // Check if file is executable (Unix-like systems)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(&decryptor_path) {
                if metadata.permissions().mode() & 0o111 == 0 {
                    anyhow::bail!(
                        "PS3 decryption binary is not executable: {}. Please make it executable with 'chmod +x {}'",
                        decryptor_path.display(),
                        decryptor_path.display()
                    );
                }
            }
        }

        Ok(())
    }

    /// Gets the key manager for accessing keys.
    pub fn key_manager(&self) -> &KeyManager {
        &self.key_manager
    }

    /// Gets the size of the encrypted file for progress estimation.
    fn get_file_size(&self, path: &Path) -> Result<u64> {
        let metadata = std::fs::metadata(path)?;
        Ok(metadata.len())
    }
} 