use crate::config::Config;
use anyhow::Result;
use std::fs;
use std::path::Path;

/// Sets up the required folders for temporary files and ISO downloads.
pub fn setup_folders(config: &Config) -> Result<()> {
    check_folder(&config.tmp_folder_path(), &config.tmp_folder_name)?;
    check_folder(&config.tmp_iso_folder_path(), &config.tmp_iso_folder_name)?;
    Ok(())
}

/// Checks if a folder exists, creates it if not, or errors if a file with the same name exists.
fn check_folder(folder_path: &Path, folder_name: &str) -> Result<()> {
    if !folder_path.exists() {
        create_folder(folder_path, folder_name)?;
    } else if !folder_path.is_dir() {
        anyhow::bail!("Please remove the file named as {}", folder_name);
    }
    Ok(())
}

/// Creates a folder and handles errors appropriately.
fn create_folder(folder_path: &Path, folder_name: &str) -> Result<()> {
    fs::create_dir_all(folder_path)
        .map_err(|e| anyhow::anyhow!("Error creating '{}' folder: {}", folder_name, e))
}
