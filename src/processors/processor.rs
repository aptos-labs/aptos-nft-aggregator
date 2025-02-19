use super::{
    config_boilerplate::{DbConfig, IndexerProcessorConfig},
    marketplace_config::MarketplaceEventConfigMapping,
    models::{
        CurrentNftMarketplaceBid, CurrentNftMarketplaceCollectionBid, CurrentNftMarketplaceListing,
        NftMarketplaceActivity, NftMarketplaceBid, NftMarketplaceCollectionBid,
        NftMarketplaceListing,
    },
    postgres_utils::{execute_in_chunks, new_db_pool, ArcDbPool},
    util::{
        convert_move_struct_tag, extract_type_arguments, get_collection_offer_metadata,
        get_collection_offer_v1, get_collection_offer_v2, get_fixed_priced_listing,
        get_listing_metadata, get_move_type_str, get_object_core, get_token_offer_metadata,
        get_token_offer_v1, get_token_offer_v2, CollectionMetadata, CollectionOfferEventMetadata,
        CollectionOfferMetadata, CollectionOfferV1, CollectionOfferV2, FixedPriceListing,
        ListingMetadata, ListingTokenV1Container, ObjectCore, TokenDataIdType, TokenMetadata,
        TokenOfferMetadata, TokenOfferV1, TokenOfferV2, TokenStandard,
    },
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
    utils::{
        convert::standardize_address, errors::ProcessorError,
        extract::get_entry_function_from_user_request,
    },
};
use aptos_protos::transaction::v1::{
    transaction::TxnData, write_set_change::Change as WriteSetChangeEnum, Transaction,
    WriteSetChange as WriteSetChangePB,
};
use chrono::NaiveDateTime;
use diesel::{
    pg::{upsert::excluded, Pg},
    query_builder::QueryFragment,
    ExpressionMethods,
};
use serde_json::Value;
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

    fn get_contract_address(&self) -> String {
        self.config.nft_marketplace_config.contract_address.clone()
    }
}

#[async_trait::async_trait]
impl ProcessorTrait for Processor {
    fn name(&self) -> &'static str {
        "nft_marketplace_processor"
    }

    async fn run_processor(&self) -> Result<()> {
        // Run migrations
        let DbConfig::PostgresConfig(ref _postgres_config) = self.config.db_config;

        // run_migrations(
        //     postgres_config.connection_string.clone(),
        //     self.db_pool.clone(),
        // )
        // .await;

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

        let event_mapping = self
            .config
            .nft_marketplace_config
            .get_event_mapping()
            .unwrap_or_else(|e| {
                error!("Failed to get event mapping: {:?}", e);
                panic!("Failed to get event mapping: {:?}", e);
            });
        let process = ProcessStep::new(
            Arc::new(event_mapping),
            self.get_contract_address().to_string(),
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
    pub event_mapping: Arc<MarketplaceEventConfigMapping>,
    pub contract_address: String,
    pub db_pool: ArcDbPool,
}

impl ProcessStep {
    pub fn new(
        event_mapping: Arc<MarketplaceEventConfigMapping>,
        contract_address: String,
        db_pool: ArcDbPool,
    ) -> Self {
        Self {
            event_mapping,
            contract_address,
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
        // let config = self.config.clone();
        let txns = transactions.data;

        let mut token_metadatas: HashMap<String, TokenMetadata> = HashMap::new();
        // Collection metaddatas parsed from events. The key is generated collection_id for token v1,
        // and collection address for token v2.
        let mut collection_metadatas: HashMap<String, CollectionMetadata> = HashMap::new();

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

                let mut collection_offer_filled_metadatas: HashMap<
                    String,
                    CollectionOfferEventMetadata,
                > = HashMap::new();

                for (event_index, event) in events.iter().enumerate() {
                    if let Some(activity) = NftMarketplaceActivity::from_event(
                        event,
                        txn.version as i64,
                        event_index as i64,
                        txn_timestamp,
                        &entry_function_id,
                        &self.event_mapping,
                        &mut token_metadatas,
                        &mut collection_metadatas,
                    ) {
                        match activity.standard_event_type.as_str() {
                            "place_listing" => {
                                let (listing, current_listing) =
                                    NftMarketplaceListing::from_activity_to_current(&activity);
                                println!("Listing: {:#?}", listing);
                                println!("Current Listing: {:#?}", current_listing);
                                listings.push(listing);
                                current_listings.push(current_listing);
                            },
                            "cancel_listing" => {
                                let (listing, current_listing) =
                                    NftMarketplaceListing::from_activity_to_current(&activity);
                                println!("Listing: {:#?}", listing);
                                println!("Current Listing: {:#?}", current_listing);
                                listings.push(listing);
                            },
                            "fill_listing" => {
                                let (listing, current_listing) =
                                    NftMarketplaceListing::from_activity_to_current(&activity);
                                println!("Listing: {:#?}", listing);
                                println!("Current Listing: {:#?}", current_listing);
                                listings.push(listing);
                            },
                            "place_offer" => {
                                let offer = NftMarketplaceBid::from_activity(&activity);
                                println!("Offer: {:#?}", offer);
                                token_bids.push(offer);
                            },
                            "cancel_offer" => {
                                let offer = NftMarketplaceBid::from_activity(&activity);
                                println!("Offer: {:#?}", offer);
                                token_bids.push(offer);
                            },
                            "fill_offer" => {
                                let offer = NftMarketplaceBid::from_activity(&activity);
                                println!("Offer: {:#?}", offer);
                                token_bids.push(offer);
                            },
                            "place_collection_offer" => {
                                let offer = NftMarketplaceCollectionBid::from_activity(&activity);
                                println!("Offer: {:#?}", offer);
                                collection_bids.push(offer);
                            },
                            "cancel_collection_offer" => {
                                let offer = NftMarketplaceCollectionBid::from_activity(&activity);
                                println!("Offer: {:#?}", offer);
                                collection_bids.push(offer);
                            },
                            "fill_collection_offer" => {
                                let price = activity.price.clone();

                                // TODO: get token_metadata from token_metadatas
                                let collection_offer_filled_metadata =
                                    CollectionOfferEventMetadata {
                                        collection_offer_id: activity
                                            .collection_id
                                            .clone()
                                            .unwrap_or_default(),
                                        collection_metadata: CollectionMetadata {
                                            collection_id: activity
                                                .collection_id
                                                .clone()
                                                .unwrap_or_default(),
                                            creator_address: activity
                                                .creator_address
                                                .clone()
                                                .unwrap_or_default(),
                                            collection_name: activity
                                                .collection_name
                                                .clone()
                                                .unwrap_or_default(),
                                            token_standard: activity
                                                .token_standard
                                                .clone()
                                                .unwrap_or_default(),
                                        },
                                        item_price: price.unwrap_or_default(),
                                        buyer: activity.buyer.clone().unwrap_or_default(),
                                    };

                                // TODO: get collection offer id from event mapping or from data
                                collection_offer_filled_metadatas.insert(
                                    collection_offer_filled_metadata.collection_offer_id.clone(), // need to use offer_or_listing_id
                                    collection_offer_filled_metadata,
                                );
                                let collection_bid =
                                    NftMarketplaceCollectionBid::from_activity(&activity);
                                println!("Collection Bid: {:#?}", collection_bid);

                                collection_bids.push(collection_bid);
                            },
                            _ => {
                                println!("Unknown event type: {:?}", activity.standard_event_type);
                            },
                        }

                        println!("activity: {:#?}", activity);
                        activities.push(activity);
                    }
                }

                let transaction_info = txn.info.as_ref().expect("Transaction info doesn't exist!");
                let write_set_changes: &[WriteSetChangePB] = &transaction_info.changes;
                // let txn_height = txn.block_height;

                let type_args = extract_type_arguments(req);
                let coin_type = if let Some(type_args) = type_args {
                    let move_type = type_args[0].clone();
                    get_move_type_str(move_type)
                } else {
                    None
                };

                let mut object_metadatas: HashMap<String, ObjectCore> = HashMap::new();
                let mut listing_metadatas: HashMap<String, ListingMetadata> = HashMap::new();
                let mut fixed_price_listings: HashMap<String, FixedPriceListing> = HashMap::new();
                let mut listing_token_v1_containers: HashMap<String, ListingTokenV1Container> =
                    HashMap::new();
                let mut token_offer_metadatas: HashMap<String, TokenOfferMetadata> = HashMap::new();
                let mut token_offer_v1s: HashMap<String, TokenOfferV1> = HashMap::new();
                let mut token_offer_v2s: HashMap<String, TokenOfferV2> = HashMap::new();
                let mut collection_offer_metadatas: HashMap<String, CollectionOfferMetadata> =
                    HashMap::new();
                let mut collection_offer_v1s: HashMap<String, CollectionOfferV1> = HashMap::new();
                let mut collection_offer_v2s: HashMap<String, CollectionOfferV2> = HashMap::new();

                // 2nd loop to get all the resources
                // # Parse out all the listing, auction, bid, and offer data from write set changes.
                // # This is a bit more complicated than the other parsers because the data is spread out across multiple write set changes,
                for wsc in write_set_changes {
                    let change = wsc
                        .change
                        .as_ref()
                        .expect("WriteSetChange must have a change");

                    if let WriteSetChangeEnum::WriteResource(write_resource) = change {
                        let move_resource_address = standardize_address(&write_resource.address);
                        let move_resource_type = &write_resource.type_str;

                        let move_struct_tag = match write_resource.r#type.as_ref() {
                            Some(t) => t,
                            None => return Ok(None),
                        };
                        let parsed_data = convert_move_struct_tag(move_struct_tag);
                        let standarized_move_resource_type_address = parsed_data.resource_address;
                        let data: Value = match serde_json::from_str(&write_resource.data) {
                            Ok(value) => value,
                            Err(e) => {
                                error!("Failed to parse write_resource data: {:?}", e);
                                return Err(ProcessorError::ProcessError {
                                    message: format!(
                                        "Failed to parse write_resource data: {:?}",
                                        e
                                    ),
                                });
                            },
                        };

                        let object_core = get_object_core(move_resource_type, &data);

                        if let Some(object_core) = object_core {
                            object_metadatas.insert(move_resource_address.clone(), object_core);
                        }

                        if standarized_move_resource_type_address != self.contract_address {
                            continue;
                        }

                        if let Some(listing_metadata) =
                            get_listing_metadata(move_resource_type, &data, &self.contract_address)
                        {
                            listing_metadatas
                                .insert(move_resource_address.clone(), listing_metadata);
                        }

                        if let Some(fixed_price_listing) = get_fixed_priced_listing(
                            move_resource_type,
                            &data,
                            &self.contract_address,
                        ) {
                            fixed_price_listings
                                .insert(move_resource_address.clone(), fixed_price_listing);
                        }

                        if let Some(listing_token_v1_container) = get_listing_token_v1_container(
                            move_resource_type,
                            &data,
                            &self.contract_address,
                        ) {
                            listing_token_v1_containers
                                .insert(move_resource_address.clone(), listing_token_v1_container);
                        }

                        if let Some(token_offer_metadata) = get_token_offer_metadata(
                            move_resource_type,
                            &data,
                            &self.contract_address,
                        ) {
                            token_offer_metadatas
                                .insert(move_resource_address.clone(), token_offer_metadata);
                        }
                        if let Some(token_offer_v1) =
                            get_token_offer_v1(move_resource_type, &data, &self.contract_address)
                        {
                            token_offer_v1s.insert(move_resource_address.clone(), token_offer_v1);
                        }
                        if let Some(token_offer_v2) =
                            get_token_offer_v2(move_resource_type, &data, &self.contract_address)
                        {
                            token_offer_v2s.insert(move_resource_address.clone(), token_offer_v2);
                        }

                        if let Some(collection_offer_metadata) = get_collection_offer_metadata(
                            move_resource_type,
                            &data,
                            &self.contract_address,
                        ) {
                            collection_offer_metadatas
                                .insert(move_resource_address.clone(), collection_offer_metadata);
                        }
                        if let Some(collection_offer_v1) = get_collection_offer_v1(
                            move_resource_type,
                            &data,
                            &self.contract_address,
                        ) {
                            collection_offer_v1s
                                .insert(move_resource_address.clone(), collection_offer_v1);
                        }
                        if let Some(collection_offer_v2) = get_collection_offer_v2(
                            move_resource_type,
                            &data,
                            &self.contract_address,
                        ) {
                            collection_offer_v2s
                                .insert(move_resource_address.clone(), collection_offer_v2);
                        }
                    }
                }

                // Loop 3
                // Reconstruct the full listing and offer models and create DB objects
                for (wsc_index, wsc) in write_set_changes.iter().enumerate() {
                    let change = wsc
                        .change
                        .as_ref()
                        .expect("WriteSetChange must have a change");
                    println!("write set change index: {:?}", wsc_index);
                    let entry_function_id_str = entry_function_id
                        .clone()
                        .expect("Entry function id is not present");

                    match change {
                        WriteSetChangeEnum::WriteResource(write_resource) => {
                            let move_resource_address: String =
                                standardize_address(&write_resource.address);
                            let move_resource_type = write_resource.type_str.clone();
                            let move_resource_type = move_resource_type.as_str();
                            println!("move resource type: {:?}", move_resource_type);
                            let move_struct_tag = match write_resource.r#type.as_ref() {
                                Some(t) => t,
                                None => return Ok(None),
                            };
                            println!("move struct tag: {:?}", move_struct_tag);
                            let parsed_data = convert_move_struct_tag(move_struct_tag);
                            let standarized_move_resource_type_address =
                                parsed_data.resource_address;

                            if standarized_move_resource_type_address != self.contract_address {
                                continue;
                            }

                            // Listing
                            if move_resource_type
                                == format!("{}::listing::Listing", self.contract_address).as_str()
                            {
                                let listing_metadata =
                                    match listing_metadatas.get(&move_resource_address) {
                                        Some(metadata) => metadata,
                                        None => {
                                            eprintln!(
                                                "Listing metadata not found for txn {}",
                                                txn.version
                                            );
                                            continue;
                                        },
                                    };

                                if let Some(fixed_price_listing) =
                                    fixed_price_listings.get(&move_resource_address)
                                {
                                    let token_address = &listing_metadata.token_address;
                                    let token_v1_container =
                                        listing_token_v1_containers.get(token_address);

                                    let (listing, current_listing) =
                                        if let Some(token_v1_container) = token_v1_container {
                                            let token_v1_metadata =
                                                &token_v1_container.token_metadata;
                                            (
                                                NftMarketplaceListing {
                                                    token_id: token_v1_metadata
                                                        .token_data_id
                                                        .clone(),
                                                    transaction_version: txn.version as i64,
                                                    creator_address: Some(
                                                        token_v1_metadata.creator_address.clone(),
                                                    ),
                                                    token_name: Some(
                                                        token_v1_metadata.token_name.clone(),
                                                    ),
                                                    token_data_id: Some(
                                                        token_v1_metadata.token_data_id.clone(),
                                                    ),
                                                    collection_name: Some(
                                                        token_v1_metadata.collection_name.clone(),
                                                    ),
                                                    collection_id: Some(
                                                        token_v1_metadata.collection_id.clone(),
                                                    ),
                                                    price: Some(fixed_price_listing.price.into()),
                                                    token_amount: Some(
                                                        token_v1_container.amount.into(),
                                                    ),
                                                    token_standard: Some(
                                                        TokenStandard::V1.to_string(),
                                                    ),
                                                    seller: Some(listing_metadata.seller.clone()),
                                                    marketplace: "EXAMPLE_V2_MARKETPLACE"
                                                        .to_string(),
                                                    contract_address: self.contract_address.clone(),
                                                    entry_function_id_str: entry_function_id_str
                                                        .clone(),
                                                    transaction_timestamp: txn_timestamp,
                                                },
                                                CurrentNftMarketplaceListing {
                                                    token_id: token_v1_metadata
                                                        .token_data_id
                                                        .clone(),
                                                    token_data_id: Some(
                                                        token_v1_metadata.token_data_id.clone(),
                                                    ),
                                                    creator_address: Some(
                                                        token_v1_metadata.creator_address.clone(),
                                                    ),
                                                    token_name: Some(
                                                        token_v1_metadata.token_name.clone(),
                                                    ),
                                                    collection_name: Some(
                                                        token_v1_metadata.collection_name.clone(),
                                                    ),
                                                    collection_id: Some(
                                                        token_v1_metadata.collection_id.clone(),
                                                    ),
                                                    price: Some(fixed_price_listing.price.into()),
                                                    token_amount: Some(
                                                        token_v1_container.amount.into(),
                                                    ),
                                                    token_standard: Some(
                                                        TokenStandard::V1.to_string(),
                                                    ),
                                                    seller: Some(listing_metadata.seller.clone()),
                                                    is_deleted: false,
                                                    marketplace: "EXAMPLE_V2_MARKETPLACE"
                                                        .to_string(),
                                                    contract_address: self.contract_address.clone(),
                                                    entry_function_id_str: entry_function_id_str
                                                        .clone(),
                                                    last_transaction_version: Some(
                                                        txn.version as i64,
                                                    ),
                                                    last_transaction_timestamp: txn_timestamp,
                                                },
                                            )
                                        } else {
                                            let token_v2_metadata = token_metadatas.get(token_address).expect("Token v2 metadata not found");

                                            (
                                                NftMarketplaceListing {
                                                    token_id: token_v2_metadata.token_data_id.clone(), // TODO: check if this is correct
                                                    transaction_version: txn.version as i64,
                                                    creator_address: Some(
                                                        token_v2_metadata.creator_address.clone(),
                                                    ),
                                                    token_name: Some(
                                                        token_v2_metadata.token_name.clone(),
                                                    ),
                                                    token_data_id: Some(
                                                        token_v2_metadata.token_data_id.clone(),
                                                    ),
                                                    collection_name: Some(
                                                        token_v2_metadata.collection_name.clone(),
                                                    ),
                                                    collection_id: Some(
                                                        token_v2_metadata.collection_id.clone(),
                                                    ),
                                                    price: Some(fixed_price_listing.price.into()),
                                                    token_amount: Some(1.into()),
                                                    token_standard: Some(
                                                        TokenStandard::V2.to_string(),
                                                    ),
                                                    seller: Some(listing_metadata.seller.clone()),
                                                    marketplace: "EXAMPLE_V2_MARKETPLACE" // TODO: Fix this
                                                        .to_string(),
                                                    contract_address: self.contract_address.clone(),
                                                    entry_function_id_str: entry_function_id_str
                                                        .clone(),
                                                    transaction_timestamp: txn_timestamp,
                                                },
                                                CurrentNftMarketplaceListing {
                                                    token_id: token_v2_metadata.token_data_id.clone(), // TODO: check if this is correct
                                                    token_data_id: Some(
                                                        token_v2_metadata.token_data_id.clone(),
                                                    ),
                                                    creator_address: Some(
                                                        token_v2_metadata.creator_address.clone(),
                                                    ),
                                                    token_name: Some(
                                                        token_v2_metadata.token_name.clone(),
                                                    ),
                                                    collection_name: Some(
                                                        token_v2_metadata.collection_name.clone(),
                                                    ),
                                                    collection_id: Some(
                                                        token_v2_metadata.collection_id.clone(),
                                                    ),
                                                    price: Some(fixed_price_listing.price.into()),
                                                    token_amount: Some(1.into()),
                                                    token_standard: Some(
                                                        TokenStandard::V2.to_string(),
                                                    ),
                                                    seller: Some(listing_metadata.seller.clone()),
                                                    is_deleted: false,
                                                    marketplace: "EXAMPLE_V2_MARKETPLACE" // TODO: Fix this
                                                        .to_string(),
                                                    contract_address: self.contract_address.clone(),
                                                    entry_function_id_str: entry_function_id_str
                                                        .clone(),
                                                    last_transaction_version: Some(
                                                        txn.version as i64,
                                                    ),
                                                    last_transaction_timestamp: txn_timestamp,
                                                },
                                            )
                                        };

                                    listings.push(listing);
                                    current_listings.push(current_listing);
                                } else {
                                    // If the listing is an acution, we ignore it for now
                                    // TODO: should we check if it's not an auction as well
                                    println!("Listing is an auction, ignoring for now");
                                }
                            } else if move_resource_type
                                == format!("{}::token_offer::TokenOffer", self.contract_address)
                                    .as_str()
                            {
                                let token_offer_object = object_metadatas
                                    .get(&move_resource_address)
                                    .expect("Token offer object not found");

                                let token_offer_metadata: &TokenOfferMetadata =
                                    token_offer_metadatas
                                        .get(&move_resource_address)
                                        .expect("Token offer metadata not found");
                                let token_offer_v1: Option<&TokenOfferV1> =
                                    token_offer_v1s.get(&move_resource_address);
                                if let Some(token_offer_v1) = token_offer_v1 {
                                    let token_v1_metadata = &token_offer_v1.token_metadata;
                                    let token_bid: NftMarketplaceBid = NftMarketplaceBid {
                                        transaction_version: txn.version as i64,
                                        event_index: 0,
                                        token_id: Some(token_v1_metadata.token_data_id.clone()),
                                        token_data_id: token_v1_metadata.token_data_id.clone(),
                                        buyer: token_offer_object.owner.clone(),
                                        price: token_offer_metadata.price.clone(),
                                        creator_address: Some(
                                            token_v1_metadata.creator_address.clone(),
                                        ),
                                        token_amount: Some(1.into()),
                                        token_name: Some(token_v1_metadata.token_name.clone()),
                                        collection_name: Some(
                                            token_v1_metadata.collection_name.clone(),
                                        ),
                                        collection_id: Some(
                                            token_v1_metadata.collection_id.clone(),
                                        ),
                                        marketplace: "EXAMPLE_V2_MARKETPLACE".to_string(),
                                        contract_address: self.contract_address.clone(),
                                        entry_function_id_str: entry_function_id_str.clone(),
                                        event_type: "token_offer".to_string(),
                                        transaction_timestamp: txn_timestamp,
                                    };

                                    let current_token_bid = CurrentNftMarketplaceBid {
                                        token_id: Some(token_v1_metadata.token_data_id.clone()),
                                        token_data_id: token_v1_metadata.token_data_id.clone(),
                                        buyer: token_offer_object.owner.clone(),
                                        price: token_offer_metadata.price.clone(),
                                        creator_address: Some(
                                            token_v1_metadata.creator_address.clone(),
                                        ),
                                        token_amount: Some(1.into()),
                                        token_name: Some(token_v1_metadata.token_name.clone()),
                                        collection_name: Some(
                                            token_v1_metadata.collection_name.clone(),
                                        ),
                                        collection_id: Some(
                                            token_v1_metadata.collection_id.clone(),
                                        ),
                                        marketplace: "EXAMPLE_V2_MARKETPLACE".to_string(),
                                        contract_address: self.contract_address.clone(),
                                        entry_function_id_str: entry_function_id_str.clone(),
                                        is_deleted: false,
                                        last_transaction_version: Some(txn.version as i64),
                                        last_transaction_timestamp: txn_timestamp,
                                    };

                                    token_bids.push(token_bid);
                                    current_token_bids.push(current_token_bid);
                                } else {
                                    let token_offer_v2 = token_offer_v2s
                                        .get(&move_resource_address)
                                        .expect("Token offer v2 not found");
                                    let token_v2_metadata = token_metadatas
                                        .get(&token_offer_v2.token_address)
                                        .expect("Token v2 metadata not found");

                                    let token_bid: NftMarketplaceBid = NftMarketplaceBid {
                                        transaction_version: txn.version as i64,
                                        event_index: 0,
                                        token_id: Some(token_v2_metadata.token_data_id.clone()),
                                        token_data_id: token_v2_metadata.token_data_id.clone(),
                                        buyer: token_offer_object.owner.clone(),
                                        price: token_offer_metadata.price.clone(),
                                        creator_address: Some(
                                            token_v2_metadata.creator_address.clone(),
                                        ),
                                        token_amount: Some(1.into()),
                                        token_name: Some(token_v2_metadata.token_name.clone()),
                                        collection_name: Some(
                                            token_v2_metadata.collection_name.clone(),
                                        ),
                                        collection_id: Some(
                                            token_v2_metadata.collection_id.clone(),
                                        ),
                                        marketplace: "EXAMPLE_V2_MARKETPLACE".to_string(),
                                        contract_address: self.contract_address.clone(),
                                        entry_function_id_str: entry_function_id_str.clone(),
                                        event_type: "token_offer".to_string(),
                                        transaction_timestamp: txn_timestamp,
                                    };

                                    let current_token_bid = CurrentNftMarketplaceBid {
                                        token_id: Some(token_v2_metadata.token_data_id.clone()),
                                        token_data_id: token_v2_metadata.token_data_id.clone(),
                                        buyer: token_offer_object.owner.clone(),
                                        price: token_offer_metadata.price.clone(),
                                        creator_address: Some(
                                            token_v2_metadata.creator_address.clone(),
                                        ),
                                        token_amount: Some(1.into()),
                                        token_name: Some(token_v2_metadata.token_name.clone()),
                                        collection_name: Some(
                                            token_v2_metadata.collection_name.clone(),
                                        ),
                                        collection_id: Some(
                                            token_v2_metadata.collection_id.clone(),
                                        ),
                                        marketplace: "EXAMPLE_V2_MARKETPLACE".to_string(),
                                        contract_address: self.contract_address.clone(),
                                        entry_function_id_str: entry_function_id_str.clone(),
                                        is_deleted: false,
                                        last_transaction_version: Some(txn.version as i64),
                                        last_transaction_timestamp: txn_timestamp,
                                    };

                                    token_bids.push(token_bid);
                                    current_token_bids.push(current_token_bid);
                                }
                            } else if move_resource_type
                                == format!(
                                    "{}::collection_offer::CollectionOffer",
                                    self.contract_address
                                )
                                .as_str()
                            {
                                println!(
                                    "Collection offer resource type: {:?}",
                                    move_resource_type
                                );
                                let collection_offer_metadata = collection_offer_metadatas
                                    .get(&move_resource_address)
                                    .expect("Collection offer metadata not found");

                                println!(
                                    "Collection offer metadata: {:?}",
                                    collection_offer_metadata
                                );
                                println!("write set change index: {:?}", wsc_index);
                                let collection_object = object_metadatas
                                    .get(&move_resource_address)
                                    .expect("Collection object not found");
                                let collection_offer_v1 =
                                    collection_offer_v1s.get(&move_resource_address);

                                // Collection offer v1
                                if let Some(collection_offer_v1) = collection_offer_v1 {
                                    let collection_v1_metadata =
                                        &collection_offer_v1.collection_metadata;

                                    let collection_bid = NftMarketplaceCollectionBid {
                                        transaction_version: txn.version as i64,
                                        event_index: None,
                                        creator_address: Some(
                                            collection_v1_metadata.creator_address.clone(),
                                        ),
                                        collection_name: Some(
                                            collection_v1_metadata.collection_name.clone(),
                                        ),
                                        collection_id: Some(
                                            collection_v1_metadata.collection_id.clone(),
                                        ),
                                        price: collection_offer_metadata.item_price.clone(),
                                        token_amount: Some(1.into()),
                                        buyer: None,
                                        seller: Some(collection_object.owner.clone()),
                                        marketplace: "EXAMPLE_V2_MARKETPLACE".to_string(),
                                        contract_address: self.contract_address.clone(),
                                        entry_function_id_str: entry_function_id_str.clone(),
                                        event_type: "collection_offer".to_string(),
                                        transaction_timestamp: txn_timestamp,
                                    };

                                    // Current collection offer
                                    let current_collection_bid =
                                        CurrentNftMarketplaceCollectionBid {
                                            collection_offer_id: move_resource_address,
                                            collection_id: collection_v1_metadata
                                                .collection_id
                                                .clone(),
                                            buyer: None,
                                            price: collection_offer_metadata.item_price.clone(),
                                            creator_address: Some(
                                                collection_v1_metadata.creator_address.clone(),
                                            ),
                                            token_amount: Some(1.into()),
                                            collection_name: Some(
                                                collection_v1_metadata.collection_name.clone(),
                                            ),
                                            marketplace: "EXAMPLE_V2_MARKETPLACE".to_string(),
                                            contract_address: self.contract_address.clone(),
                                            entry_function_id_str: entry_function_id_str.clone(),
                                            coin_type: coin_type.clone(),
                                            expiration_time: collection_offer_metadata
                                                .expiration_time,
                                            is_deleted: false,
                                            last_transaction_version: Some(txn.version as i64),
                                            last_transaction_timestamp: txn_timestamp,
                                        };

                                    collection_bids.push(collection_bid);
                                    current_collection_bids.push(current_collection_bid);
                                }
                                // Collection offer v2
                                else {
                                    println!(
                                        "Collection offer v2 resource type: {:?}",
                                        move_resource_type
                                    );
                                    let collection_offer_v2_resource_type: String = format!(
                                        "{}::collection_offer::CollectionOffer",
                                        self.contract_address
                                    );
                                    println!(
                                        "Collection offer v2 resource type: {:?}",
                                        collection_offer_v2_resource_type
                                    );
                                    let collection_offer_v2: &CollectionOfferV2 =
                                        collection_offer_v2s
                                            .get(&move_resource_address)
                                            .expect("Collection offer v2 not found");

                                    let collection_bid = NftMarketplaceCollectionBid {
                                        transaction_version: txn.version as i64,
                                        event_index: None,
                                        creator_address: None,
                                        collection_name: None,
                                        collection_id: Some(
                                            collection_offer_v2.collection_address.clone(),
                                        ),
                                        price: collection_offer_metadata.item_price.clone(),
                                        token_amount: Some(1.into()),
                                        buyer: None,
                                        seller: None,
                                        marketplace: "EXAMPLE_V2_MARKETPLACE".to_string(),
                                        contract_address: self.contract_address.clone(),
                                        entry_function_id_str: entry_function_id_str.clone(),
                                        event_type: "collection_offer".to_string(),
                                        transaction_timestamp: txn_timestamp,
                                    };

                                    let current_collection_bid =
                                        CurrentNftMarketplaceCollectionBid {
                                            collection_offer_id: move_resource_address,
                                            collection_id: collection_offer_v2
                                                .collection_address
                                                .clone(),
                                            buyer: None,
                                            price: collection_offer_metadata.item_price.clone(),
                                            creator_address: None,
                                            token_amount: Some(1.into()),
                                            collection_name: None,
                                            marketplace: "EXAMPLE_V2_MARKETPLACE".to_string(),
                                            contract_address: self.contract_address.clone(),
                                            entry_function_id_str: entry_function_id_str.clone(),
                                            last_transaction_version: Some(txn.version as i64),
                                            last_transaction_timestamp: txn_timestamp,
                                            coin_type: coin_type.clone(),
                                            expiration_time: collection_offer_metadata
                                                .expiration_time,
                                            is_deleted: false,
                                        };

                                    collection_bids.push(collection_bid);
                                    current_collection_bids.push(current_collection_bid);
                                }
                            }
                        },
                        WriteSetChangeEnum::DeleteResource(delete_resource) => {
                            let move_resource_address: String =
                                standardize_address(&delete_resource.address);
                            // # If a collection offer resource gets deleted, that means it the offer was filled completely
                            // # and we handle that here.
                            // TODO: handle collection offer filled

                            let maybe_collection_offer_filled_metadata =
                                collection_offer_filled_metadatas.get(&move_resource_address);
                            if let Some(collection_offer_filled_metadata) =
                                maybe_collection_offer_filled_metadata
                            {
                                let collection_metadata: CollectionMetadata =
                                    collection_offer_filled_metadata.collection_metadata.clone();
                                let collection_bid = NftMarketplaceCollectionBid {
                                    transaction_version: txn.version as i64,
                                    event_index: None,
                                    seller: Some("seller".to_string()),
                                    event_type: "collection_offer_filled".to_string(),
                                    collection_id: Some(collection_metadata.collection_id.clone()),
                                    buyer: Some(collection_offer_filled_metadata.buyer.clone()),
                                    price: collection_offer_filled_metadata.item_price.clone(),
                                    creator_address: Some(
                                        collection_metadata.creator_address.clone(),
                                    ),
                                    token_amount: Some(1.into()),
                                    collection_name: Some(
                                        collection_metadata.collection_name.clone(),
                                    ),
                                    marketplace: "EXAMPLE_V2_MARKETPLACE".to_string(),
                                    contract_address: self.contract_address.clone(),
                                    entry_function_id_str: entry_function_id_str.clone(),
                                    transaction_timestamp: txn_timestamp,
                                };

                                let current_collection_bid = CurrentNftMarketplaceCollectionBid {
                                    collection_offer_id: move_resource_address,
                                    collection_id: collection_metadata.collection_id,
                                    buyer: Some(collection_offer_filled_metadata.buyer.clone()),
                                    price: collection_offer_filled_metadata.item_price.clone(),
                                    creator_address: Some(
                                        collection_metadata.creator_address.clone(),
                                    ),
                                    token_amount: Some(1.into()),
                                    collection_name: Some(collection_metadata.collection_name),
                                    marketplace: "EXAMPLE_V2_MARKETPLACE".to_string(),
                                    coin_type: coin_type.clone(),
                                    expiration_time: 0,
                                    contract_address: self.contract_address.clone(),
                                    entry_function_id_str: entry_function_id_str.clone(),
                                    is_deleted: true,
                                    last_transaction_version: Some(txn.version as i64),
                                    last_transaction_timestamp: txn_timestamp,
                                };

                                collection_bids.push(collection_bid);
                                current_collection_bids.push(current_collection_bid);
                            }
                        },
                        WriteSetChangeEnum::WriteTableItem(table_item) => {
                            // let table_item_handle = table_item.handle;
                            println!("Write table item: {:?}", table_item);
                        },
                        _ => {},
                    }
                }
            }
        }

        let nma = execute_in_chunks(
            self.db_pool.clone(),
            insert_nft_marketplace_activities,
            &activities,
            200,
        );

        let nmb = execute_in_chunks(
            self.db_pool.clone(),
            insert_nft_marketplace_bids,
            &token_bids,
            200,
        );

        let cnmb = execute_in_chunks(
            self.db_pool.clone(),
            insert_current_nft_marketplace_bids,
            &current_token_bids,
            200,
        );

        let nml = execute_in_chunks(
            self.db_pool.clone(),
            insert_nft_marketplace_listings,
            &listings,
            200,
        );

        let cnml = execute_in_chunks(
            self.db_pool.clone(),
            insert_current_nft_marketplace_listings,
            &current_listings,
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
            &current_collection_bids,
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

fn get_listing_token_v1_container(
    move_resource_type: &str,
    data: &Value,
    marketplace_contract_address: &str,
) -> Option<ListingTokenV1Container> {
    if move_resource_type
        != format!(
            "{}::listing::TokenV1Container",
            marketplace_contract_address
        )
    {
        return None;
    }

    let token = data.get("token")?;
    let amount = token.get("amount")?.as_i64()?;
    let property_version = token.get("id")?.get("property_version")?.as_i64();
    let token_data_id_struct = token.get("id")?.get("token_data_id")?;

    let token_data_id_type = TokenDataIdType::new(
        token_data_id_struct.get("creator")?.as_str()?.to_string(),
        token_data_id_struct
            .get("collection")?
            .as_str()?
            .to_string(),
        token_data_id_struct.get("name")?.as_str()?.to_string(),
    );

    Some(ListingTokenV1Container {
        token_metadata: TokenMetadata {
            collection_id: token_data_id_type.get_collection_data_id_hash(),
            token_data_id: token_data_id_type.to_hash(),
            creator_address: token_data_id_type.get_creator(),
            collection_name: token_data_id_type.get_collection_trunc(),
            token_name: token_data_id_type.get_name_trunc(),
            property_version,
            token_standard: TokenStandard::V1.to_string(),
        },
        amount,
    })
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
            .on_conflict((txn_version, event_index))
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
            .on_conflict((token_data_id, buyer, price))
            .do_update()
            .set((
                is_deleted.eq(excluded(is_deleted)),
            )),
        Some(" WHERE current_nft_marketplace_bids.last_transaction_timestamp <= excluded.last_transaction_timestamp "),
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
            .on_conflict((token_data_id, buyer, price))
            .do_nothing(),
        None,
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
            .on_conflict((collection_id, buyer, price))
            .do_update()
            .set((
                is_deleted.eq(excluded(is_deleted)),
            )),
        Some(" WHERE current_nft_marketplace_collection_bids.last_transaction_timestamp <= excluded.last_transaction_timestamp "),
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
            .on_conflict((collection_id, buyer, price))
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
            .on_conflict((transaction_version, token_id))
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
            .on_conflict((token_id, token_data_id))
            .do_update()
            .set((
                is_deleted.eq(excluded(is_deleted)),
            )),
        Some(" WHERE current_nft_marketplace_listings.last_transaction_timestamp <= excluded.last_transaction_timestamp "),
    )
}
