use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::api::types::BenchmarkEntry;

/// Cached benchmark results, keyed by hashcat version.
#[derive(Debug, Serialize, Deserialize)]
pub struct BenchmarkCache {
    pub cracker_version: String,
    pub entries: Vec<BenchmarkEntry>,
}

/// Load cached benchmark results from disk.
pub async fn load_cache(cache_dir: &Path) -> Result<Option<BenchmarkCache>> {
    let path = cache_dir.join("benchmarks.json");
    if !path.exists() {
        return Ok(None);
    }

    let data = fs::read_to_string(&path)
        .await
        .context("failed to read benchmark cache")?;

    let cache: BenchmarkCache =
        serde_json::from_str(&data).context("failed to parse benchmark cache")?;

    Ok(Some(cache))
}

/// Save benchmark results to disk for future sessions.
pub async fn save_cache(cache_dir: &Path, cache: &BenchmarkCache) -> Result<()> {
    fs::create_dir_all(cache_dir)
        .await
        .context("failed to create cache directory")?;

    let path = cache_dir.join("benchmarks.json");
    let temp_path = cache_dir.join(".benchmarks.json.tmp");

    let data =
        serde_json::to_string_pretty(cache).context("failed to serialize benchmark cache")?;

    // Atomic write: temp file then rename
    fs::write(&temp_path, &data)
        .await
        .context("failed to write benchmark temp file")?;

    fs::rename(&temp_path, &path)
        .await
        .context("failed to rename benchmark cache file")?;

    Ok(())
}
