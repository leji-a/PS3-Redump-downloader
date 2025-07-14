# PS3 Redump Downloader (Rust)

A simple CLI tool to search, download, extract, and decrypt PlayStation 3 ISOs from the Redump database.

- Fast, minimal, and cross-platform
- Auto-downloads and manages decryption keys
- Progress bars for download, extraction, and decryption
- Rust port inspired by the original Python tool by juanpomares ([link](https://github.com/juanpomares/PS3-Redump-downloader))

## Quick Start

1. **Install Rust:** https://rustup.rs/
2. **Clone and build:**
   ```bash
   git clone https://github.com/leji-a/ps3-redump-downloader.git
   cd ps3-redump-downloader
   cargo build --release
   ```
3. **Get the decryptor:**
   - Download and build [PS3Dec](https://github.com/al3xtjames/PS3Dec/) separately.
   - Set its path in `config.ini` (see below).
4. **Run:**
   ```bash
   ./target/release/ps3-redump-downloader
   ```

## Global Installation

Install globally for easy access from anywhere:

```bash
cargo install --path .
ps3-redump-downloader
```

## Config Example

```ini
[url]
PS3_ISO = https://myrient.erista.me/files/Redump/Sony%20-%20PlayStation%203/
PS3_KEYS = https://myrient.erista.me/files/Redump/Sony%20-%20PlayStation%203%20-%20Disc%20Keys%20TXT/

[Download]
LIST_PS3_FILES_JSON_NAME = listPS3Titles.json
EXTERNAL_ISO = 0
MAX_RETRIES = 10
DELAY_BETWEEN_RETRIES = 10
TIMEOUT_REQUEST = 1800

[folder]
TMP_FOLDER_NAME = ~/PS3-Games
TMP_ISO_FOLDER_NAME = iso_files

[PS3]
DECRYPTOR_PATH = /path/to/PS3Dec
DECRYPTION_TIMEOUT = 300
```
> You can leave 'TMP_ISO_FOLDER_NAME' empty in case you want the isos in 'TMP_FOLDER_NAME'

## Config File Location

The application looks for `config.ini` in these locations (in order):

- **Current directory:** Where you run the binary or `cargo run` (recommended for development)
- **Linux/macOS:**
  - `~/.config/ps3-redump-downloader/config.ini`
  - `/etc/ps3-redump-downloader/config.ini`
- **Windows:**
  - `%APPDATA%\ps3-redump-downloader\config.ini`
  - `C:\ProgramData\ps3-redump-downloader\config.ini`

## Basic Usage

```
$ ps3-redump-downloader
Find PS3 title to download: gran turismo
1. Gran Turismo 5 (Europe)
Enter PS3 title number: 1
Downloading... [progress]
Extracting... [progress]
Decrypting... [progress]
Done!
```

## Download Location

By default, downloaded ISOs are saved to:
- `~/PS3-Games/iso_files/` (Linux/macOS)
- `C:\Users\YourName\PS3-Games\iso_files\` (Windows)

You can change this in the `[folder]` section of `config.ini`.

## Tips
- **Decryption:** Requires [PS3Dec](https://github.com/al3xtjames/PS3Dec/). Set the path in `config.ini`.
- **Download timeout:** Set with `TIMEOUT_REQUEST` (seconds) in `config.ini` (default: 1800 = 30 minutes)
- **Decryption timeout:** Set with `DECRYPTION_TIMEOUT` (seconds) in `config.ini` (default: 300 = 5 minutes)
- **Retries:** Set `MAX_RETRIES` and `DELAY_BETWEEN_RETRIES` for failed downloads
- **EXTERNAL_ISO:** Set to `1` to use your browser for downloads instead of the built-in downloader
- **Game list cache:** The game list is cached as `listPS3Titles.json` in your chosen folder

---
- The decryptor (PS3Dec) is **not included**. Download/build it from [here](https://github.com/al3xtjames/PS3Dec/).

---
