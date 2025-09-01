use crate::{config::Config, decryptor::Decryptor, models::Game};
use anyhow::Result;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncSeekExt, AsyncWriteExt, SeekFrom};
use zip::ZipArchive;

/// Minimal PARAM.SFO parser
mod sfo {
    use std::collections::HashMap;

    pub struct Sfo {
        pub entries: HashMap<String, String>,
    }

    impl Sfo {
        pub fn from_bytes(data: &[u8]) -> Option<Self> {
            if data.len() < 20 || &data[0..4] != b"\0PSF" {
                return None;
            }

            let key_table_start = u32::from_le_bytes(data[8..12].try_into().ok()?) as usize;
            let data_table_start = u32::from_le_bytes(data[12..16].try_into().ok()?) as usize;
            let count = u32::from_le_bytes(data[16..20].try_into().ok()?) as usize;

            let mut entries = HashMap::new();
            let mut offset = 20;

            for _ in 0..count {
                let key_offset =
                    u16::from_le_bytes(data[offset..offset + 2].try_into().ok()?) as usize;
                let data_fmt =
                    u16::from_le_bytes(data[offset + 2..offset + 4].try_into().ok()?) as u32;
                let data_len =
                    u32::from_le_bytes(data[offset + 4..offset + 8].try_into().ok()?) as usize;
                let data_offset =
                    u32::from_le_bytes(data[offset + 12..offset + 16].try_into().ok()?) as usize;
                offset += 16;

                let key_end = data[key_table_start + key_offset..]
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(0);
                let key = String::from_utf8_lossy(
                    &data[key_table_start + key_offset..key_table_start + key_offset + key_end],
                )
                .to_string();

                let value_offset = data_table_start + data_offset;
                let value_bytes = &data[value_offset..value_offset + data_len];

                if data_fmt == 516 {
                    if let Ok(val) = String::from_utf8(value_bytes.to_vec()) {
                        entries.insert(key, val.trim_end_matches('\0').to_string());
                    }
                }
            }

            Some(Sfo { entries })
        }

        pub fn get(&self, key: &str) -> Option<&String> {
            self.entries.get(key)
        }
    }
}

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
        self.download_extract_and_decrypt(&full_url, game, &key)
            .await?;
        println!("\n{} downloaded and decrypted :)", title);

        // Open the folder containing the decrypted ISO
        let decrypted_iso_file = self
            .config
            .tmp_iso_folder_path()
            .join(game.output_iso_filename());
        if decrypted_iso_file.exists() {
            self.open_explorer(&decrypted_iso_file);
        }

        Ok(())
    }

    /// Download, extract, and decrypt the file, handling both direct and external download methods.
    async fn download_extract_and_decrypt(&self, link: &str, game: &Game, key: &str) -> Result<()> {
        println!(" # PS3 ISO file...");

        let tmp_folder = self.config.tmp_iso_folder_path().join(game.clean_title());
        fs::create_dir_all(&tmp_folder)?;

        // --- NEW: clear temp folder to avoid leftovers from previous games ---
        if tmp_folder.exists() {
            fs::remove_dir_all(&tmp_folder)?;
        }
        fs::create_dir_all(&tmp_folder)?;
        // -------------------------------------------------------------

        let decrypted_file_name = game.output_iso_filename();
        let decrypted_file_path = tmp_folder.join(&decrypted_file_name);

        // Skip download if file already exists
        if decrypted_file_path.exists() {
            println!(" - File previously downloaded and decrypted :)\n");
            return Ok(());
        }

        let new_file_name = format!("{}.zip", game.clean_title());
        let tmp_file = tmp_folder.join(&new_file_name);
        let encrypted_file_name = format!("{}.iso", game.clean_title());
        let encrypted_file_path = tmp_folder.join(&encrypted_file_name);

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

            // After extraction, find the ISO and rename it to gamename.iso
            use std::ffi::OsStr;
            if let Ok(entries) = fs::read_dir(&tmp_folder) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension() == Some(OsStr::new("iso")) {
                        if path != encrypted_file_path {
                            if let Err(e) = fs::rename(&path, &encrypted_file_path) {
                                println!(
                                    "Error renaming extracted ISO: {} -> {}: {}",
                                    path.display(),
                                    encrypted_file_path.display(),
                                    e
                                );
                            }
                        }
                        break;
                    }
                }
            }
        }

        // Decrypt the extracted ISO with the key
        if encrypted_file_path.exists() {
            self.decryptor
                .decrypt_iso(&encrypted_file_path, &decrypted_file_path, key)
                .await?;
            self.remove_file(&encrypted_file_path)?;

            // Rename ISO using PARAM.SFO with fallback
            self.rename_iso_with_param_sfo(&decrypted_file_path)?;
        }

        println!(" ");
        Ok(())
    }

    /// Extracts TITLE_ID and TITLE from decrypted ISO and renames the file.
    /// Falls back to old naming if PARAM.SFO can't be read.
    fn rename_iso_with_param_sfo(&self, iso_path: &Path) -> Result<()> {
        let tmp_folder = self.config.tmp_iso_folder_path();
        let param_sfo_path = tmp_folder.join("PARAM.SFO");

        // Try to extract PARAM.SFO with 7z
        let status = std::process::Command::new("7z")
            .args([
                "e",
                iso_path.to_str().unwrap(),
                "PS3_GAME/PARAM.SFO",
                &format!("-o{}", tmp_folder.display()),
                "-y",
            ])
            .status();

        if let Ok(status) = status {
            if !status.success() || !param_sfo_path.exists() {
                println!("⚠️ Could not extract PARAM.SFO, keeping original filename.");
                return Ok(()); // fallback
            }
        } else {
            println!("⚠️ Failed to run 7z, keeping original filename.");
            return Ok(()); // fallback
        }

        // Try parsing PARAM.SFO
        let mut buf = Vec::new();
        File::open(&param_sfo_path)?.read_to_end(&mut buf)?;
        let param = match sfo::Sfo::from_bytes(&buf) {
            Some(p) => p,
            None => {
                println!("⚠️ Invalid PARAM.SFO, keeping original filename.");
                let _ = fs::remove_file(&param_sfo_path);
                return Ok(()); // fallback
            }
        };

        let title_id = param.get("TITLE_ID").cloned().unwrap_or("UNKNOWN".into());
        let title = param.get("TITLE").cloned().unwrap_or("Unknown".into());

        let safe_title = title
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect::<String>();

        let new_name = format!("{}-{}.iso", title_id, safe_title);
        let new_path = iso_path.parent().unwrap().join(&new_name);

        if iso_path != new_path {
            fs::rename(&iso_path, &new_path)?;
            println!("✅ Renamed ISO to {}", new_path.display());
        }

        let _ = fs::remove_file(&param_sfo_path);
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
                        println!(
                            "The file {} was downloaded previously.",
                            file_path.display()
                        );
                        return Ok(());
                    }
                }
                headers.insert(
                    "Range",
                    format!("bytes={}-{}", first_byte, size - 1).parse()?,
                );
            }

            // Print the message before creating the progress bar
            println!("Attempting download from: {}", link);
            std::io::stdout().flush().ok();
            let progress_bar = if let Some(total) = total_size {
                let pb = ProgressBar::new(total);
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                        .unwrap()
                        .progress_chars("#>-")
                );
                pb.set_draw_target(indicatif::ProgressDrawTarget::stdout());
                std::io::stdout().flush().ok();
                Some(pb)
            } else {
                None
            };

            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(
                    self.config.timeout_request.unwrap_or(1800), // Longer timeout for PS3 files
                ))
                .connect_timeout(std::time::Duration::from_secs(30))
                .build()?;

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
                        // Use the new streaming API for reqwest 0.12
                        let mut stream = response.bytes_stream();

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
                                    if let Some(pb) = &progress_bar {
                                        pb.println(format!("Error during download: {}", e));
                                    } else {
                                        println!("Error during download: {}", e);
                                    }
                                    error_occurred = true;
                                    break;
                                }
                            }
                        }
                        if let Some(pb) = &progress_bar {
                            if let Some(length) = pb.length() {
                                if pb.position() >= length {
                                    pb.finish_with_message("Download completed");
                                } else {
                                    pb.finish_with_message("Download incomplete");
                                }
                            } else {
                                pb.finish_with_message("Download completed");
                            }
                        }
                        std::io::stdout().flush().ok();
                        if let Some(pb) = progress_bar {
                            drop(pb);
                        }
                        if error_occurred {
                            retries += 1;
                            if retries < self.config.max_retries {
                                println!(
                                    "Waiting {} seconds before retry...",
                                    self.config.delay_between_retries
                                );
                                tokio::time::sleep(tokio::time::Duration::from_secs(
                                    self.config.delay_between_retries,
                                ))
                                .await;
                            }
                            continue;
                        }
                        break;
                    } else {
                        println!(
                            "HTTP error: {} - {}",
                            response.status(),
                            response.status().as_str()
                        );
                        retries += 1;
                    }
                }
                Err(e) => {
                    println!(
                        "Request error (attempt {}/{}): {}",
                        retries + 1,
                        self.config.max_retries,
                        e
                    );
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
        use indicatif::ProgressDrawTarget;
        println!("Extracting ZIP file...");
        std::io::stdout().flush().ok();
        let dest = zip_path.parent().unwrap();
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
        let total_files = archive.len();
        let mut total_size: u64 = 0;
        let mut file_sizes = Vec::with_capacity(total_files);
        for i in 0..total_files {
            if let Ok(file) = archive.by_index(i) {
                let size = file.size();
                total_size += size;
                file_sizes.push(size);
            } else {
                file_sizes.push(0);
            }
        }
        std::io::stdout().flush().ok();
        if total_size > 0 {
            let progress_bar = ProgressBar::new(total_size);
            progress_bar.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} Extracting: [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                    .unwrap()
                    .progress_chars("#>-")
            );
            progress_bar.set_draw_target(ProgressDrawTarget::stdout());
            progress_bar.tick();
            std::io::stdout().flush().ok();
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
            std::io::stdout().flush().ok();
        } else {
            // Always show a progress bar based on file count if size is unknown
            let progress_bar = ProgressBar::new(total_files as u64);
            progress_bar.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} Extracting: [{bar:40.cyan/blue}] {pos}/{len} files ({eta})")
                    .unwrap()
                    .progress_chars("#>-")
            );
            progress_bar.set_draw_target(ProgressDrawTarget::stdout());
            progress_bar.tick();
            std::io::stdout().flush().ok();
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
                progress_bar.inc(1);
            }
            progress_bar.finish_with_message("Extraction completed");
            std::io::stdout().flush().ok();
        }
        Ok(())
    }

    /// Removes a file, printing an error if it fails.
    fn remove_file(&self, file_path: &Path) -> Result<()> {
        // Attempt to remove the file; log error if it fails
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
