use anyhow::Result;
use configparser::ini::Ini;
use serde::{Deserialize, Serialize};

/// Configuration for the PS3 downloader application, loaded from config.ini.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Base URL for PS3 ISO downloads
    pub ps3_iso_url: String,
    /// Base URL for PS3 decryption keys
    pub ps3_keys_url: String,
    /// Name of the JSON file containing the list of PS3 games
    pub list_ps3_files_json_name: String,
    /// Whether to use external browser for ISO download
    pub external_iso_download: bool,
    /// Maximum number of download retries
    pub max_retries: u32,
    /// Delay between retries (seconds)
    pub delay_between_retries: u64,
    /// Timeout for requests (seconds)
    pub timeout_request: Option<u64>,
    /// Name of the temporary folder
    pub tmp_folder_name: String,
    /// Name of the ISO folder inside the temporary folder
    pub tmp_iso_folder_name: String,
    /// Path to the PS3 decryption binary
    pub decryptor_path: String,
    /// Timeout for decryption process (seconds)
    pub decryption_timeout: u64,
}

impl Config {
    /// Loads configuration from the given path (expands tilde if present).
    pub fn load(path: &str) -> Result<Self> {
        let mut config = Ini::new();
        config.load(Self::expand_tilde(path)).map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;

        let ps3_url_section = config.get("url", "PS3_ISO").map_or("https://myrient.erista.me/files/Redump/Sony%20-%20PlayStation%203/".to_string(), |s| s.to_string());
        let ps3_keys_url = config.get("url", "PS3_KEYS").map_or("https://myrient.erista.me/files/Redump/Sony%20-%20PlayStation%203%20-%20Disc%20Keys%20TXT/".to_string(), |s| s.to_string());
        
        let list_ps3_files_json_name = config.get("Download", "LIST_PS3_FILES_JSON_NAME").map_or("listPS3Titles.json".to_string(), |s| s.to_string());
        let external_iso_download = config.getuint("Download", "EXTERNAL_ISO").unwrap_or(Some(0)).unwrap_or(0) != 0;
        let max_retries = config.getuint("Download", "MAX_RETRIES").unwrap_or(Some(5)).unwrap_or(5) as u32;
        let delay_between_retries = config.getuint("Download", "DELAY_BETWEEN_RETRIES").unwrap_or(Some(5)).unwrap_or(5) as u64;
        let timeout_request = config.getuint("Download", "TIMEOUT_REQUEST").unwrap_or(None).map(|v| v as u64);
        
        let tmp_folder_name = config.get("folder", "TMP_FOLDER_NAME").map_or("~/PS3-Games".to_string(), |s| s.to_string());
        let tmp_iso_folder_name = config.get("folder", "TMP_ISO_FOLDER_NAME").map_or("iso_files".to_string(), |s| s.to_string());

        let decryptor_path = config.get("PS3", "DECRYPTOR_PATH").map_or("./ps3_decryptor".to_string(), |s| s.to_string());
        let decryption_timeout = config.getuint("PS3", "DECRYPTION_TIMEOUT").unwrap_or(Some(300)).unwrap_or(300) as u64;

        let config = Config {
            ps3_iso_url: ps3_url_section,
            ps3_keys_url,
            list_ps3_files_json_name,
            external_iso_download,
            max_retries,
            delay_between_retries,
            timeout_request,
            tmp_folder_name,
            tmp_iso_folder_name,
            decryptor_path,
            decryption_timeout,
        };

        // Validate configuration
        if config.max_retries == 0 {
            anyhow::bail!("MAX_RETRIES must be greater than 0");
        }
        if config.delay_between_retries == 0 {
            anyhow::bail!("DELAY_BETWEEN_RETRIES must be greater than 0");
        }
        if config.decryption_timeout == 0 {
            anyhow::bail!("DECRYPTION_TIMEOUT must be greater than 0");
        }

        Ok(config)
    }

    /// Expands a path that starts with ~ to the user's home directory.
    fn expand_tilde(path: &str) -> std::path::PathBuf {
        if path.starts_with("~/") {
            #[cfg(windows)]
            {
                if let Some(home) = std::env::var_os("USERPROFILE") {
                    return std::path::PathBuf::from(home).join(&path[2..]);
                }
            }
            #[cfg(not(windows))]
            {
                if let Some(home) = std::env::var_os("HOME") {
                    return std::path::PathBuf::from(home).join(&path[2..]);
                }
            }
        }
        std::path::PathBuf::from(path)
    }

    /// Returns the expanded path to the temporary folder.
    pub fn tmp_folder_path(&self) -> std::path::PathBuf {
        Self::expand_tilde(&self.tmp_folder_name)
    }

    /// Returns the expanded path to the ISO folder inside the temporary folder.
    pub fn tmp_iso_folder_path(&self) -> std::path::PathBuf {
        Self::expand_tilde(&self.tmp_folder_name).join(&self.tmp_iso_folder_name)
    }

    /// Returns the expanded path to the JSON file containing the PS3 game list.
    pub fn list_ps3_json_path(&self) -> std::path::PathBuf {
        Self::expand_tilde(&self.tmp_folder_name).join(&self.list_ps3_files_json_name)
    }

    /// Returns the expanded path to the decryption binary.
    pub fn decryptor_path(&self) -> std::path::PathBuf {
        Self::expand_tilde(&self.decryptor_path)
    }

    /// Returns the expanded path to the keys folder.
    pub fn keys_folder_path(&self) -> std::path::PathBuf {
        Self::expand_tilde(&self.tmp_folder_name).join("keys")
    }
}
