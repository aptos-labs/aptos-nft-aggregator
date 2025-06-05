use crate::{
    config::{DbConfig, IndexerProcessorConfig},
    steps::{
        db_writing_step::DBWritingStep,
        processor_status_saver_step::{
            get_end_version, get_starting_version, PostgresProcessorStatusSaver,
        },
        reduction_step::NFTReductionStep,
        remapper_step::ProcessStep,
    },
    MIGRATIONS,
};
use anyhow::Result;
use aptos_indexer_processor_sdk::{
    aptos_indexer_transaction_stream::TransactionStreamConfig,
    builder::ProcessorBuilder,
    common_steps::{
        TransactionStreamStep, VersionTrackerStep, DEFAULT_UPDATE_PROCESSOR_STATUS_SECS,
    },
    postgres::utils::{
        checkpoint::PostgresChainIdChecker,
        database::{new_db_pool, run_migrations, ArcDbPool},
    },
    traits::{processor_trait::ProcessorTrait, IntoRunnableStep},
    utils::chain_id_check::check_or_update_chain_id,
};
use tracing::{debug, info};

pub struct Processor {
    pub config: IndexerProcessorConfig,
    pub db_pool: ArcDbPool,
}

impl Processor {
    pub async fn new(config: IndexerProcessorConfig) -> Result<Self> {
        match config.db_config {
            DbConfig::PostgresConfig(ref postgres_config) => {
                let conn_pool = new_db_pool(
                    &postgres_config.connection_string,
                    Some(postgres_config.db_pool_size),
                )
                .await
                .map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to create connection pool for PostgresConfig: {:?}",
                        e
                    )
                })?;

                Ok(Self {
                    config,
                    db_pool: conn_pool,
                })
            },
        }
    }
}

#[async_trait::async_trait]
impl ProcessorTrait for Processor {
    fn name(&self) -> &'static str {
        self.config.nft_marketplace_config.get_name()
    }

    async fn run_processor(&self) -> Result<()> {
        // Run migrations
        let DbConfig::PostgresConfig(ref postgres_config) = self.config.db_config;
        run_migrations(
            postgres_config.connection_string.clone(),
            self.db_pool.clone(),
            MIGRATIONS,
        )
        .await;

        // Merge the starting version from config and the latest processed version from the DB
        let (starting_version, ending_version) = (
            get_starting_version(&self.config, self.db_pool.clone()).await?,
            get_end_version(&self.config, self.db_pool.clone()).await?,
        );

        // Check and update the ledger chain id to ensure we're indexing the correct chain
        check_or_update_chain_id(
            &self.config.transaction_stream_config,
            &PostgresChainIdChecker::new(self.db_pool.clone()),
        )
        .await?;

        let channel_size = 100;

        // Define processor steps
        let transaction_stream = TransactionStreamStep::new(TransactionStreamConfig {
            starting_version,
            request_ending_version: ending_version,
            ..self.config.transaction_stream_config.clone()
        })
        .await?;

        let nft_marketplace_config = self.config.nft_marketplace_config.clone();

        let process = ProcessStep::new(nft_marketplace_config.clone())?;
        let reduction_step = NFTReductionStep::new();
        let db_writing = DBWritingStep::new(self.db_pool.clone());
        let version_tracker = VersionTrackerStep::new(
            PostgresProcessorStatusSaver::new(self.config.clone(), self.db_pool.clone()),
            DEFAULT_UPDATE_PROCESSOR_STATUS_SECS,
        );

        // Connect processor steps together
        let (_, buffer_receiver) = ProcessorBuilder::new_with_inputless_first_step(
            transaction_stream.into_runnable_step(),
        )
        .connect_to(process.into_runnable_step(), channel_size)
        .connect_to(reduction_step.into_runnable_step(), channel_size)
        .connect_to(db_writing.into_runnable_step(), channel_size)
        .connect_to(version_tracker.into_runnable_step(), channel_size)
        .end_and_return_output_receiver(channel_size);

        // (Optional) Parse the results
        loop {
            match buffer_receiver.recv().await {
                Ok(txn_context) => {
                    debug!(
                        "Finished processing events from versions [{:?}, {:?}]",
                        txn_context.metadata.start_version, txn_context.metadata.end_version,
                    );
                },
                Err(e) => {
                    info!("No more transactions in channel: {:?}", e);
                    break Ok(());
                },
            }
        }
    }
}
