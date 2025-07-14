use anyhow::Result;
use ps3_redump_downloader::{
    config::Config, downloader::Downloader, models::Game, scraper::Scraper, utils::setup_folders,
};
use tokio::io::{self, AsyncBufReadExt, BufReader};
use std::io::Write;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = Config::load("config.ini")?;

    // Setup working folders
    setup_folders(&config)?;

    // Initialize scraper and downloader
    let scraper = Scraper::new(&config);
    let downloader = Downloader::new(&config);

    // Get PS3 game list
    let games = scraper.get_ps3_list().await?;

    // Main application loop
    run_main_loop(&downloader, games).await?;

    Ok(())
}

/// Main interactive loop for searching and downloading PS3 games.
/// Uses async-compatible input/output for better performance.
async fn run_main_loop(
    downloader: &Downloader,
    games: Vec<Game>,
) -> Result<()> {
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut input = String::new();
    loop {
        print!("Find PS3 title to download (leave empty to exit): ");
        std::io::stdout().flush()?;
        input.clear();
        reader.read_line(&mut input).await?;
        let search_input = input.trim();

        if search_input.is_empty() {
            println!("Exiting...");
            break Ok(());
        }

        let filtered_games = filter_games(&games, search_input);

        if filtered_games.is_empty() {
            println!("No PS3 games found\n");
            continue;
        }

        print_games(&filtered_games);

        print!("Enter PS3 title number [1-{}]: ", filtered_games.len());
        std::io::stdout().flush()?;
        input.clear();
        reader.read_line(&mut input).await?;

        if let Ok(file_number) = input.trim().parse::<usize>() {
            if file_number > 0 && file_number <= filtered_games.len() {
                let selected_game = &filtered_games[file_number - 1];
                downloader.download_ps3_element(selected_game).await?;
            } else {
                println!("Number not in valid range (1-{})\n", filtered_games.len());
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
    }
}

/// Filters PS3 games by search string using the precomputed lowercased_title for efficiency.
fn filter_games<'a>(games: &'a [Game], search: &str) -> Vec<&'a Game> {
    let search_lower = search.to_lowercase();
    let searches: Vec<&str> = search_lower.split_whitespace().collect();

    games
        .iter()
        .filter(|game| {
            searches.iter().all(|search| game.lowercased_title.contains(search))
        })
        .collect()
}

/// Displays the list of filtered PS3 games with their titles and sizes.
fn print_games(games: &[&Game]) {
    for (index, game) in games.iter().enumerate() {
        let region_info = game.region.as_ref().map(|r| format!(" ({})", r)).unwrap_or_default();
        println!("{}. {}{} ({})", index + 1, game.title, region_info, game.size);
    }
    println!();
}
