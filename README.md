# PS3 Redump Downloader

A fast and efficient Rust application for downloading PlayStation 3 games from Redump databases. This tool provides a command-line interface to search, download, extract, decrypt PS3 ISO files with automatic key management, progress indicators and resume capability.

## Table of Contents

1. [Features](#features)
2. [Prerequisites](#prerequisites)
3. [Installation](#installation)
4. [PS3Dec Usage](#ps3dec-usage)
5. [Usage](#usage)
6. [Configuration](#configuration)
7. [Troubleshooting](#troubleshooting)
8. [Building from Source](#building-from-source)
9. [Contributing](#contributing)

## Features

- 🔍 **Search & Filter**: Search PS3 games by title with real-time filtering
- ⬇️ **Resume Downloads**: Automatically resumes interrupted downloads
- 📊 **Progress Indicators**: Visual progress bars for downloads, extraction, and decryption
- 🗜️ **Auto-Extraction**: Automatically extracts ZIP files to encrypted ISO format
- 🔑 **Auto-Key Management**: Automatically downloads and manages PS3 decryption keys
- 🔓 **Auto-Decryption**: Automatically decrypts PS3 ISOs using PS3Dec and keys
- 🔧 **Configurable**: Customizable timeouts, retries, and folder paths
- 🌐 **Cross-Platform**: Works on Windows, macOS, and Linux

## Prerequisites

- **Rust** (1.70 or higher)
- **Internet connection** for downloading games and keys
- **Sufficient disk space** for PS3 ISO files (typically 4-50GB per game)
- **Build tools**: CMake, GCC/G++, Make
- **PS3Dec**: C program for decrypting PS3 ISOs (included in decryptor/PS3Dec/)

### Installing Rust

**Windows:**
1. Download the Rust installer from [https://rustup.rs/](https://rustup.rs/)
2. Run the installer and follow the prompts
3. Restart your terminal/command prompt

**Linux/macOS:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
## Installation

### From Source

1. **Clone and build**:
   ```bash
   git clone https://github.com/leji-a/ps3-redump-downloader.git
   cd ps3-redump-downloader
   cargo build --release
   ```

2. **Run the application**:
   ```bash
   # Development
   cargo run
   
   # Or use the compiled binary
   ./target/release/ps3-redump-downloader
   ```

### Global Installation

Install globally for easy access from anywhere:

```bash
cargo install --path .
ps3-redump-downloader
```

### Cross-Platform Compilation

**For Windows (from Linux/macOS):**
```bash
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

**For macOS (from Linux):**
```bash
cargo build --release --target x86_64-apple-darwin
```

## PS3Dec Usage

PS3Dec supports multiple key types:
- **3k3y**: Self-contained encrypted images
- **d1**: Raw D1 keys that need processing  
- **key**: Direct decryption keys (what we use)

## Usage

### Basic Usage

1. **Start the application**:
   ```bash
   cargo run
   ```

2. **Search for PS3 games**:
   - Type part of the game title
   - Press Enter to search
   - The search is case-insensitive and supports partial matches

3. **Select a game**:
   - Choose a game from the filtered list by entering its number
   - The download, key fetching, extraction, and decryption will start automatically

4. **Monitor progress**:
   - Download progress is shown with a progress bar
   - Key downloading progress is shown
   - Extraction progress is shown during ZIP extraction
   - Decryption progress is shown during ISO decryption
   - Files are automatically saved to the configured download directory

### Download Location

PS3 games are downloaded to the following location:

- **Default location**: `~/PS3-Games/iso_files/`
  - Linux/macOS: `/home/username/PS3-Games/iso_files/`
  - Windows: `C:\Users\username\PS3-Games\iso_files\`

Keys are cached in:
- **Keys location**: `~/PS3-Games/keys/keys_cache.json`

You can change the download location by modifying the `TMP_FOLDER_NAME` setting in `config.ini`:
- `~/Downloads/PS3-Games` - Downloads folder
- `~/Documents/PS3-Games` - Documents folder
- `/path/to/custom/location` - Custom absolute path

### Example Session

```
PS3 Redump Downloader

Find PS3 title to download (leave empty to exit): god of war
1. God of War III (USA) (4.2 GB)
2. God of War - Ascension (Europe) (8.1 GB)
3. God of War Collection (USA) (6.7 GB)

Enter PS3 title number [1-3]: 1

Selected God of War III (USA)

Downloading decryption key...
Found decryption key for God of War III (USA)

 # PS3 ISO file...
⠲ [00:08:45] [##########################>-------------] 2.8 GB/4.2 GB (5m)

Extracting ZIP file...
⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ Extracting files... 00:02:15

Decrypting PS3 ISO file with key...
⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ Decrypting PS3 ISO... 00:05:30

God of War III (USA) downloaded and decrypted :)
```

## Configuration

The application uses a `config.ini` file for configuration.

### Config File Location

The `config.ini` file should be placed in one of these locations (in order of priority):

#### When Running from Source (Development)
- **Current directory**: Place `config.ini` in the same folder where you run `cargo run`

#### When Running Installed Binary (Global Installation)

**Linux/macOS:**
- **User config**: `~/.config/ps3-redump-downloader/config.ini`
- **System-wide**: `/etc/ps3-redump-downloader/config.ini`
- **Current directory**: `./config.ini` (where you run the binary)

**Windows:**
- **User config**: `%APPDATA%\ps3-redump-downloader\config.ini`
  - Usually: `C:\Users\YourName\AppData\Roaming\ps3-redump-downloader\config.ini`
- **System-wide**: `C:\ProgramData\ps3-redump-downloader\config.ini`
- **Current directory**: `.\config.ini` (where you run the binary)

### config.ini

```ini
[url]
# PS3 Redump sources (requires decryption)
PS3_ISO = https://myrient.erista.me/files/Redump/Sony%20-%20PlayStation%203/
# PS3 decryption keys
PS3_KEYS = https://myrient.erista.me/files/Redump/Sony%20-%20PlayStation%203%20-%20Disc%20Keys%20TXT/

[Download]
# Downloaded PS3 Game list fileName 
LIST_PS3_FILES_JSON_NAME = listPS3Titles.json 

# Download ISO file using navigator (0 = use built-in downloader, 1 = open browser)
EXTERNAL_ISO = 0 

# Retry settings
MAX_RETRIES = 10
DELAY_BETWEEN_RETRIES = 10
TIMEOUT_REQUEST = 1800

[folder]
TMP_FOLDER_NAME = ~/PS3-Games
TMP_ISO_FOLDER_NAME = iso_files

[PS3]
# Path to the PS3Dec binary (built from decryptor/PS3Dec)
DECRYPTOR_PATH = ./decryptor/PS3Dec/build/Release/PS3Dec
# Timeout for decryption process (seconds)
DECRYPTION_TIMEOUT = 300
```

### Configuration Options

| Option | Description | Default |
|--------|-------------|---------|
| `PS3_ISO` | Base URL for PS3 game downloads | Redump PS3 URL |
| `PS3_KEYS` | Base URL for PS3 decryption keys | Redump PS3 Keys URL |
| `EXTERNAL_ISO` | Use browser download instead of built-in downloader | 0 (built-in) |
| `MAX_RETRIES` | Number of retry attempts for failed downloads | 10 |
| `DELAY_BETWEEN_RETRIES` | Seconds to wait between retries | 10 |
| `TIMEOUT_REQUEST` | Request timeout in seconds | 1800 (30 minutes) |
| `TMP_FOLDER_NAME` | Temporary folder name | ~/PS3-Games |
| `TMP_ISO_FOLDER_NAME` | ISO files folder name | iso_files |
| `DECRYPTOR_PATH` | Path to PS3Dec binary | ./decryptor/PS3Dec/build/Release/PS3Dec |
| `DECRYPTION_TIMEOUT` | Decryption timeout in seconds | 300 (5 minutes) |

## File Structure

```
ps3-redump-downloader/
├── src/                    # Source code
├── decryptor/              # PS3Dec C program
│   └── PS3Dec/            # PS3Dec source code
│       ├── src/           # C source files
│       ├── build/         # Build directory
│       │   └── Release/   # Built PS3Dec binary
│       └── CMakeLists.txt # Build configuration
├── config.ini             # Configuration file
├── Cargo.toml            # Rust dependencies
└── README.md             # This file

# Downloaded files (created automatically)
~/PS3-Games/
├── iso_files/             # Downloaded and decrypted ISO files
├── keys/                  # Cached decryption keys
│   └── keys_cache.json   # Keys cache file
└── listPS3Titles.json    # Cached PS3 game list
```

## Troubleshooting

### Common Issues

#### PS3Dec Binary Not Found
- **Problem**: "PS3Dec binary not found" error
- **Solution**: 
  - Build PS3Dec: `cd decryptor/PS3Dec && mkdir -p build && cd build && cmake .. && make`
  - Check the `DECRYPTOR_PATH` setting in `config.ini`
  - Make the binary executable: `chmod +x decryptor/PS3Dec/build/Release/PS3Dec`

#### PS3Dec Build Failures
- **Problem**: CMake or make fails
- **Solution**: 
  - Install build dependencies: `sudo dnf install cmake gcc-c++ make`
  - Ensure you have OpenMP support: `sudo dnf install libomp-devel`
  - On macOS: `brew install libomp`
  - Initialize git submodules: `git submodule update --init --recursive`

#### Decryption Key Not Found
- **Problem**: "Could not find decryption key for game" error
- **Solution**: 
  - The game may not have a key available in the Redump database
  - Try a different game or check if the key exists manually
  - Clear the keys cache: `rm ~/PS3-Games/keys/keys_cache.json`

#### Decryption Timeout
- **Problem**: Decryption process times out
- **Solution**: Increase `DECRYPTION_TIMEOUT` in config.ini (default: 300 seconds)

#### Download Timeout
- **Problem**: Downloads timeout before completing
- **Solution**: Increase `TIMEOUT_REQUEST` in config.ini (default: 1800 seconds)

#### ZIP Extraction Errors
- **Problem**: "Could not find central directory end" error
- **Solution**: The download was incomplete. The app will automatically retry.

#### No Games Found
- **Problem**: Search returns no results
- **Solution**: 
  - Check your internet connection
  - Try different search terms
  - Delete `~/PS3-Games/listPS3Titles.json` to refresh the game list

#### Permission Errors
- **Problem**: Cannot create folders or write files
- **Solution**: 
  - Ensure you have write permissions in the current directory
  - Run as administrator on Windows if needed

#### Config File Not Found
- **Problem**: "Failed to load config" error
- **Solution**: 
  - Ensure `config.ini` is in the correct location (see [Configuration](#configuration))
  - For global installations, create the config directory and copy the file
  - Check file permissions on the config file

## Building from Source

### Development Setup

1. **Install Rust** (see Prerequisites)
2. **Clone the repository**:
   ```bash
   git clone https://github.com/leji-a/ps3-redump-downloader.git
   cd ps3-redump-downloader
   ```

3. **Build PS3Dec**:
   ```bash
   cd decryptor/PS3Dec
   mkdir -p build && cd build
   cmake .. && make
   chmod +x Release/PS3Dec
   ```

4. **Install Rust dependencies**:
   ```bash
   cargo build
   ```

### Dependencies

The application uses these main dependencies:

- `reqwest` - HTTP client for downloads
- `scraper` - HTML parsing for game lists
- `indicatif` - Progress bars and UI
- `tokio` - Async runtime
- `zip` - ZIP file extraction
- `configparser` - Configuration file parsing
- `tokio::process` - Subprocess execution for decryption

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Disclaimer

This tool is for educational and preservation purposes only. Please ensure you comply with your local laws regarding software downloads and usage. Only download games you own or have the right to access.

## Support

If you encounter issues:

1. Check the [Troubleshooting](#troubleshooting) section
2. Search existing [Issues](https://github.com/yourusername/ps3-redump-downloader/issues)
3. Create a new issue with:
   - Your operating system
   - Rust version (`rustc --version`)
   - Error message
   - Steps to reproduce

---

Note: This is a Rust port of the original Python [PS3 Redump downloader](https://github.com/juanpomares/PS3-Redump-downloader).
