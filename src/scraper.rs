use crate::{config::Config, models::Game};
use anyhow::Result;
use reqwest;
use scraper::{Html, Selector};
use serde_json;
use std::fs;
use std::path::Path;

/// Scraper handles fetching and parsing PS3 game lists from Redump.
pub struct Scraper {
    config: Config,
}

impl Scraper {
    /// Create a new Scraper with the given configuration.
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Fetches the PS3 game list, either from cache or from the web.
    pub async fn get_ps3_list(&self) -> Result<Vec<Game>> {
        let json_path = self.config.list_ps3_json_path();

        // Try to load from cache first
        if json_path.exists() {
            if let Ok(games) = self.load_from_cache(&json_path) {
                println!("Loaded {} PS3 games from cache", games.len());
                return Ok(games);
            }
        }

        // Fetch from web if cache doesn't exist or is invalid
        println!("Fetching PS3 game list from Redump...");
        let games = self.fetch_ps3_list_from_web().await?;
        
        // Save to cache
        self.save_to_cache(&json_path, &games)?;
        
        println!("Cached {} PS3 games", games.len());
        Ok(games)
    }

    /// Fetches the PS3 game list from the Redump website.
    async fn fetch_ps3_list_from_web(&self) -> Result<Vec<Game>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let response = client.get(&self.config.ps3_iso_url).send().await?;
        
        if !response.status().is_success() {
            anyhow::bail!("Failed to fetch PS3 game list: HTTP {}", response.status());
        }

        let html_content = response.text().await?;
        let document = Html::parse_document(&html_content);

        // Selector for PS3 game links (adjust based on actual Redump PS3 page structure)
        let link_selector = Selector::parse("a[href*='.zip']").unwrap();
        let mut games = Vec::new();

        for element in document.select(&link_selector) {
            if let Some(href) = element.value().attr("href") {
                let title = element.text().collect::<String>().trim().to_string();
                
                // Skip if title is empty or doesn't look like a PS3 game
                if title.is_empty() || !title.contains("PS3") {
                    continue;
                }

                // Extract size information if available
                let size = self.extract_size_from_title(&title);
                
                // Extract region information
                let region = self.extract_region_from_title(&title);

                let game = Game::new_ps3(
                    title.clone(),
                    href.to_string(),
                    size,
                    region,
                );

                games.push(game);
            }
        }

        // Sort games by title for easier browsing
        games.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));

        Ok(games)
    }

    /// Extracts size information from the game title.
    fn extract_size_from_title(&self, title: &str) -> String {
        // Look for size patterns like (4.2 GB), (2.1 GiB), etc.
        if let Some(size_match) = title.find('(') {
            if let Some(end_match) = title[size_match..].find(')') {
                let size_text = &title[size_match + 1..size_match + end_match];
                if size_text.contains("GB") || size_text.contains("GiB") || size_text.contains("MB") {
                    return size_text.to_string();
                }
            }
        }
        
        // Default size if not found
        "Unknown size".to_string()
    }

    /// Extracts region information from the game title.
    fn extract_region_from_title(&self, title: &str) -> Option<String> {
        let regions = ["USA", "Europe", "Japan", "Asia", "Australia", "PAL", "NTSC"];
        
        for region in &regions {
            if title.contains(region) {
                return Some(region.to_string());
            }
        }
        
        None
    }

    /// Loads the game list from the JSON cache file.
    fn load_from_cache(&self, json_path: &Path) -> Result<Vec<Game>> {
        let content = fs::read_to_string(json_path)?;
        let games: Vec<Game> = serde_json::from_str(&content)?;
        
        // Ensure all games have lowercased_title set
        let games = games.into_iter().map(|game| game.with_lowercased()).collect();
        
        Ok(games)
    }

    /// Saves the game list to the JSON cache file.
    fn save_to_cache(&self, json_path: &Path, games: &[Game]) -> Result<()> {
        let json_content = serde_json::to_string_pretty(games)?;
        fs::write(json_path, json_content)?;
        Ok(())
    }
}
