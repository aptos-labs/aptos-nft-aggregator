use crate::{
    config::{DbConfig, IndexerProcessorConfig},
    postgres::postgres_utils::{new_db_pool, run_migrations, ArcDbPool},
    steps::{
        db_writing_step::DBWritingStep, processor_status_saver_step::get_processor_status_saver,
        reduction_step::NFTReductionStep, remapper_step::ProcessStep,
    },
};
use anyhow::Result;
use aptos_indexer_processor_sdk::{
    aptos_indexer_transaction_stream::{TransactionStream, TransactionStreamConfig},
    builder::ProcessorBuilder,
    common_steps::{
        TransactionStreamStep, VersionTrackerStep, DEFAULT_UPDATE_PROCESSOR_STATUS_SECS,
    },
    traits::{processor_trait::ProcessorTrait, IntoRunnableStep},
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
        "nft_marketplace_processor"
    }

    async fn run_processor(&self) -> Result<()> {
        // Run migrations
        let DbConfig::PostgresConfig(ref postgres_config) = self.config.db_config;

        run_migrations(
            postgres_config.connection_string.clone(),
            self.db_pool.clone(),
        )
        .await;

        // Merge the starting version from config and the latest processed version from the DB
        // let starting_version = get_starting_version(&self.config, self.db_pool.clone()).await?;
        let starting_version = self
            .config
            .transaction_stream_config
            .starting_version
            .unwrap();

        // Check and update the ledger chain id to ensure we're indexing the correct chain
        let _grpc_chain_id = TransactionStream::new(self.config.transaction_stream_config.clone())
            .await?
            .get_chain_id()
            .await?;
        // check_or_update_chain_id(grpc_chain_id as i64, self.db_pool.clone()).await?;

        let channel_size = self.config.channel_size as usize;

        // Define processor steps
        let transaction_stream = TransactionStreamStep::new(TransactionStreamConfig {
            starting_version: Some(starting_version),
            ..self.config.transaction_stream_config.clone()
        })
        .await?;

        let nft_marketplace_config = self.config.nft_marketplace_config.clone();

        let process = ProcessStep::new(nft_marketplace_config.clone())?;
        let reduction_step = NFTReductionStep::new();
        let db_writing = DBWritingStep::new(self.db_pool.clone());
        let version_tracker = VersionTrackerStep::new(
            get_processor_status_saver(self.db_pool.clone()),
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
