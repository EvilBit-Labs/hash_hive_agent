use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use futures::StreamExt;
use sha2::{Digest, Sha256};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info};

/// Download a file from `url` into `dest_dir`, returning the final file path.
///
/// Uses streaming to avoid buffering 100GB+ files in memory.
/// Writes to a temporary file first, then atomically renames on success.
pub async fn download_file(
    client: &reqwest::Client,
    url: &str,
    dest_dir: &Path,
) -> Result<PathBuf> {
    let parsed: url::Url = url.parse().context("invalid download URL")?;
    let filename = parsed
        .path_segments()
        .and_then(|s| s.last())
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
        let chunk = chunk.context("error reading download stream")?;
        file.write_all(&chunk)
            .await
            .context("error writing to temp file")?;
        bytes_written += chunk.len() as u64;
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
pub async fn verify_checksum(path: &Path, expected_hex: &str) -> Result<bool> {
    let data = fs::read(path)
        .await
        .context("failed to read file for checksum")?;

    let mut hasher = Sha256::new();
    hasher.update(&data);
    let result = hasher.finalize();
    let actual_hex = result
        .iter()
        .fold(String::with_capacity(64), |mut acc, byte| {
            use std::fmt::Write;
            let _ = write!(acc, "{byte:02x}");
            acc
        });

    Ok(actual_hex == expected_hex)
}
