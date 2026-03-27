pub mod cache;

use anyhow::{Context, Result};
use tracing::info;

use crate::api::ApiClient;
use crate::api::types::BenchmarkSubmission;
use crate::config::AgentConfig;

use cache::load_cache;

/// Run benchmarks if no valid cache exists, then submit to the server.
pub async fn run_and_submit(client: &ApiClient, config: &AgentConfig) -> Result<()> {
    // Check for cached results
    if let Some(cached) = load_cache(&config.cache_dir).await? {
        info!(
            entries = cached.entries.len(),
            version = %cached.cracker_version,
            "using cached benchmark results"
        );

        let submission = BenchmarkSubmission {
            entries: cached.entries,
            cracker_version: Some(cached.cracker_version),
        };

        client
            .submit_benchmarks(&submission)
            .await
            .context("failed to submit cached benchmarks")?;

        return Ok(());
    }

    // TODO: Run actual hashcat benchmarks (`hashcat -b --machine-readable`)
    // Parse output, build BenchmarkEntry vec, submit, and cache results.
    info!("benchmark execution not yet implemented \u{2014} skipping");

    Ok(())
}
