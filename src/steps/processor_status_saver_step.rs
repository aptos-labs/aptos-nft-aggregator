use crate::{
    postgres::{
        postgres_utils::{execute_with_better_error, ArcDbPool},
        processor_status::ProcessorStatus,
    },
    schema::processor_status,
};
use anyhow::Result;
use aptos_indexer_processor_sdk::{
    aptos_indexer_transaction_stream::utils::time::parse_timestamp,
    common_steps::ProcessorStatusSaver, types::transaction_context::TransactionContext,
    utils::errors::ProcessorError,
};
use async_trait::async_trait;
use diesel::{query_dsl::methods::FilterDsl, upsert::excluded, ExpressionMethods};

pub fn get_processor_status_saver(conn_pool: ArcDbPool) -> ProcessorStatusSaverEnum {
    ProcessorStatusSaverEnum::Postgres {
        conn_pool,
        processor_name: "nft_marketplace_processor".to_string(),
    }
}

pub enum ProcessorStatusSaverEnum {
    Postgres {
        conn_pool: ArcDbPool,
        processor_name: String,
    },
}

#[async_trait]
impl ProcessorStatusSaver for ProcessorStatusSaverEnum {
    async fn save_processor_status(
        &self,
        last_success_batch: &TransactionContext<()>,
    ) -> Result<(), ProcessorError> {
        self.save_processor_status_with_optional_table_names(last_success_batch)
            .await
    }
}

impl ProcessorStatusSaverEnum {
    async fn save_processor_status_with_optional_table_names(
        &self,
        last_success_batch: &TransactionContext<()>,
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
                            processor_status::last_updated
                                .eq(excluded(processor_status::last_updated)),
                            processor_status::last_transaction_timestamp
                                .eq(excluded(processor_status::last_transaction_timestamp)),
                        ))
                        .filter(
                            processor_status::last_success_version
                                .le(excluded(processor_status::last_success_version)),
                        ),
                )
                .await?;

                Ok(())
            },
        }
    }
}
