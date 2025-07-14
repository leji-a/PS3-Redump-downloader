use serde::{Deserialize, Serialize};

/// Represents a PS3 game entry with title, download link, size, and a lowercased title for fast search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    /// The display title of the game (may include .zip extension)
    pub title: String,
    /// The relative download link for the game
    pub link: String,
    /// The size of the game as a string (e.g., '4.2 GB')
    pub size: String,
    /// Lowercased version of the title for fast case-insensitive search
    #[serde(skip)]
    pub lowercased_title: String,
    /// Game type (PS3 for this downloader)
    pub game_type: GameType,
    /// Whether the game needs decryption (true for PS3 games)
    pub needs_decryption: bool,
    /// Game region (optional)
    pub region: Option<String>,
    /// The key file name for this game (optional)
    pub key_file: Option<String>,
    /// The decryption key for this game (optional)
    pub key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameType {
    PS3,
}

impl Game {
    /// Returns the cleaned title (removes .zip extension)
    pub fn clean_title(&self) -> String {
        self.title.replace(".zip", "")
    }

    /// Creates a new Game with lowercased_title initialized
    pub fn with_lowercased(mut self) -> Self {
        self.lowercased_title = self.title.to_lowercase();
        self
    }

    /// Creates a new PS3 game
    pub fn new_ps3(title: String, link: String, size: String, region: Option<String>) -> Self {
        let game = Self {
            title,
            link,
            size,
            lowercased_title: String::new(),
            game_type: GameType::PS3,
            needs_decryption: true, // PS3 games always need decryption
            region,
            key_file: None,
            key: None,
        };
        game.with_lowercased()
    }

    /// Sets the key file for this game
    pub fn with_key_file(mut self, key_file: String) -> Self {
        self.key_file = Some(key_file);
        self
    }

    /// Sets the decryption key for this game
    pub fn with_key(mut self, key: String) -> Self {
        self.key = Some(key);
        self
    }

    /// Gets the game identifier for key lookup
    pub fn get_game_id(&self) -> String {
        // Use the clean title as the game ID to match the key lookup format
        self.clean_title()
    }

    /// Returns the output ISO filename in the format regioncode-nameofgame.iso
    pub fn output_iso_filename(&self) -> String {
        let clean = self.clean_title();
        // Split at the first space or dash to get region code
        let mut parts = clean.splitn(2, |c: char| c == ' ' || c == '-');
        let region = parts.next().unwrap_or("").to_lowercase();
        let rest = parts.next().unwrap_or("").trim();
        // Extract main game name (up to first parenthesis or end)
        let main_name = rest
            .split('(')
            .next()
            .unwrap_or("")
            .trim()
            .replace([' ', '-', ',', ':', ';', '\'', '"'], "_")
            .replace("__", "_")
            .trim_matches('_')
            .to_lowercase();
        if !region.is_empty() && !main_name.is_empty() {
            format!("{}-{}.iso", region, main_name)
        } else {
            // fallback to cleaned title
            format!("{}.iso", clean.replace([' ', '-', '(', ')', ','], "_").to_lowercase())
        }
    }
}
