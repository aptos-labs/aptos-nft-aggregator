use super::{
    config_boilerplate::{DbConfig, IndexerProcessorConfig},
    marketplace_config::{ContractToMarketplaceMap, MarketplaceEventConfigMappings},
    models::NftMarketplaceActivity,
    postgres_utils::{execute_in_chunks, new_db_pool, run_migrations, ArcDbPool},
};
use crate::{processors::processor_status_saver::get_processor_status_saver, schema};
use anyhow::Result;
use aptos_indexer_processor_sdk::{
    aptos_indexer_transaction_stream::{TransactionStream, TransactionStreamConfig},
    builder::ProcessorBuilder,
    common_steps::{
        TransactionStreamStep, VersionTrackerStep, DEFAULT_UPDATE_PROCESSOR_STATUS_SECS,
    },
    traits::{
        processor_trait::ProcessorTrait, AsyncRunType, AsyncStep, IntoRunnableStep, NamedStep,
        Processable,
    },
    types::transaction_context::TransactionContext,
    utils::{errors::ProcessorError, extract::get_entry_function_from_user_request},
};
use aptos_protos::transaction::v1::{transaction::TxnData, Transaction};
use chrono::NaiveDateTime;
use diesel::{pg::Pg, query_builder::QueryFragment};
use std::{collections::HashMap, sync::Arc};
use tonic::async_trait;
use tracing::{debug, error, info};

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

        //  Merge the starting version from config and the latest processed version from the DB
        // let starting_version = get_starting_version(&self.config, self.db_pool.clone()).await?;
        let starting_version = self
            .config
            .transaction_stream_config
            .starting_version
            .unwrap_or(0);

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

        let (event_mappings, contract_to_marketplace_map) = self
            .config
            .nft_marketplace_configs
            .get_event_mappings()
            .unwrap_or_else(|e| {
                error!("Failed to get event mapping: {:?}", e);
                panic!("Failed to get event mapping: {:?}", e);
            });

        let process = ProcessStep::new(
            Arc::new(event_mappings),
            Arc::new(contract_to_marketplace_map),
            self.db_pool.clone(),
        );
        let version_tracker = VersionTrackerStep::new(
            get_processor_status_saver(self.db_pool.clone()),
            DEFAULT_UPDATE_PROCESSOR_STATUS_SECS,
        );

        // Connect processor steps together
        let (_, buffer_receiver) = ProcessorBuilder::new_with_inputless_first_step(
            transaction_stream.into_runnable_step(),
        )
        .connect_to(version_tracker.into_runnable_step(), channel_size)
        .connect_to(process.into_runnable_step(), channel_size)
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

pub struct ProcessStep {
    pub event_mappings: Arc<MarketplaceEventConfigMappings>,
    pub contract_to_marketplace_map: Arc<ContractToMarketplaceMap>,
    pub db_pool: ArcDbPool,
}

impl ProcessStep {
    pub fn new(
        event_mappings: Arc<MarketplaceEventConfigMappings>,
        contract_to_marketplace_map: Arc<ContractToMarketplaceMap>,
        db_pool: ArcDbPool,
    ) -> Self {
        Self {
            event_mappings,
            contract_to_marketplace_map,
            db_pool,
        }
    }
}

#[async_trait]
impl Processable for ProcessStep {
    type Input = Vec<Transaction>;
    type Output = ();
    type RunType = AsyncRunType;

    async fn process(
        &mut self,
        transactions: TransactionContext<Vec<Transaction>>,
    ) -> Result<Option<TransactionContext<()>>, ProcessorError> {
        let txns = transactions.data;

        let mut activities: Vec<NftMarketplaceActivity> = Vec::new();

        for txn in txns {
            let txn_data = txn.txn_data.as_ref().unwrap();
            if let TxnData::User(tx_inner) = txn_data {
                let req = tx_inner
                    .request
                    .as_ref()
                    .expect("Sends is not present in user txn");
                let entry_function_id = get_entry_function_from_user_request(req);

                let events = tx_inner.events.clone();
                let txn_timestamp = txn
                    .timestamp
                    .as_ref()
                    .expect("Transaction timestamp doesn't exist!")
                    .seconds;
                #[allow(deprecated)]
                let txn_timestamp = NaiveDateTime::from_timestamp_opt(txn_timestamp, 0)
                    .expect("Txn Timestamp is invalid!");
                for (event_index, event) in events.iter().enumerate() {
                    if let Some(activity) = NftMarketplaceActivity::from_event(
                        event,
                        txn.version as i64,
                        event_index as i64,
                        txn_timestamp,
                        &entry_function_id,
                        &self.event_mappings,
                        &self.contract_to_marketplace_map,
                    ) {
                        activities.push(activity);
                    }
                }
            }
        }

        // Deduplicate activities
        let mut activities_map: HashMap<(i64, i64), NftMarketplaceActivity> = HashMap::new();
        for activity in activities {
            let key = (activity.txn_version, activity.index);
            activities_map.entry(key).or_insert(activity);
        }
        let mut deduped_activities: Vec<NftMarketplaceActivity> =
            activities_map.into_values().collect();

        // Sort vectors by primary key to avoid deadlocks
        deduped_activities.sort_by(|a, b| {
            a.txn_version
                .cmp(&b.txn_version)
                .then(a.index.cmp(&b.index))
        });

        let nma = execute_in_chunks(
            self.db_pool.clone(),
            insert_nft_marketplace_activities,
            &deduped_activities,
            200,
        )
        .await;

        match nma {
            Ok(_) => {
                debug!(
                    "Successfully stored NftMarketplaceActivities [{:?}, {:?}]",
                    transactions.metadata.start_version, transactions.metadata.end_version
                );
                Ok(Some(TransactionContext {
                    data: (),
                    metadata: transactions.metadata,
                }))
            },
            Err(e) => {
                return Err(ProcessorError::DBStoreError {
                    message: format!(
                        "Failed to store NftMarketplaceActivities versions {} to {}: {:?}",
                        transactions.metadata.start_version, transactions.metadata.end_version, e,
                    ),
                    query: None,
                });
            },
        }
    }
}

impl AsyncStep for ProcessStep {}

impl NamedStep for ProcessStep {
    fn name(&self) -> String {
        "ProcessStep".to_string()
    }
}

pub fn insert_nft_marketplace_activities(
    items_to_insert: Vec<NftMarketplaceActivity>,
) -> (
    impl QueryFragment<Pg> + diesel::query_builder::QueryId + Send,
    Option<&'static str>,
) {
    use crate::schema::nft_marketplace_activities::dsl::*;

    (
        diesel::insert_into(schema::nft_marketplace_activities::table)
            .values(items_to_insert)
            .on_conflict((txn_version, index))
            .do_nothing(),
        None,
    )
}
