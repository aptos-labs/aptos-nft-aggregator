use crate::{
    config::marketplace_config::{
        EventModel, EventRemappingConfig, EventType, FieldRemappings, MarketplaceEventMappings,
        MarketplaceEventType, NFTMarketplaceConfig, RemappableColumn, TableMappings, ALL_EVENTS,
    },
    models::nft_models::{
        CurrentNFTMarketplaceCollectionOffer, CurrentNFTMarketplaceListing,
        CurrentNFTMarketplaceTokenOffer, MarketplaceModel, NftMarketplaceActivity,
    },
    steps::{extract_string, HashableJsonPath},
    utils::parse_timestamp,
};
use anyhow::Result;
use aptos_indexer_processor_sdk::utils::convert::{sha3_256, standardize_address};
use aptos_protos::transaction::v1::{transaction::TxnData, Transaction};
use rayon::iter::{
    IntoParallelIterator, IntoParallelRefIterator, ParallelBridge, ParallelIterator,
};
use std::{collections::HashMap, sync::Arc};
use tracing::warn;

pub struct EventRemapper {
    field_remappings: FieldRemappings,
    marketplace_name: String,
}

impl EventRemapper {
    pub fn new(config: &NFTMarketplaceConfig) -> Result<Arc<Self>> {
        let mut field_remappings: FieldRemappings = HashMap::new();

        for (event_type, event_remapping) in &config.events {
            let event_type: EventType = event_type.as_str().try_into()?;
            let mut db_mappings_for_event = HashMap::new();

            for (json_path, db_mappings) in &event_remapping.event_fields {
                let json_path = HashableJsonPath::new(json_path)?;
                let db_mappings = db_mappings
                    .iter()
                    .map(|db_mapping| {
                        // let (column_metadata, move_type) =
                        //     get_metadata_and_move_type(db_schema, db_mapping)?;
                        Ok(RemappableColumn::new(db_mapping.clone()))
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?;

                db_mappings_for_event.insert(json_path, db_mappings);
            }

            field_remappings.insert(event_type, db_mappings_for_event);
        }

        // for (event_type, remappings) in &field_remappings {
        //     println!("Event type: {}, Remappings: {:#?}", event_type, remappings.keys());
        // }
        // println!("Created new EventRemapper with field_remappings: {:#?}", field_remappings);
        Ok(Arc::new(Self {
            field_remappings,
            marketplace_name: config.name.clone(),
        }))
    }

    // The remap_events function:
    // 1. Takes a transaction and activity map as input
    // 2. Extracts events from the transaction data
    // 3. For each event:
    //    - Checks if it matches a known marketplace event type
    //    - Creates a NftMarketplaceActivity with basic transaction info
    //    - Populates activity fields from event data based on table mappings
    //    - Sets token standard (v1 or v2) based on presence of token/collection IDs
    //    - Generates token_data_id and collection_id if missing
    // 4. Stores activities in the activity_map, indexed by either:
    //    - token_data_id if present
    //    - collection_id as fallback
    pub fn remap_events(&self, txn: Transaction) -> Result<(Vec<NftMarketplaceActivity>, Vec<CurrentNFTMarketplaceListing>, Vec<CurrentNFTMarketplaceTokenOffer>, Vec<CurrentNFTMarketplaceCollectionOffer>)> {
        println!(
            "Starting remap_events for transaction version: {}",
            txn.version
        );
        // let mut activities: Vec<NftMarketplaceActivity> = Vec::new();
        // let txn_data = txn.txn_data.as_ref().unwrap();
        let mut activities: Vec<NftMarketplaceActivity> = Vec::new();
        let mut current_token_offers: Vec<CurrentNFTMarketplaceTokenOffer> = Vec::new();
        let mut current_collection_offers: Vec<CurrentNFTMarketplaceCollectionOffer> = Vec::new();
        let mut current_listings: Vec<CurrentNFTMarketplaceListing> = Vec::new();

        let txn_timestamp = parse_timestamp(txn.timestamp.as_ref().unwrap(), txn.version as i64);
        println!("Transaction timestamp: {:?}", txn_timestamp);

        // if let TxnData::User(tx_inner) = txn_data {
        let events = self.get_events(Arc::new(txn))?;
        println!("Found {} events to process", events.len());

        for event in events {
            println!("Processing event type: {}", event.event_type);
            if let Some(remappings) = self.field_remappings.get(&event.event_type) {
                println!("Found remappings for event type");

                // let mut activity = NftMarketplaceActivity::default();
                let mut activity = NftMarketplaceActivity {
                    txn_version: event.transaction_version,
                    index: event.event_index,
                    marketplace: self.marketplace_name.clone(),
                    contract_address: event.account_address.clone(),
                    block_timestamp: txn_timestamp,
                    raw_event_type: event.event_type.to_string(),
                    // standard_event_type: event.event_type.to_string(), // TODO: we need a mapping for this
                    json_data: serde_json::to_value(&event).unwrap(),
                    ..Default::default()
                };

                // Create the appropriate second model based on event type
                // TODO: Make it general so it works with other marketplaces
                let mut secondary_model = match event.event_type.get_struct() {
                    "ListingPlacedEvent" | "ListingCanceledEvent" | "ListingFilledEvent" => {
                        println!(
                            "Creating Listing model for event: {}",
                            event.event_type.get_struct()
                        );
                        activity.standard_event_type = MarketplaceEventType::PlaceListing;
                        Some(SecondaryModel::Listing(
                            CurrentNFTMarketplaceListing::build_default(
                                self.marketplace_name.clone(),
                                &event,
                            ),
                        ))
                    },
                    "TokenOfferPlacedEvent"
                    | "TokenOfferCanceledEvent"
                    | "TokenOfferFilledEvent" => {
                        println!(
                            "Creating TokenOffer model for event: {}",
                            event.event_type.get_struct()
                        );
                        activity.standard_event_type = MarketplaceEventType::PlaceTokenOffer;
                        Some(SecondaryModel::TokenOffer(
                            CurrentNFTMarketplaceTokenOffer::build_default(
                                self.marketplace_name.clone(),
                                &event,
                            ),
                        ))
                    },
                    "CollectionOfferPlacedEvent"
                    | "CollectionOfferCanceledEvent"
                    | "CollectionOfferFilledEvent" => {
                        println!(
                            "Creating CollectionOffer model for event: {}",
                            event.event_type.get_struct()
                        );
                        activity.standard_event_type = MarketplaceEventType::PlaceCollectionOffer;
                        Some(SecondaryModel::CollectionOffer(
                            CurrentNFTMarketplaceCollectionOffer::build_default(
                                self.marketplace_name.clone(),
                                &event,
                            ),
                        ))
                    },
                    _ => {
                        // warn!("Skipping event type '{}'- not a recognized marketplace event", event.event_type.get_struct());
                        continue;
                    },
                };

                // let marketplace_name = self.marketplace_name.clone();
                // Step 1: Build model structs from the values obtained by the JsonPaths
                remappings.iter().try_for_each(|(json_path, db_mappings)| {
                    // println!("Processing JSON path: {:?}", json_path);
                    db_mappings.iter().try_for_each(|db_mapping| {
                        let extracted_value = json_path.extract_from(&event.data)?;
                        // println!("Extracted value for path {}: {:?}", json_path, extracted_value);
                        let value = extracted_value
                            .as_str()
                            .map(|s| s.to_string())
                            .or_else(|| extracted_value.as_u64().map(|n| n.to_string()))
                            .unwrap_or_default();

                        match TableType::from_str(db_mapping.db_column.table.as_str()) {
                            Some(TableType::Activities) => {
                                println!(
                                    "Setting activity field {} = {}",
                                    db_mapping.db_column.column, value
                                );
                                activity.set_field(&db_mapping.db_column.column, value);
                            },
                            Some(_) => {
                                if let Some(model) = &mut secondary_model {
                                    println!(
                                        "Setting {} field {} = {}",
                                        model.table_name(),
                                        db_mapping.db_column.column,
                                        value
                                    );
                                    model.set_field(&db_mapping.db_column.column, value);
                                }
                            },
                            None => anyhow::bail!("Unknown table: {}", db_mapping.db_column.table),
                        }

                        anyhow::Ok(())
                    })
                })?;

                println!("activity: {:#?}", activity);
                activities.push(activity);

                // Push the secondary model if it exists and is valid
                if let Some(model) = secondary_model {
                    if model.is_valid() {
                        println!("Secondary model is valid, pushing to appropriate vector");
                        match model {
                            SecondaryModel::Listing(listing) => {
                                current_listings.push(listing);
                            },
                            SecondaryModel::TokenOffer(token_offer) => {
                                current_token_offers.push(token_offer);
                            },
                            SecondaryModel::CollectionOffer(collection_offer) => {
                                current_collection_offers.push(collection_offer);
                            },
                        }
                    }
                }
            }

            // // Store activity in the map only if it has a token_data_id
            // // This ensures we can later match resources to activities
            // // if it's empty it means it's v1
            // if activity.token_data_id.is_none() {
            //     activity.token_data_id = match generate_token_data_id(
            //         activity.creator_address.clone(),
            //         activity.collection_name.clone(),
            //         activity.token_name.clone(),
            //     ) {
            //         Some(token_data_id) => Some(token_data_id),
            //         None => {
            //             debug!(
            //                 "Failed to generate token data id for activity: {:#?}",
            //                 activity
            //             );
            //             None
            //         },
            //     }
            // }

            // // Store activity in the map only if it has a collection_id
            // // This ensures we can later match resources to activities
            // if activity.collection_id.is_none() {
            //     // only if we can generate a collection id
            //     activity.collection_id = match generate_collection_id(
            //         activity.creator_address.clone(),
            //         activity.collection_name.clone(),
            //     ) {
            //         Some(collection_id) => Some(collection_id),
            //         None => {
            //             // V2 events may be missing data to generate collection id
            //             debug!(
            //                 "Failed to generate collection id for activity: {:#?}",
            //                 activity
            //             );
            //             None
            //         },
            //     };
            // }
            // if let Some(ref token_data_id) = activity.token_data_id {
            //     activity_map
            //         .entry(token_data_id.clone())
            //         .or_default()
            //         .insert(standard_event_type.clone(), activity.clone());
            // } else if let Some(ref collection_id) = activity.collection_id {
            //     activity_map
            //         .entry(collection_id.clone())
            //         .or_default()
            //         .insert(standard_event_type.clone(), activity.clone());
            // }
        }
        println!("Finished processing all events. Activities: {}, Listings: {}, Token Offers: {}, Collection Offers: {}", 
            activities.len(), current_listings.len(), current_token_offers.len(), current_collection_offers.len());
        Ok((activities, current_listings, current_token_offers, current_collection_offers))
    }

    fn get_events(&self, transaction: Arc<Transaction>) -> Result<Vec<EventModel>> {
        let txn_version = transaction.version as i64;
        let block_height = transaction.block_height as i64;
        let txn_data = match transaction.txn_data.as_ref() {
            Some(data) => data,
            None => {
                println!("No transaction data found for version {}", txn_version);
                return Ok(vec![]);
            },
        };
        let txn_timestamp = parse_timestamp(transaction.timestamp.as_ref().unwrap(), txn_version);
        let default = vec![];
        let raw_events = match txn_data {
            TxnData::User(tx_inner) => &tx_inner.events,
            _ => &default,
        };
        println!(
            "Found {} raw events in transaction {}",
            raw_events.len(),
            txn_version
        );
        EventModel::from_events(raw_events, txn_version, block_height, txn_timestamp)
    }
}

fn generate_token_data_id(
    creator_address: Option<String>,
    collection_name: Option<String>,
    token_name: Option<String>,
) -> Option<String> {
    match (creator_address, collection_name, token_name) {
        (Some(creator), Some(collection), Some(token))
            if !creator.is_empty() && !collection.is_empty() && !token.is_empty() =>
        {
            let creator_address = standardize_address(&creator);
            let input = format!("{}::{}::{}", creator_address, collection, token);
            let hash = sha3_256(input.as_bytes());
            Some(standardize_address(&hex::encode(hash)))
        },
        _ => None,
    }
}

fn generate_collection_id(
    creator_address: Option<String>,
    collection_name: Option<String>,
) -> Option<String> {
    match (creator_address, collection_name) {
        (Some(creator), Some(collection)) if !creator.is_empty() && !collection.is_empty() => {
            let creator_address = standardize_address(&creator);
            let input = format!("{}::{}", creator_address, collection);
            let hash = sha3_256(input.as_bytes());
            Some(standardize_address(&hex::encode(hash)))
        },
        _ => None,
    }
}

enum SecondaryModel {
    Listing(CurrentNFTMarketplaceListing),
    TokenOffer(CurrentNFTMarketplaceTokenOffer),
    CollectionOffer(CurrentNFTMarketplaceCollectionOffer),
}

impl MarketplaceModel for SecondaryModel {
    fn set_field(&mut self, column: &str, value: String) {
        match self {
            SecondaryModel::Listing(l) => l.set_field(column, value),
            SecondaryModel::TokenOffer(t) => t.set_field(column, value),
            SecondaryModel::CollectionOffer(c) => c.set_field(column, value),
        }
    }

    fn is_valid(&self) -> bool {
        match self {
            SecondaryModel::Listing(l) => l.is_valid(),
            SecondaryModel::TokenOffer(t) => t.is_valid(),
            SecondaryModel::CollectionOffer(c) => c.is_valid(),
        }
    }

    fn table_name(&self) -> &'static str {
        match self {
            SecondaryModel::Listing(_) => "current_nft_marketplace_listings",
            SecondaryModel::TokenOffer(_) => "current_nft_marketplace_token_offers",
            SecondaryModel::CollectionOffer(_) => "current_nft_marketplace_collection_offers",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum TableType {
    Activities,
    Listings,
    TokenOffers,
    CollectionOffers,
}

impl TableType {
    fn from_str(table_name: &str) -> Option<Self> {
        match table_name {
            "nft_marketplace_activities" => Some(TableType::Activities),
            "current_nft_marketplace_listings" => Some(TableType::Listings),
            "current_nft_marketplace_token_offers" => Some(TableType::TokenOffers),
            "current_nft_marketplace_collection_offers" => Some(TableType::CollectionOffers),
            _ => None,
        }
    }
}
