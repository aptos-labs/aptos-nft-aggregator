use super::database::ArcDbPool;
use crate::{
    config::indexer_processor_config::IndexerProcessorConfig,
    db::processor_status::ProcessorStatusQuery,
};
use anyhow::{Context, Result};

/// Get the appropriate starting version for the processor.
///
/// If it is a regular processor, this will return the higher of the checkpointed version,
/// or `staring_version` from the config, or 0 if not set.
///
/// If this is a backfill processor and threre is an in-progress backfill, this will return
/// the checkpointed version + 1.
///
/// If this is a backfill processor and there is not an in-progress backfill (i.e., no checkpoint or
/// backfill status is COMPLETE), this will return `starting_version` from the config, or 0 if not set.
pub async fn get_starting_version(
    indexer_processor_config: &IndexerProcessorConfig,
    conn_pool: ArcDbPool,
) -> Result<u64> {
    // Check if there's a checkpoint in the appropriate processor status table.
    let latest_processed_version =
        get_starting_version_from_db(indexer_processor_config, conn_pool)
            .await
            .context("Failed to get latest processed version from DB")?;

    // If nothing checkpointed, return the `starting_version` from the config, or 0 if not set.
    Ok(latest_processed_version.unwrap_or(
        indexer_processor_config
            .transaction_stream_config
            .starting_version
            .unwrap_or(0),
    ))
}

async fn get_starting_version_from_db(
    indexer_processor_config: &IndexerProcessorConfig,
    conn_pool: ArcDbPool,
) -> Result<Option<u64>> {
    let mut conn = conn_pool.get().await?;

    let status = ProcessorStatusQuery::get_by_processor(
        indexer_processor_config.processor_config.name(),
        &mut conn,
    )
    .await
    .context("Failed to query processor_status table.")?;

    // Return None if there is no checkpoint. Otherwise,
    // return the higher of the checkpointed version + 1 and `starting_version`.
    Ok(status.map(|status| {
        std::cmp::max(
            status.last_success_version as u64,
            indexer_processor_config
                .transaction_stream_config
                .starting_version
                .unwrap_or(0),
        )
    }))
}
