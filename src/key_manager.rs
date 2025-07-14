use crate::{config::Config, models::Game};
use anyhow::Result;
use reqwest;
use std::fs;
use std::path::Path;
use std::collections::HashMap;

/// KeyManager handles downloading and managing PS3 decryption keys.
pub struct KeyManager {
    config: Config,
}

impl KeyManager {
    /// Create a new KeyManager with the given configuration.
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Downloads and caches the PS3 keys list.
    pub async fn download_keys_list(&self) -> Result<HashMap<String, String>> {
        let keys_cache_path = self.config.keys_folder_path().join("keys_cache.json");
        
        // Try to load from cache first
        if keys_cache_path.exists() {
            if let Ok(keys) = self.load_keys_from_cache(&keys_cache_path) {
                println!("Loaded {} PS3 keys from cache", keys.len());
                return Ok(keys);
            }
        }

        // Fetch from web if cache doesn't exist or is invalid
        println!("Fetching PS3 keys list from Redump...");
        let keys = self.fetch_keys_from_web().await?;
        
        // Save to cache
        self.save_keys_to_cache(&keys_cache_path, &keys)?;
        
        println!("Cached {} PS3 keys", keys.len());
        Ok(keys)
    }

    /// Fetches the PS3 keys list from the Redump website.
    async fn fetch_keys_from_web(&self) -> Result<HashMap<String, String>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let response = client.get(&self.config.ps3_keys_url).send().await?;
        
        if !response.status().is_success() {
            anyhow::bail!("Failed to fetch PS3 keys list: HTTP {}", response.status());
        }

        let html_content = response.text().await?;
        let mut keys = HashMap::new();

        // Parse the HTML to extract key files
        // This will need to be adjusted based on the actual structure of the keys page
        for line in html_content.lines() {
            if line.contains(".txt") && line.contains("href") {
                // Extract key file name and parse it
                if let Some(key_info) = self.parse_key_line(line) {
                    keys.insert(key_info.0, key_info.1);
                }
            }
        }

        Ok(keys)
    }

    /// Parses a line from the keys page to extract game ID and key.
    fn parse_key_line(&self, line: &str) -> Option<(String, String)> {
        // This is a placeholder implementation
        // The actual parsing will depend on the structure of the keys page
        if let Some(href_start) = line.find("href=\"") {
            if let Some(href_end) = line[href_start + 6..].find("\"") {
                let key_file = &line[href_start + 6..href_start + 6 + href_end];
                if key_file.ends_with(".txt") {
                    // Extract game ID from filename
                    let game_id = key_file.replace(".txt", "").to_uppercase();
                    // For now, we'll need to download the actual key file
                    // This is a simplified version - you might need to download each key file
                    return Some((game_id, key_file.to_string()));
                }
            }
        }
        None
    }

    /// Downloads a specific key file for a game.
    pub async fn download_key_for_game(&self, game: &Game) -> Result<Option<String>> {
        let game_id = game.get_game_id();
        let keys = self.download_keys_list().await?;
        
        // Look for the key file for this game
        if let Some(key_file) = keys.get(&game_id) {
            let key_url = format!("{}{}", self.config.ps3_keys_url, key_file);
            let key_content = self.download_key_file(&key_url).await?;
            
            // Parse the key from the file content
            if let Some(key) = self.parse_key_from_content(&key_content) {
                return Ok(Some(key));
            }
        }
        
        Ok(None)
    }

    /// Downloads a key file from the given URL.
    async fn download_key_file(&self, url: &str) -> Result<String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let response = client.get(url).send().await?;
        
        if !response.status().is_success() {
            anyhow::bail!("Failed to download key file: HTTP {}", response.status());
        }

        let content = response.text().await?;
        Ok(content)
    }

    /// Parses the key from the key file content.
    fn parse_key_from_content(&self, content: &str) -> Option<String> {
        // This will need to be adjusted based on the actual format of the key files
        for line in content.lines() {
            let line = line.trim();
            if line.len() == 32 && line.chars().all(|c| c.is_ascii_hexdigit()) {
                // Looks like a 32-character hex key
                return Some(line.to_string());
            }
        }
        None
    }

    /// Loads the keys from the JSON cache file.
    fn load_keys_from_cache(&self, cache_path: &Path) -> Result<HashMap<String, String>> {
        let content = fs::read_to_string(cache_path)?;
        let keys: HashMap<String, String> = serde_json::from_str(&content)?;
        Ok(keys)
    }

    /// Saves the keys to the JSON cache file.
    fn save_keys_to_cache(&self, cache_path: &Path, keys: &HashMap<String, String>) -> Result<()> {
        // Ensure the directory exists
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let json_content = serde_json::to_string_pretty(keys)?;
        fs::write(cache_path, json_content)?;
        Ok(())
    }

    /// Finds the best matching key for a game.
    pub async fn find_key_for_game(&self, game: &Game) -> Result<Option<String>> {
        // Try multiple strategies to find the key
        let strategies = vec![
            self.find_key_by_exact_match(game).await,
            self.find_key_by_partial_match(game).await,
            self.find_key_by_alternative_names(game).await,
        ];

        for strategy in strategies {
            if let Ok(Some(key)) = strategy {
                return Ok(Some(key));
            }
        }

        Ok(None)
    }

    /// Finds key by exact game ID match.
    async fn find_key_by_exact_match(&self, game: &Game) -> Result<Option<String>> {
        self.download_key_for_game(game).await
    }

    /// Finds key by partial match of game title.
    async fn find_key_by_partial_match(&self, game: &Game) -> Result<Option<String>> {
        // Implementation for partial matching
        // This would try different variations of the game title
        Ok(None)
    }

    /// Finds key by alternative game names.
    async fn find_key_by_alternative_names(&self, game: &Game) -> Result<Option<String>> {
        // Implementation for alternative name matching
        // This would try common alternative names for the game
        Ok(None)
    }
} 