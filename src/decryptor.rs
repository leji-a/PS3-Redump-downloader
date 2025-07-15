use crate::{config::Config, key_manager::KeyManager};
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use tokio::process::Command;
use std::io::Write;

/// Decryptor handles PS3 ISO decryption using the PS3Dec C binary and keys.
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

    /// Decrypts a PS3 ISO file using the PS3Dec C binary and key.
    pub async fn decrypt_iso(&self, encrypted_path: &Path, decrypted_path: &Path, key: &str) -> Result<()> {
        use std::fs;
        use std::time::Duration;
        use tokio::time::sleep;
        use indicatif::ProgressDrawTarget;

        let decryptor_path = self.config.decryptor_path();
        
        // Check if decryption binary exists
        if !decryptor_path.exists() {
            anyhow::bail!(
                "PS3Dec binary not found at: {}. Please build the PS3Dec program first:\n\
                 cd decryptor/PS3Dec && mkdir -p build && cd build && cmake .. && make",
                decryptor_path.display()
            );
        }

        let input_size = fs::metadata(encrypted_path)
            .map(|m| m.len())
            .unwrap_or(0);
        if input_size == 0 {
            anyhow::bail!("Encrypted ISO file is empty or missing: {}", encrypted_path.display());
        }

        println!("Decrypting PS3 ISO file with key...");
        std::io::stdout().flush().ok();
        // Create progress bar for decryption
        let progress_bar = ProgressBar::new(input_size);
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-")
        );
        progress_bar.set_draw_target(ProgressDrawTarget::stdout());
        progress_bar.tick();
        std::io::stdout().flush().ok();

        // Build command for PS3Dec: PS3Dec d key <key> <input> <output>
        let mut command = Command::new(&decryptor_path);
        command.arg("d");           // decrypt mode
        command.arg("key");         // key type (direct key)
        command.arg(key);            // 32-character hex key
        command.arg(encrypted_path); // input file
        command.arg(decrypted_path); // output file

        // Start the decryption process
        let mut child = command.spawn()?;
        let timeout_duration = Duration::from_secs(self.config.decryption_timeout);
        let poll_interval = Duration::from_millis(500);
        let mut last_size = 0;
        let mut stalled_count = 0;
        let max_stalled = 20; // 10 seconds
        let start_time = std::time::Instant::now();

        let mut used_spinner = false;
        // Progress bar loop
        loop {
            // Check if process has exited
            match child.try_wait()? {
                Some(status) => {
                    // Final update
                    if decrypted_path.exists() {
                        let final_size = fs::metadata(decrypted_path).map(|m| m.len()).unwrap_or(0);
                        progress_bar.set_position(final_size.min(input_size));
                    }
                    if status.success() {
                        progress_bar.finish_with_message("Decryption completed");
                        std::io::stdout().flush().ok();
                        break;
                    } else {
                        progress_bar.abandon_with_message("Decryption failed");
                        std::io::stdout().flush().ok();
                        let stderr = status.code().map(|c| format!("Exit code: {}", c)).unwrap_or_else(|| "Unknown error".to_string());
                        anyhow::bail!("PS3Dec failed: {}", stderr);
                    }
                }
                None => {
                    // Process is still running
                    if decrypted_path.exists() {
                        let size = fs::metadata(decrypted_path).map(|m| m.len()).unwrap_or(0);
                        progress_bar.set_position(size.min(input_size));
                        if size == last_size {
                            stalled_count += 1;
                        } else {
                            stalled_count = 0;
                        }
                        last_size = size;
                        if stalled_count > max_stalled {
                            if !used_spinner {
                                progress_bar.println("Warning: Decryption appears stalled. Output file size is not growing. Showing spinner instead.");
                                progress_bar.abandon_with_message("Decryption appears stalled");
                                let spinner = ProgressBar::new_spinner();
                                spinner.set_style(
                                    ProgressStyle::default_spinner()
                                        .template("{spinner:.green} Decrypting... {elapsed_precise}")
                                        .unwrap()
                                        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
                                );
                                spinner.set_draw_target(ProgressDrawTarget::stdout());
                                spinner.enable_steady_tick(Duration::from_millis(120));
                                used_spinner = true;
                            }
                        }
                    }
                    if start_time.elapsed() > timeout_duration {
                        progress_bar.abandon_with_message("Decryption timed out");
                        std::io::stdout().flush().ok();
                        let _ = child.kill().await;
                        anyhow::bail!("Decryption timed out after {} seconds", self.config.decryption_timeout);
                    }
                    sleep(poll_interval).await;
                }
            }
        }

        // Final error check
        if decrypted_path.exists() {
            let final_size = fs::metadata(decrypted_path).map(|m| m.len()).unwrap_or(0);
            if final_size < input_size / 2 {
                progress_bar.println("Warning: Decrypted file is much smaller than the input. Decryption may have failed.");
            }
        } else {
            anyhow::bail!("Decryption failed: Output file was not created.");
        }
        println!("PS3 ISO decryption completed successfully");
        std::io::stdout().flush().ok();
        Ok(())
    }

    /// Validates that the PS3Dec binary is available and executable.
    pub fn validate_decryptor(&self) -> Result<()> {
        let decryptor_path = self.config.decryptor_path();
        
        if !decryptor_path.exists() {
            anyhow::bail!(
                "PS3Dec binary not found at: {}. Please build the PS3Dec program first:\n\
                 cd decryptor/PS3Dec && mkdir -p build && cd build && cmake .. && make",
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
                        "PS3Dec binary is not executable: {}. Please make it executable with 'chmod +x {}'",
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
} 