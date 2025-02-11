use crate::{
    config::indexer_processor_config::IndexerProcessorConfig,
    db::processor_status::ProcessorStatus,
    schema::processor_status,
    utils::database::{execute_with_better_error, ArcDbPool},
};
use anyhow::Result;
use aptos_indexer_processor_sdk::{
    common_steps::ProcessorStatusSaver,
    types::transaction_context::TransactionContext,
    utils::{errors::ProcessorError, time::parse_timestamp},
};
use async_trait::async_trait;
use diesel::{upsert::excluded, ExpressionMethods};

pub fn get_processor_status_saver(
    conn_pool: ArcDbPool,
    config: IndexerProcessorConfig,
) -> ProcessorStatusSaverEnum {
    match config.backfill_config {
        Some(backfill_config) => {
            let txn_stream_cfg = config.transaction_stream_config;
            let backfill_start_version = txn_stream_cfg.starting_version;
            let backfill_end_version = txn_stream_cfg.request_ending_version;
            let backfill_alias = backfill_config.backfill_alias.clone();
            ProcessorStatusSaverEnum::Backfill {
                conn_pool,
                backfill_alias,
                backfill_start_version,
                backfill_end_version,
            }
        },
        None => {
            let processor_name = config.processor_config.name().to_string();
            ProcessorStatusSaverEnum::Postgres {
                conn_pool,
                processor_name,
            }
        },
    }
}

pub enum ProcessorStatusSaverEnum {
    Postgres {
        conn_pool: ArcDbPool,
        processor_name: String,
    },
    Backfill {
        conn_pool: ArcDbPool,
        backfill_alias: String,
        backfill_start_version: Option<u64>,
        backfill_end_version: Option<u64>,
    },
}

#[async_trait]
impl ProcessorStatusSaver for ProcessorStatusSaverEnum {
    async fn save_processor_status(
        &self,
        last_success_batch: &TransactionContext<()>,
    ) -> Result<(), ProcessorError> {
        self.save_processor_status_with_optional_table_names(last_success_batch, None)
            .await
    }
}

impl ProcessorStatusSaverEnum {
    async fn save_processor_status_with_optional_table_names(
        &self,
        last_success_batch: &TransactionContext<()>,
        _table_name: Option<String>,
    ) -> Result<(), ProcessorError> {
        let end_timestamp = last_success_batch
            .metadata
            .end_transaction_timestamp
            .as_ref()
            .map(|t| parse_timestamp(t, last_success_batch.metadata.end_version as i64))
            .map(|t| t.naive_utc());
        match self {
            ProcessorStatusSaverEnum::Postgres {
                conn_pool,
                processor_name,
            } => {
                let status = ProcessorStatus {
                    processor: processor_name.clone(),
                    last_success_version: last_success_batch.metadata.end_version as i64,
                    last_transaction_timestamp: end_timestamp,
                };

                // Save regular processor status to the database
                execute_with_better_error(
                    conn_pool.clone(),
                    diesel::insert_into(processor_status::table)
                        .values(&status)
                        .on_conflict(processor_status::processor)
                        .do_update()
                        .set((
                            processor_status::last_success_version
                                .eq(excluded(processor_status::last_success_version)),
                            processor_status::last_updated.eq(excluded(processor_status::last_updated)),
                            processor_status::last_transaction_timestamp
                                .eq(excluded(processor_status::last_transaction_timestamp)),
                        )),
                    Some(" WHERE processor_status.last_success_version <= EXCLUDED.last_success_version "),
                )
                    .await?;

                Ok(())
            },
            _ => {
                // Not implemented
                Ok(())
            },
        }
    }
}
