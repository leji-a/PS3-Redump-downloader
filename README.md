# PS3 Redump Downloader

A fast CLI tool to search, download, and decrypt PlayStation 3 ISOs from Redump.

- Search and filter PS3 games
- Auto-downloads and manages decryption keys
- Progress bars for download, extraction, and decryption
- Resumes interrupted downloads

## Quick Start

1. **Install Rust:** https://rustup.rs/
2. **Build:**
   ```bash
   git clone https://github.com/leji-a/ps3-redump-downloader.git
   cd ps3-redump-downloader
   cargo build --release
   ```
3. **Get the Decryptor:**
   - Download and build [PS3Dec](https://github.com/al3xtjames/PS3Dec/) separately.
   - Set its path in `config.ini` (see below).

4. **Run:**
   ```bash
   ./target/release/ps3-redump-downloader
   ```

## Configuration

Edit `config.ini` (create if missing):

```ini
[url]
PS3_ISO = https://myrient.erista.me/files/Redump/Sony%20-%20PlayStation%203/
PS3_KEYS = https://myrient.erista.me/files/Redump/Sony%20-%20PlayStation%203%20-%20Disc%20Keys%20TXT/

[folder]
TMP_FOLDER_NAME = ~/PS3-Games
TMP_ISO_FOLDER_NAME = iso_files

[PS3]
DECRYPTOR_PATH = /path/to/PS3Dec  # Set to your PS3Dec binary
```
---

## Notes
- The decryptor (PS3Dec) is **not included**. Download/build it from [here](https://github.com/al3xtjames/PS3Dec/).

---

This project is a Rust port of the original Python [PS3 Redump Downloader](https://github.com/juanpomares/PS3-Redump-downloader).
