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
        let mut game = Self {
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
        // Extract game ID from title or link
        // This might need adjustment based on actual PS3 game naming conventions
        self.clean_title().to_uppercase().replace(" ", "_")
    }
}
