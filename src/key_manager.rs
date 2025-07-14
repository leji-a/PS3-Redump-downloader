use crate::{config::Config, models::Game};
use anyhow::Result;
use reqwest;
use std::fs;
use std::path::Path;
use std::collections::HashMap;
use std::io::Read;

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
        let document = scraper::Html::parse_document(&html_content);
        let mut keys = HashMap::new();

        // Parse the HTML to extract key files using the same approach as scraper
        let row_selector = scraper::Selector::parse("tbody tr").unwrap();
        let link_selector = scraper::Selector::parse("td.link a").unwrap();

        for row in document.select(&row_selector) {
            // Skip the parent directory row
            if let Some(link_element) = row.select(&link_selector).next() {
                if let Some(href) = link_element.value().attr("href") {
                    let title = link_element.text().collect::<String>().trim().to_string();
                    
                    // Skip if title is empty or doesn't end with .zip
                    if title.is_empty() || !title.ends_with(".zip") {
                        continue;
                    }

                    // Extract game ID from filename
                    let game_id = title.replace(".zip", "");
                    
                    // URL-decode the href
                    let decoded_href = match percent_encoding::percent_decode_str(href).decode_utf8() {
                        Ok(decoded) => decoded.to_string(),
                        Err(_) => href.to_string(), // Fallback to original if decoding fails
                    };
                    
                    keys.insert(game_id.clone(), decoded_href.clone());
                    
                    // Debug: Print first few keys to see the format
                    if keys.len() <= 5 {
                        println!("DEBUG: Parsed key - ID: '{}', href: '{}'", game_id, decoded_href);
                    }
                }
            }
        }

        Ok(keys)
    }

    /// Downloads a specific key file for a game.
    pub async fn download_key_for_game(&self, game: &Game) -> Result<Option<String>> {
        let game_id = game.get_game_id();
        println!("DEBUG: Looking for game ID: '{}'", game_id);
        
        let keys = self.download_keys_list().await?;
        println!("DEBUG: Found {} keys in cache", keys.len());
        
        // Look for the key file for this game
        if let Some(key_file) = keys.get(&game_id) {
            println!("DEBUG: Found key file: '{}'", key_file);
            let key_url = format!("{}{}", self.config.ps3_keys_url, key_file);
            let key_content = self.download_key_file(&key_url).await?;
            
            // Parse the key from the zip file content
            if let Some(key) = self.parse_key_from_zip_content(&key_content) {
                println!("DEBUG: Successfully extracted key: {}", key);
                return Ok(Some(key));
            } else {
                println!("DEBUG: Failed to parse key from zip content");
            }
        } else {
            println!("DEBUG: No key file found for game ID: '{}'", game_id);
            // Let's check what keys we have that might match
            let matching_keys: Vec<_> = keys.keys()
                .filter(|k| k.to_lowercase().contains(&game_id.to_lowercase()))
                .take(5)
                .collect();
            if !matching_keys.is_empty() {
                println!("DEBUG: Similar keys found: {:?}", matching_keys);
            }
        }
        
        Ok(None)
    }

    /// Downloads a key file from the given URL.
    async fn download_key_file(&self, url: &str) -> Result<Vec<u8>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let response = client.get(url).send().await?;
        
        if !response.status().is_success() {
            anyhow::bail!("Failed to download key file: HTTP {}", response.status());
        }

        let content = response.bytes().await?;
        Ok(content.to_vec())
    }

    /// Parses the key from the zip file content.
    fn parse_key_from_zip_content(&self, zip_data: &[u8]) -> Option<String> {
        // Use zip crate to extract the key from the zip file
        use std::io::Cursor;
        
        println!("DEBUG: Attempting to parse zip file of {} bytes", zip_data.len());
        
        let cursor = Cursor::new(zip_data);
        if let Ok(mut archive) = zip::ZipArchive::new(cursor) {
            println!("DEBUG: Zip archive opened successfully, {} files found", archive.len());
            
            // Look for .key files inside the zip
            for i in 0..archive.len() {
                if let Ok(mut file) = archive.by_index(i) {
                    let file_name = file.name().to_string();
                    println!("DEBUG: Found file in zip: '{}'", file_name);
                    
                    if file_name.ends_with(".key") {
                        let mut buffer = Vec::new();
                        if file.read_to_end(&mut buffer).is_ok() {
                            // Try as text
                            if let Ok(text) = std::str::from_utf8(&buffer) {
                                let trimmed = text.trim();
                                if trimmed.len() == 32 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
                                    println!("DEBUG: Found 32-char hex key in text: {}", trimmed);
                                    return Some(trimmed.to_lowercase());
                                }
                            }
                            // Try as 16-byte binary
                            if buffer.len() == 16 {
                                let hex_string = buffer.iter().map(|b| format!("{:02x}", b)).collect::<String>();
                                println!("DEBUG: Converted 16-byte binary key to hex: {}", hex_string);
                                return Some(hex_string);
                            }
                            println!("DEBUG: .key file is neither valid text nor 16-byte binary");
                        } else {
                            println!("DEBUG: Failed to read .key file as bytes: '{}'", file_name);
                        }
                    }
                } else {
                    println!("DEBUG: Failed to access file at index {}", i);
                }
            }
        } else {
            println!("DEBUG: Failed to open zip archive");
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
    async fn find_key_by_partial_match(&self, _game: &Game) -> Result<Option<String>> {
        // Implementation for partial matching
        // This would try different variations of the game title
        Ok(None)
    }

    /// Finds key by alternative game names.
    async fn find_key_by_alternative_names(&self, _game: &Game) -> Result<Option<String>> {
        // Implementation for alternative name matching
        // This would try common alternative names for the game
        Ok(None)
    }
} 