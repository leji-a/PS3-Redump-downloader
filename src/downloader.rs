use crate::{config::Config, models::Game, decryptor::Decryptor};
use anyhow::Result;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::Path;
use std::io::{Read, Write};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncSeekExt, SeekFrom, AsyncWriteExt};
use zip::ZipArchive;

/// Downloader handles downloading, extracting, and decrypting PS3 ISO files.
pub struct Downloader {
    config: Config,
    decryptor: Decryptor,
}

impl Downloader {
    /// Create a new Downloader with the given configuration.
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
            decryptor: Decryptor::new(config),
        }
    }

    /// Download, extract, and decrypt the selected PS3 game.
    pub async fn download_ps3_element(&self, game: &Game) -> Result<()> {
        let title = game.clean_title();
        println!("\nSelected {}\n", title);

        // Validate decryption binary before starting
        self.decryptor.validate_decryptor()?;

        // Download the key for this game
        println!("Downloading decryption key...");
        let key = self.decryptor.key_manager().find_key_for_game(game).await?;
        
        if key.is_none() {
            anyhow::bail!("Could not find decryption key for game: {}. The game may not be available or the key may not exist.", title);
        }
        
        let key = key.unwrap();
        println!("Found decryption key for {}", title);

        // Construct the full URL by combining base URL with relative path
        let full_url = format!("{}{}", self.config.ps3_iso_url, game.link);
        self.download_extract_and_decrypt(&full_url, &title, &key).await?;
        println!("\n{} downloaded and decrypted :)", title);

        // Open the folder containing the decrypted ISO
        let decrypted_iso_file = self
            .config
            .tmp_iso_folder_path()
            .join(format!("{}.iso", title));
        if decrypted_iso_file.exists() {
            self.open_explorer(&decrypted_iso_file);
        }

        Ok(())
    }

    /// Download, extract, and decrypt the file, handling both direct and external download methods.
    async fn download_extract_and_decrypt(&self, link: &str, title: &str, key: &str) -> Result<()> {
        println!(" # PS3 ISO file...");

        let decrypted_file_name = format!("{}.iso", title);
        let decrypted_file_path = self.config.tmp_iso_folder_path().join(&decrypted_file_name);

        // Skip download if file already exists
        if decrypted_file_path.exists() {
            println!(" - File previously downloaded and decrypted :)\n");
            return Ok(());
        }

        let new_file_name = format!("{}.zip", title);
        let tmp_file = self.config.tmp_iso_folder_path().join(&new_file_name);
        let encrypted_file_name = format!("{}_encrypted.iso", title);
        let encrypted_file_path = self.config.tmp_iso_folder_path().join(&encrypted_file_name);

        if self.config.external_iso_download {
            self.download_using_navigator(link, &new_file_name, &tmp_file, &encrypted_file_name)
                .await?;
        } else {
            self.download_using_request(link, &tmp_file).await?;
        }

        // Unzip and clean up
        if tmp_file.exists() {
            self.unzip_file(&tmp_file).await?;
            self.remove_file(&tmp_file)?;
        }

        // Decrypt the extracted ISO with the key
        if encrypted_file_path.exists() {
            self.decryptor.decrypt_iso(&encrypted_file_path, &decrypted_file_path, key).await?;
            
            // Clean up encrypted file after successful decryption
            self.remove_file(&encrypted_file_path)?;
        }

        println!(" ");
        Ok(())
    }

    /// Downloads a file using reqwest, supporting resume and progress bar.
    /// Retries on failure up to max_retries.
    async fn download_using_request(&self, link: &str, file_path: &Path) -> Result<()> {
        let total_size = self.get_file_size(link).await?;
        let mut retries = 0;

        while retries < self.config.max_retries {
            let mut headers = reqwest::header::HeaderMap::new();
            let mut first_byte = 0;

            if let Some(size) = total_size {
                if file_path.exists() {
                    first_byte = fs::metadata(file_path)?.len();
                    if first_byte >= size {
                        println!("The file {} was downloaded previously.", file_path.display());
                        return Ok(());
                    }
                }
                headers.insert("Range", format!("bytes={}-{}", first_byte, size - 1).parse()?);
            }

            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(
                    self.config.timeout_request.unwrap_or(1800), // Longer timeout for PS3 files
                ))
                .connect_timeout(std::time::Duration::from_secs(30))
                .build()?;

            println!("Attempting download from: {}", link);
            
            match client.get(link).headers(headers).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        // Open file for append and seek to the correct position
                        let mut file = OpenOptions::new()
                            .create(true)
                            .append(false)
                            .write(true)
                            .open(file_path)
                            .await?;
                        file.seek(SeekFrom::Start(first_byte)).await?;
                        let mut stream = response.bytes_stream();

                        let progress_bar = if let Some(total) = total_size {
                            let pb = ProgressBar::new(total);
                            pb.set_style(
                                ProgressStyle::default_bar()
                                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                                    .unwrap()
                                    .progress_chars("#>-")
                            );
                            pb.set_position(first_byte);
                            Some(pb)
                        } else {
                            None
                        };

                        let mut downloaded = first_byte;
                        let mut error_occurred = false;
                        while let Some(chunk_result) = stream.next().await {
                            match chunk_result {
                                Ok(chunk) => {
                                    file.write_all(&chunk).await?;
                                    downloaded += chunk.len() as u64;
                                    if let Some(pb) = &progress_bar {
                                        pb.set_position(downloaded);
                                    }
                                }
                                Err(e) => {
                                    println!("Error during download: {}", e);
                                    error_occurred = true;
                                    break;
                                }
                            }
                        }

                        if error_occurred {
                            retries += 1;
                            if retries < self.config.max_retries {
                                println!("Waiting {} seconds before retry...", self.config.delay_between_retries);
                                tokio::time::sleep(tokio::time::Duration::from_secs(
                                    self.config.delay_between_retries,
                                ))
                                .await;
                            }
                            continue;
                        }

                        // Check if download was completed successfully
                        if let Some(pb) = &progress_bar {
                            if let Some(length) = pb.length() {
                                if pb.position() >= length {
                                    pb.finish_with_message("Download completed");
                                    break;
                                } else {
                                    pb.finish_with_message("Download incomplete");
                                }
                            } else {
                                pb.finish_with_message("Download completed");
                                break;
                            }
                        } else {
                            break;
                        }
                    } else {
                        println!("HTTP error: {} - {}", response.status(), response.status().as_str());
                        retries += 1;
                    }
                }
                Err(e) => {
                    println!("Request error (attempt {}/{}): {}", retries + 1, self.config.max_retries, e);
                    retries += 1;
                }
            }
        }

        if retries == self.config.max_retries {
            anyhow::bail!(
                "Failed to download file after {} attempts.",
                self.config.max_retries
            );
        }

        Ok(())
    }

    /// Prompts the user to download the file manually using a browser.
    async fn download_using_navigator(
        &self,
        route: &str,
        downloaded_file_name: &str,
        zip_file: &Path,
        encrypted_file: &str,
    ) -> Result<()> {
        let destination_folder = self.config.tmp_iso_folder_path();

        println!("Opening browser with download link ({})", route);
        open::that(route)?;

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        println!(
            "Please download the file and copy '{}' to '{}'",
            downloaded_file_name,
            destination_folder.display()
        );
        self.open_explorer(&destination_folder);

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        println!("Waiting for the file to be copied...");

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        // Wait until the file is present
        while !zip_file.exists() && !destination_folder.join(encrypted_file).exists() {
            println!(
                "\nFile not found!! Make sure to download and copy the file to '{}'",
                destination_folder.display()
            );
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
        }

        println!();
        Ok(())
    }

    /// Gets the file size from the server using a range request or content-length.
    async fn get_file_size(&self, link: &str) -> Result<Option<u64>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()?;
            
        let response = client.get(link).header("Range", "bytes=0-1").send().await?;

        if let Some(range_header) = response.headers().get("content-range") {
            if let Ok(range_str) = range_header.to_str() {
                if let Some(total_str) = range_str.split('/').nth(1) {
                    if let Ok(total_size) = total_str.parse::<u64>() {
                        return Ok(Some(total_size));
                    }
                }
            }
        }

        // Try to get content-length as fallback
        if let Some(content_length) = response.headers().get("content-length") {
            if let Ok(length_str) = content_length.to_str() {
                if let Ok(total_size) = length_str.parse::<u64>() {
                    return Ok(Some(total_size));
                }
            }
        }

        Ok(None)
    }

    /// Unzips the downloaded file, showing a progress bar if possible.
    async fn unzip_file(&self, zip_path: &Path) -> Result<()> {
        println!("Extracting ZIP file...");
        let dest = zip_path.parent().unwrap();
        
        // Check file size first
        let file_size = fs::metadata(zip_path)?.len();
        if file_size == 0 {
            anyhow::bail!("ZIP file is empty (0 bytes)");
        }
        
        let file = fs::File::open(zip_path)?;
        let mut archive = match ZipArchive::new(file) {
            Ok(archive) => archive,
            Err(e) => {
                anyhow::bail!("Invalid ZIP archive: {}. The file may be corrupted or incomplete. Try downloading again.", e);
            }
        };

        // Calculate total size first
        let total_size: u64 = {
            let file_names: Vec<String> = archive.file_names().map(|s| s.to_string()).collect();
            file_names
                .iter()
                .filter_map(|name| {
                    archive
                        .by_name(name)
                        .ok()
                        .and_then(|file| file.size().checked_add(0))
                })
                .sum()
        };

        if total_size > 0 {
            let progress_bar = ProgressBar::new(total_size);
            progress_bar.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} Extracting: [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                    .unwrap()
                    .progress_chars("#>-")
            );

            for i in 0..archive.len() {
                let mut file = archive.by_index(i)?;
                let outpath = dest.join(file.name());

                if file.name().ends_with('/') {
                    fs::create_dir_all(&outpath)?;
                } else {
                    if let Some(p) = outpath.parent() {
                        if !p.exists() {
                            fs::create_dir_all(p)?;
                        }
                    }
                    let mut outfile = fs::File::create(&outpath)?;
                    let mut buffer = [0u8; 8192];
                    loop {
                        let bytes_read = file.read(&mut buffer)?;
                        if bytes_read == 0 {
                            break;
                        }
                        outfile.write_all(&buffer[..bytes_read])?;
                        progress_bar.inc(bytes_read as u64);
                    }
                }
            }

            progress_bar.finish_with_message("Extraction completed");
        } else {
            // Fallback spinner for small files or when size calculation fails
            let spinner = ProgressBar::new_spinner();
            spinner.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} Extracting files... {elapsed_precise}")
                    .unwrap()
                    .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
            );

            for i in 0..archive.len() {
                let mut file = archive.by_index(i)?;
                let outpath = dest.join(file.name());

                if file.name().ends_with('/') {
                    fs::create_dir_all(&outpath)?;
                } else {
                    if let Some(p) = outpath.parent() {
                        if !p.exists() {
                            fs::create_dir_all(p)?;
                        }
                    }
                    let mut outfile = fs::File::create(&outpath)?;
                    let mut buffer = [0u8; 8192];
                    loop {
                        let bytes_read = file.read(&mut buffer)?;
                        if bytes_read == 0 {
                            break;
                        }
                        outfile.write_all(&buffer[..bytes_read])?;
                    }
                }
                spinner.tick();
            }

            spinner.finish_with_message("Extraction completed");
        }

        Ok(())
    }

    /// Removes a file, printing an error if it fails.
    fn remove_file(&self, file_path: &Path) -> Result<()> {
        match fs::remove_file(file_path) {
            Ok(_) => Ok(()),
            Err(e) => {
                println!("Error removing {}: {}", file_path.display(), e);
                Ok(())
            }
        }
    }

    /// Opens the file explorer at the given path.
    fn open_explorer(&self, path: &Path) {
        if let Err(e) = open::that(path) {
            println!("Error opening {}: {}", path.display(), e);
        }
    }
}
