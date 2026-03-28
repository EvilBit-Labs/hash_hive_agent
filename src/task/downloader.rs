use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use futures::StreamExt;
use sha2::{Digest, Sha256};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, info};

/// Download a file from `url` into `dest_dir`, returning the final file path.
///
/// Uses streaming to avoid buffering 100GB+ files in memory.
/// Writes to a temporary file first, then atomically renames on success.
#[allow(clippy::arithmetic_side_effects, clippy::as_conversions)]
pub async fn download_file(
    client: &reqwest::Client,
    url: &str,
    dest_dir: &Path,
) -> Result<PathBuf> {
    let parsed: url::Url = url.parse().context("invalid download URL")?;
    let filename = parsed
        .path_segments()
        .and_then(|mut s| s.next_back())
        .unwrap_or("download");

    let dest_path = dest_dir.join(filename);
    let temp_path = dest_dir.join(format!(".{filename}.tmp"));

    fs::create_dir_all(dest_dir)
        .await
        .context("failed to create download directory")?;

    info!(url = %url, dest = %dest_path.display(), "downloading resource");

    let resp = client
        .get(url)
        .send()
        .await
        .context("download request failed")?;

    if !resp.status().is_success() {
        bail!("download failed with status {}", resp.status());
    }

    let mut file = fs::File::create(&temp_path)
        .await
        .context("failed to create temp file")?;

    let mut stream = resp.bytes_stream();
    let mut bytes_written: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let data = chunk.context("error reading download stream")?;
        file.write_all(&data)
            .await
            .context("error writing to temp file")?;
        bytes_written += data.len() as u64;
    }

    file.flush().await.context("failed to flush temp file")?;
    drop(file);

    // Atomic rename from temp to final path
    fs::rename(&temp_path, &dest_path)
        .await
        .context("failed to rename temp file to final path")?;

    debug!(bytes = bytes_written, path = %dest_path.display(), "download complete");
    Ok(dest_path)
}

/// Verify the SHA-256 checksum of a file.
///
/// Streams the file in chunks to avoid buffering 100GB+ files in memory.
pub async fn verify_checksum(path: &Path, expected_hex: &str) -> Result<bool> {
    let file = fs::File::open(path)
        .await
        .context("failed to open file for checksum")?;

    let mut reader = tokio::io::BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buf = vec![0_u8; 64 * 1024];

    loop {
        let n = reader
            .read(&mut buf)
            .await
            .context("failed to read file for checksum")?;
        if n == 0 {
            break;
        }
        if let Some(chunk) = buf.get(..n) {
            hasher.update(chunk);
        }
    }

    let result = hasher.finalize();
    let actual_hex = result
        .iter()
        .fold(String::with_capacity(64), |mut acc, byte| {
            use std::fmt::Write;
            // Writing to a String is infallible (only OOM would fail, which aborts).
            #[allow(clippy::let_underscore_must_use)]
            let _ = write!(acc, "{byte:02x}");
            acc
        });

    Ok(actual_hex == expected_hex)
}
