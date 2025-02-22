use super::{
    config_boilerplate::{DbConfig, IndexerProcessorConfig},
    marketplace_config::{ContractToMarketplaceMap, MarketplaceEventConfigMappings},
    models::{
        CurrentNftMarketplaceBid, CurrentNftMarketplaceCollectionBid, CurrentNftMarketplaceListing,
        NftMarketplaceActivity, NftMarketplaceBid, NftMarketplaceCollectionBid,
        NftMarketplaceListing,
    },
    postgres_utils::{execute_in_chunks, new_db_pool, run_migrations, ArcDbPool},
};
use crate::schema;
use anyhow::Result;
use aptos_indexer_processor_sdk::{
    aptos_indexer_transaction_stream::{TransactionStream, TransactionStreamConfig},
    builder::ProcessorBuilder,
    common_steps::TransactionStreamStep,
    traits::{
        processor_trait::ProcessorTrait, AsyncRunType, AsyncStep, IntoRunnableStep, NamedStep,
        Processable,
    },
    types::transaction_context::TransactionContext,
    utils::{errors::ProcessorError, extract::get_entry_function_from_user_request},
};
use aptos_protos::transaction::v1::{transaction::TxnData, Transaction};
use chrono::NaiveDateTime;
use diesel::{
    pg::{upsert::excluded, Pg},
    query_builder::QueryFragment,
    ExpressionMethods,
};
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
        // let version_tracker = VersionTrackerStep::new(
        //     get_processor_status_saver(self.db_pool.clone(), self.config.clone()),
        //     DEFAULT_UPDATE_PROCESSOR_STATUS_SECS,
        // );

        // Connect processor steps together
        let (_, buffer_receiver) = ProcessorBuilder::new_with_inputless_first_step(
            transaction_stream.into_runnable_step(),
        )
        // .connect_to(version_tracker.into_runnable_step(), channel_size)
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

        let mut token_bids: Vec<NftMarketplaceBid> = Vec::new();
        let mut current_token_bids: Vec<CurrentNftMarketplaceBid> = Vec::new();

        let mut listings: Vec<NftMarketplaceListing> = Vec::new();
        let mut current_listings: Vec<CurrentNftMarketplaceListing> = Vec::new();

        let mut collection_bids: Vec<NftMarketplaceCollectionBid> = Vec::new();
        let mut current_collection_bids: Vec<CurrentNftMarketplaceCollectionBid> = Vec::new();

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
                        // println!("Activity: {:#?}", activity);

                        match activity.standard_event_type.as_str() {
                            "place_listing" => {
                                let (listing, current_listing) =
                                    NftMarketplaceListing::from_activity_to_current(
                                        &activity, false,
                                    );
                                listings.push(listing);
                                current_listings.push(current_listing);
                            },
                            "cancel_listing" => {
                                let (listing, current_listing) =
                                    NftMarketplaceListing::from_activity_to_current(
                                        &activity, true,
                                    );
                                listings.push(listing);
                                current_listings.push(current_listing);
                            },
                            "fill_listing" => {
                                let (listing, current_listing) =
                                    NftMarketplaceListing::from_activity_to_current(
                                        &activity, false,
                                    );
                                listings.push(listing);
                                current_listings.push(current_listing);
                            },
                            "place_offer" => {
                                let (bid, current_bid) =
                                    NftMarketplaceBid::from_activity_to_current(&activity, false);
                                token_bids.push(bid);
                                current_token_bids.push(current_bid);
                            },
                            "cancel_offer" => {
                                let (bid, current_bid) =
                                    NftMarketplaceBid::from_activity_to_current(&activity, true);
                                token_bids.push(bid);
                                current_token_bids.push(current_bid);
                            },
                            "fill_offer" => {
                                let (bid, current_bid) =
                                    NftMarketplaceBid::from_activity_to_current(&activity, false);
                                token_bids.push(bid);
                                current_token_bids.push(current_bid);
                            },
                            "place_collection_offer" => {
                                let (bid, current_bid) =
                                    NftMarketplaceCollectionBid::from_activity_to_current(
                                        &activity, false,
                                    );
                                collection_bids.push(bid);
                                current_collection_bids.push(current_bid);
                            },
                            "cancel_collection_offer" => {
                                let (bid, current_bid) =
                                    NftMarketplaceCollectionBid::from_activity_to_current(
                                        &activity, true,
                                    );
                                collection_bids.push(bid);
                                current_collection_bids.push(current_bid);
                            },
                            "fill_collection_offer" => {
                                let (collection_bid, current_collection_bid) =
                                    NftMarketplaceCollectionBid::from_activity_to_current(
                                        &activity, false,
                                    );
                                collection_bids.push(collection_bid);
                                current_collection_bids.push(current_collection_bid);
                            },
                            _ => {
                                return Err(ProcessorError::ProcessError {
                                    message: format!(
                                        "Unknown event type: {:?}",
                                        activity.standard_event_type
                                    ),
                                });
                            },
                        }
                        activities.push(activity);
                    }
                }
            }
        }

        // Deduplicate listings
        let mut listings_map: HashMap<(Option<String>, Option<String>), NftMarketplaceListing> =
            HashMap::new();
        for listing in listings {
            let key = (listing.listing_id.clone(), listing.token_data_id.clone());
            listings_map.entry(key).or_insert(listing);
        }
        let mut deduped_listings: Vec<NftMarketplaceListing> = listings_map.into_values().collect();

        // Deduplicate bids
        let mut bids_map: HashMap<(Option<String>, String), NftMarketplaceBid> = HashMap::new();
        for bid in token_bids {
            let key = (bid.offer_id.clone(), bid.token_data_id.clone());
            bids_map.entry(key).or_insert(bid);
        }
        let mut deduped_bids: Vec<NftMarketplaceBid> = bids_map.into_values().collect();

        // Deduplicate activities
        let mut activities_map: HashMap<(i64, i64), NftMarketplaceActivity> = HashMap::new();
        for activity in activities {
            let key = (activity.txn_version, activity.index);
            activities_map.entry(key).or_insert(activity);
        }
        let mut deduped_activities: Vec<NftMarketplaceActivity> =
            activities_map.into_values().collect();

        // Deduplicate current listings
        let mut current_listings_map: HashMap<(String, String), CurrentNftMarketplaceListing> =
            HashMap::new();
        for current_listing in current_listings {
            let key: (String, String) = (
                current_listing.listing_id.clone(),
                current_listing.token_data_id.clone(),
            );
            current_listings_map
                .entry(key)
                .and_modify(|existing_listing| {
                    if current_listing.last_transaction_version
                        > existing_listing.last_transaction_version
                    {
                        *existing_listing = current_listing.clone();
                    }
                })
                .or_insert_with(|| current_listing.clone());
        }
        let deduped_current_listings: Vec<CurrentNftMarketplaceListing> =
            current_listings_map.into_values().collect();

        // Deduplicate current token bids
        let mut current_token_bids_map: HashMap<(String, String), CurrentNftMarketplaceBid> =
            HashMap::new();
        for current_bid in current_token_bids {
            let key = (
                current_bid.offer_id.clone(),
                current_bid.token_data_id.clone(),
            );
            current_token_bids_map
                .entry(key)
                .and_modify(|existing_bid| {
                    if current_bid.last_transaction_version > existing_bid.last_transaction_version
                    {
                        *existing_bid = current_bid.clone();
                    }
                })
                .or_insert_with(|| current_bid.clone());
        }
        let deduped_current_token_bids: Vec<CurrentNftMarketplaceBid> =
            current_token_bids_map.into_values().collect();

        // Deduplicate current collection bids
        let mut current_collection_bids_map: HashMap<
            (String, String),
            CurrentNftMarketplaceCollectionBid,
        > = HashMap::new();
        for current_collection_bid in current_collection_bids {
            let key = (
                current_collection_bid.collection_id.clone(),
                current_collection_bid.offer_id.clone().unwrap(),
            );
            current_collection_bids_map
                .entry(key)
                .and_modify(|existing_bid| {
                    if current_collection_bid.last_transaction_version
                        > existing_bid.last_transaction_version
                    {
                        *existing_bid = current_collection_bid.clone();
                    }
                })
                .or_insert_with(|| current_collection_bid.clone());
        }

        let deduped_current_collection_bids: Vec<CurrentNftMarketplaceCollectionBid> =
            current_collection_bids_map.into_values().collect();

        // Sort vectors by primary key to avoid deadlocks
        deduped_activities.sort_by(|a, b| {
            a.txn_version
                .cmp(&b.txn_version)
                .then(a.index.cmp(&b.index))
        });

        deduped_listings.sort_by(|a, b| {
            a.txn_version
                .cmp(&b.txn_version)
                .then(a.index.cmp(&b.index))
        });

        deduped_bids.sort_by(|a, b| {
            a.txn_version
                .cmp(&b.txn_version)
                .then(a.index.cmp(&b.index))
        });

        let nma = execute_in_chunks(
            self.db_pool.clone(),
            insert_nft_marketplace_activities,
            &deduped_activities,
            200,
        );

        let nmb = execute_in_chunks(
            self.db_pool.clone(),
            insert_nft_marketplace_bids,
            &deduped_bids,
            200,
        );

        let cnmb = execute_in_chunks(
            self.db_pool.clone(),
            insert_current_nft_marketplace_bids,
            &deduped_current_token_bids,
            200,
        );

        let nml = execute_in_chunks(
            self.db_pool.clone(),
            insert_nft_marketplace_listings,
            &deduped_listings,
            200,
        );

        let cnml = execute_in_chunks(
            self.db_pool.clone(),
            insert_current_nft_marketplace_listings,
            &deduped_current_listings,
            200,
        );

        let nmcb = execute_in_chunks(
            self.db_pool.clone(),
            insert_nft_marketplace_collection_bids,
            &collection_bids,
            200,
        );

        let cnmcb = execute_in_chunks(
            self.db_pool.clone(),
            insert_current_nft_marketplace_collection_bids,
            &deduped_current_collection_bids,
            200,
        );

        let (nma_res, nmb_res, cnmb_res, nml_res, cnml_res, nmcb_res, cnmcb_res) =
            tokio::join!(nma, nmb, cnmb, nml, cnml, nmcb, cnmcb);

        for res in [
            nma_res, nmb_res, cnmb_res, nml_res, cnml_res, nmcb_res, cnmcb_res,
        ] {
            match res {
                Ok(_) => (),
                Err(e) => {
                    println!("Error: {:?}", e);
                    return Err(ProcessorError::DBStoreError {
                        message: format!(
                            "Failed to store versions {} to {}: {:?}",
                            transactions.metadata.start_version,
                            transactions.metadata.end_version,
                            e,
                        ),
                        query: None,
                    });
                },
            }
        }

        Ok(Some(TransactionContext {
            data: (),
            metadata: transactions.metadata,
        }))
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

pub fn insert_current_nft_marketplace_bids(
    items_to_insert: Vec<CurrentNftMarketplaceBid>,
) -> (
    impl QueryFragment<Pg> + diesel::query_builder::QueryId + Send,
    Option<&'static str>,
) {
    use crate::schema::current_nft_marketplace_bids::dsl::*;

    (
        diesel::insert_into(schema::current_nft_marketplace_bids::table)
            .values(items_to_insert)
            .on_conflict((offer_id, token_data_id))
            .do_update()
            .set((
                is_deleted.eq(excluded(is_deleted)),
                last_transaction_timestamp.eq(excluded(last_transaction_timestamp)),
                token_amount.eq(excluded(token_amount)),
                last_transaction_version.eq(excluded(last_transaction_version)),
                price.eq(excluded(price)),
                index.eq(excluded(index)),
            )),
        Some(" WHERE current_nft_marketplace_bids.last_transaction_version < excluded.last_transaction_version "),
    )
}

pub fn insert_current_nft_marketplace_collection_bids(
    items_to_insert: Vec<CurrentNftMarketplaceCollectionBid>,
) -> (
    impl QueryFragment<Pg> + diesel::query_builder::QueryId + Send,
    Option<&'static str>,
) {
    use crate::schema::current_nft_marketplace_collection_bids::dsl::*;

    (
        diesel::insert_into(schema::current_nft_marketplace_collection_bids::table)
            .values(items_to_insert)
            .on_conflict((offer_id, collection_id))
            .do_update()
            .set((
                is_deleted.eq(excluded(is_deleted)),
                last_transaction_timestamp.eq(excluded(last_transaction_timestamp)),
                token_amount.eq(excluded(token_amount)),
                last_transaction_version.eq(excluded(last_transaction_version)),
                expiration_time.eq(excluded(expiration_time)),
                price.eq(excluded(price)),
                index.eq(excluded(index)),
            )),
        Some(" WHERE current_nft_marketplace_collection_bids.last_transaction_version < excluded.last_transaction_version "),
    )
}

pub fn insert_nft_marketplace_bids(
    items_to_insert: Vec<NftMarketplaceBid>,
) -> (
    impl QueryFragment<Pg> + diesel::query_builder::QueryId + Send,
    Option<&'static str>,
) {
    use crate::schema::nft_marketplace_bids::dsl::*;
    (
        diesel::insert_into(schema::nft_marketplace_bids::table)
            .values(items_to_insert)
            .on_conflict((txn_version, index))
            .do_nothing(),
        None,
    )
}

pub fn insert_nft_marketplace_collection_bids(
    items_to_insert: Vec<NftMarketplaceCollectionBid>,
) -> (
    impl QueryFragment<Pg> + diesel::query_builder::QueryId + Send,
    Option<&'static str>,
) {
    use crate::schema::nft_marketplace_collection_bids::dsl::*;

    (
        diesel::insert_into(schema::nft_marketplace_collection_bids::table)
            .values(items_to_insert)
            .on_conflict((txn_version, index))
            .do_nothing(),
        None,
    )
}

pub fn insert_nft_marketplace_listings(
    items_to_insert: Vec<NftMarketplaceListing>,
) -> (
    impl QueryFragment<Pg> + diesel::query_builder::QueryId + Send,
    Option<&'static str>,
) {
    use crate::schema::nft_marketplace_listings::dsl::*;

    (
        diesel::insert_into(schema::nft_marketplace_listings::table)
            .values(items_to_insert)
            .on_conflict((txn_version, index))
            .do_nothing(),
        None,
    )
}

pub fn insert_current_nft_marketplace_listings(
    items_to_insert: Vec<CurrentNftMarketplaceListing>,
) -> (
    impl QueryFragment<Pg> + diesel::query_builder::QueryId + Send,
    Option<&'static str>,
) {
    use crate::schema::current_nft_marketplace_listings::dsl::*;

    (
        diesel::insert_into(schema::current_nft_marketplace_listings::table)
            .values(items_to_insert)
            .on_conflict((listing_id, token_data_id))
            .do_update()
            .set((
                is_deleted.eq(excluded(is_deleted)),
                last_transaction_timestamp.eq(excluded(last_transaction_timestamp)),
                token_amount.eq(excluded(token_amount)),
                last_transaction_version.eq(excluded(last_transaction_version)),
                price.eq(excluded(price)),
            )),
        Some(" WHERE current_nft_marketplace_listings.last_transaction_version < excluded.last_transaction_version "),
    )
}
