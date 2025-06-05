use crate::{
    config::{
        processor_mode::{BackfillConfig, BootStrapConfig, ProcessorMode, TestingConfig},
        IndexerProcessorConfig,
    },
    postgres::backfill_processor_status::{
        BackfillProcessorStatus, BackfillProcessorStatusQuery, BackfillStatus,
    },
    schema::backfill_processor_status,
};
use anyhow::Result;
use aptos_indexer_processor_sdk::{
    aptos_indexer_transaction_stream::utils::time::parse_timestamp,
    common_steps::ProcessorStatusSaver,
    postgres::{
        models::processor_status::{ProcessorStatus, ProcessorStatusQuery},
        processor_metadata_schema::processor_metadata::processor_status,
        utils::database::{execute_with_better_error, ArcDbPool},
    },
    types::transaction_context::TransactionContext,
    utils::errors::ProcessorError,
};
use async_trait::async_trait;
use diesel::{query_dsl::methods::FilterDsl, upsert::excluded, ExpressionMethods};

/// A trait implementation of ProcessorStatusSaver for Postgres.
pub struct PostgresProcessorStatusSaver {
    pub config: IndexerProcessorConfig,
    pub db_pool: ArcDbPool,
}

impl PostgresProcessorStatusSaver {
    pub fn new(config: IndexerProcessorConfig, db_pool: ArcDbPool) -> Self {
        Self { config, db_pool }
    }
}

#[async_trait]
impl ProcessorStatusSaver for PostgresProcessorStatusSaver {
    async fn save_processor_status(
        &self,
        last_success_batch: &TransactionContext<()>,
    ) -> Result<(), ProcessorError> {
        save_processor_status(
            &self.config.nft_marketplace_config.name,
            self.config.processor_mode.clone(),
            last_success_batch,
            self.db_pool.clone(),
        )
        .await
    }
}

pub async fn save_processor_status(
    processor_id: &str,
    processor_mode: ProcessorMode,
    last_success_batch: &TransactionContext<()>,
    db_pool: ArcDbPool,
) -> Result<(), ProcessorError> {
    let last_success_version = last_success_batch.metadata.end_version as i64;
    let last_transaction_timestamp = last_success_batch
        .metadata
        .end_transaction_timestamp
        .as_ref()
        .map(|t| parse_timestamp(t, last_success_batch.metadata.end_version as i64))
        .map(|t| t.naive_utc());
    let status = ProcessorStatus {
        processor: processor_id.to_string(),
        last_success_version,
        last_transaction_timestamp,
    };

    match processor_mode {
        ProcessorMode::Default(_) => {
            // Save regular processor status to the database
            execute_with_better_error(
                db_pool.clone(),
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
                    ))
                    .filter(
                        processor_status::last_success_version
                            .le(excluded(processor_status::last_success_version)),
                    ),
            )
            .await?;
        },
        ProcessorMode::Backfill(BackfillConfig {
            backfill_id,
            initial_starting_version,
            ending_version,
            overwrite_checkpoint,
        }) => {
            let backfill_alias = format!("{processor_id}_{backfill_id}");
            let backfill_status = if ending_version.is_some()
                && last_success_version >= ending_version.unwrap() as i64
            {
                BackfillStatus::Complete
            } else {
                BackfillStatus::InProgress
            };
            let status = BackfillProcessorStatus {
                backfill_alias,
                backfill_status,
                last_success_version,
                last_transaction_timestamp,
                backfill_start_version: initial_starting_version as i64,
                backfill_end_version: ending_version.map(|v| v as i64),
            };

            let query = diesel::insert_into(backfill_processor_status::table)
                .values(&status)
                .on_conflict(backfill_processor_status::backfill_alias)
                .do_update()
                .set((
                    backfill_processor_status::backfill_status
                        .eq(excluded(backfill_processor_status::backfill_status)),
                    backfill_processor_status::last_success_version
                        .eq(excluded(backfill_processor_status::last_success_version)),
                    backfill_processor_status::last_updated
                        .eq(excluded(backfill_processor_status::last_updated)),
                    backfill_processor_status::last_transaction_timestamp.eq(excluded(
                        backfill_processor_status::last_transaction_timestamp,
                    )),
                    backfill_processor_status::backfill_start_version
                        .eq(excluded(backfill_processor_status::backfill_start_version)),
                    backfill_processor_status::backfill_end_version
                        .eq(excluded(backfill_processor_status::backfill_end_version)),
                ));

            // If overwrite_checkpoint is true, then always update the backfill status.
            if overwrite_checkpoint {
                execute_with_better_error(db_pool.clone(), query).await?;
            } else {
                execute_with_better_error(
                    db_pool.clone(),
                    query.filter(
                        backfill_processor_status::last_success_version
                            .le(excluded(backfill_processor_status::last_success_version)),
                    ),
                )
                .await?;
            }
        },
        ProcessorMode::Testing(_) => {
            // In testing mode, the last success version is not stored.
        },
    }
    Ok(())
}

pub async fn get_starting_version(
    config: &IndexerProcessorConfig,
    db_pool: ArcDbPool,
) -> Result<Option<u64>, ProcessorError> {
    let processor_name = &config.nft_marketplace_config.name;
    let mut conn = db_pool
        .get()
        .await
        .map_err(|e| ProcessorError::ProcessError {
            message: format!("Failed to get database connection. {e:?}"),
        })?;

    match &config.processor_mode {
        ProcessorMode::Default(BootStrapConfig {
            initial_starting_version,
        }) => {
            let status = ProcessorStatusQuery::get_by_processor(processor_name, &mut conn)
                .await
                .map_err(|e| ProcessorError::ProcessError {
                    message: format!("Failed to query processor_status table. {e:?}"),
                })?;

            // If there's no last success version saved, start with the version from config
            Ok(Some(status.map_or(*initial_starting_version, |status| {
                std::cmp::max(
                    status.last_success_version as u64,
                    *initial_starting_version,
                )
            })))
        },
        ProcessorMode::Backfill(BackfillConfig {
            backfill_id,
            initial_starting_version,
            ending_version,
            overwrite_checkpoint,
        }) => {
            let backfill_status_option: Option<BackfillProcessorStatusQuery> =
                BackfillProcessorStatusQuery::get_by_processor(
                    processor_name,
                    backfill_id,
                    &mut conn,
                )
                .await
                .map_err(|e| ProcessorError::ProcessError {
                    message: format!("Failed to query backfill_processor_status table. {e:?}"),
                })?;

            // Return None if there is no checkpoint, if the backfill is old (complete), or if overwrite_checkpoint is true.
            // Otherwise, return the checkpointed version + 1.
            if let Some(status) = backfill_status_option {
                // If the backfill is complete and overwrite_checkpoint is false, return the ending_version to end the backfill.
                if status.backfill_status == BackfillStatus::Complete && !overwrite_checkpoint {
                    return Ok(*ending_version);
                }
                // If status is Complete or overwrite_checkpoint is true, this is the start of a new backfill job.
                if *overwrite_checkpoint {
                    let backfill_alias = status.backfill_alias.clone();

                    // If the ending_version is provided, use it. If not, compute the ending_version from processor_status.last_success_version.
                    let backfill_end_version = match *ending_version {
                        Some(e) => Some(e as i64),
                        None => get_end_version(config, db_pool.clone())
                            .await?
                            .map(|v| v as i64),
                    };
                    let status = BackfillProcessorStatus {
                        backfill_alias,
                        backfill_status: BackfillStatus::InProgress,
                        last_success_version: 0,
                        last_transaction_timestamp: None,
                        backfill_start_version: *initial_starting_version as i64,
                        backfill_end_version,
                    };
                    execute_with_better_error(
                        db_pool.clone(),
                        diesel::insert_into(backfill_processor_status::table)
                            .values(&status)
                            .on_conflict(backfill_processor_status::backfill_alias)
                            .do_update()
                            .set((
                                backfill_processor_status::backfill_status
                                    .eq(excluded(backfill_processor_status::backfill_status)),
                                backfill_processor_status::last_success_version
                                    .eq(excluded(backfill_processor_status::last_success_version)),
                                backfill_processor_status::last_updated
                                    .eq(excluded(backfill_processor_status::last_updated)),
                                backfill_processor_status::last_transaction_timestamp.eq(excluded(
                                    backfill_processor_status::last_transaction_timestamp,
                                )),
                                backfill_processor_status::backfill_start_version.eq(excluded(
                                    backfill_processor_status::backfill_start_version,
                                )),
                                backfill_processor_status::backfill_end_version
                                    .eq(excluded(backfill_processor_status::backfill_end_version)),
                            )),
                    )
                    .await?;
                    return Ok(Some(*initial_starting_version));
                }

                // `backfill_config.initial_starting_version` is NOT respected.
                // Return the last success version + 1.
                let starting_version = status.last_success_version as u64 + 1;
                log_ascii_warning(starting_version);
                Ok(Some(starting_version))
            } else {
                Ok(Some(*initial_starting_version))
            }
        },
        ProcessorMode::Testing(TestingConfig {
            override_starting_version,
            ..
        }) => {
            // Always start from the override_starting_version.
            Ok(Some(*override_starting_version))
        },
    }
}

pub async fn get_end_version(
    config: &IndexerProcessorConfig,
    db_pool: ArcDbPool,
) -> Result<Option<u64>, ProcessorError> {
    let processor_name = &config.nft_marketplace_config.get_name();
    let processor_mode = &config.processor_mode;
    match processor_mode {
        ProcessorMode::Default(_) => Ok(None),
        ProcessorMode::Backfill(BackfillConfig { ending_version, .. }) => {
            match ending_version {
                Some(ending_version) => Ok(Some(*ending_version)),
                None => {
                    // If there is no ending version in the config, use the processor_status.last_success_version
                    let mut conn =
                        db_pool
                            .get()
                            .await
                            .map_err(|e| ProcessorError::ProcessError {
                                message: format!("Failed to get database connection. {e:?}"),
                            })?;
                    let status = ProcessorStatusQuery::get_by_processor(processor_name, &mut conn)
                        .await
                        .map_err(|e| ProcessorError::ProcessError {
                            message: format!("Failed to query processor_status table. {e:?}"),
                        })?;
                    Ok(status.map(|status| status.last_success_version as u64))
                },
            }
        },
        ProcessorMode::Testing(TestingConfig {
            override_starting_version,
            ending_version,
        }) => {
            // If no ending version is provided, use the override_starting_version so testing mode only processes 1 transaction at a time.
            Ok(Some(ending_version.unwrap_or(*override_starting_version)))
        },
    }
}

pub fn log_ascii_warning(version: u64) {
    println!(
        r#"
 ██╗    ██╗ █████╗ ██████╗ ███╗   ██╗██╗███╗   ██╗ ██████╗ ██╗
 ██║    ██║██╔══██╗██╔══██╗████╗  ██║██║████╗  ██║██╔════╝ ██║
 ██║ █╗ ██║███████║██████╔╝██╔██╗ ██║██║██╔██╗ ██║██║  ███╗██║
 ██║███╗██║██╔══██║██╔══██╗██║╚██╗██║██║██║╚██╗██║██║   ██║╚═╝
 ╚███╔███╔╝██║  ██║██║  ██║██║ ╚████║██║██║ ╚████║╚██████╔╝██╗
  ╚══╝╚══╝ ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝╚═╝╚═╝  ╚═══╝ ╚═════╝ ╚═╝
                                                               
=================================================================
   This backfill job is resuming progress at version {version}
=================================================================
"#
    );
}
