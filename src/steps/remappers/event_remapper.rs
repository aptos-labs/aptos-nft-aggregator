use crate::{
    config::marketplace_config::{
        EventFieldRemappings, EventType, MarketplaceEventType, NFTMarketplaceConfig,
        RemappableColumn,
    },
    models::{
        nft_models::{
            CurrentNFTMarketplaceCollectionOffer, CurrentNFTMarketplaceListing,
            CurrentNFTMarketplaceTokenOffer, MarketplaceModel, NftMarketplaceActivity,
        },
        EventModel,
    },
    steps::HashableJsonPath,
    utils::parse_timestamp,
};
use anyhow::Result;
use aptos_indexer_processor_sdk::utils::convert::{sha3_256, standardize_address};
use aptos_protos::transaction::v1::{transaction::TxnData, Transaction};
use std::{collections::HashMap, sync::Arc};
use tracing::warn;

pub struct EventRemapper {
    field_remappings: EventFieldRemappings,
    marketplace_name: String,
    marketplace_event_type_mapping: HashMap<String, MarketplaceEventType>,
}

impl EventRemapper {
    pub fn new(config: &NFTMarketplaceConfig) -> Result<Arc<Self>> {
        let mut field_remappings: EventFieldRemappings = HashMap::new();
        for (event_type, event_remapping) in &config.events {
            let event_type: EventType = event_type.as_str().try_into()?;
            let mut db_mappings_for_event = HashMap::new();

            for (json_path, db_mappings) in &event_remapping.event_fields {
                let json_path = HashableJsonPath::new(json_path)?;
                let db_mappings = db_mappings
                    .iter()
                    .map(|db_mapping| {
                        // We only map json path here for now, might have to support move_type as well.
                        Ok(RemappableColumn::new(db_mapping.clone()))
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?;

                db_mappings_for_event.insert(json_path, db_mappings);
            }

            field_remappings.insert(event_type, db_mappings_for_event);
        }

        Ok(Arc::new(Self {
            field_remappings,
            marketplace_name: config.name.clone(),
            marketplace_event_type_mapping: config.event_model_mapping.clone(),
        }))
    }

    /// Remaps events from a transaction into marketplace activities and current state models
    ///
    /// # Key responsibilities:
    /// 1. Takes a transaction and extracts relevant NFT marketplace events
    /// 2. Maps event fields to database columns based on configured remappings
    /// 3. Creates marketplace activity records for event history
    /// 4. Updates current state models (listings, token offers, collection offers)
    /// 5. Handles different event types (listings, offers, etc) with appropriate model creation
    /// 6. Standardizes data formats (addresses, timestamps, etc)
    /// 7. Maintains data consistency across related models
    /// 8. Provides structured output for database persistence
    ///
    /// # Returns
    /// - Vector of marketplace activities (historical events)
    /// - Vector of current listings
    /// - Vector of current token offers  
    /// - Vector of current collection offers
    pub fn remap_events(
        &self,
        txn: Transaction,
    ) -> Result<(
        Vec<NftMarketplaceActivity>,
        Vec<CurrentNFTMarketplaceListing>,
        Vec<CurrentNFTMarketplaceTokenOffer>,
        Vec<CurrentNFTMarketplaceCollectionOffer>,
    )> {
        let mut activities: Vec<NftMarketplaceActivity> = Vec::new();
        let mut current_token_offers: Vec<CurrentNFTMarketplaceTokenOffer> = Vec::new();
        let mut current_collection_offers: Vec<CurrentNFTMarketplaceCollectionOffer> = Vec::new();
        let mut current_listings: Vec<CurrentNFTMarketplaceListing> = Vec::new();

        let txn_timestamp = parse_timestamp(txn.timestamp.as_ref().unwrap(), txn.version as i64);
        let events = self.get_events(Arc::new(txn))?;

        for event in events {
            if let Some(remappings) = self.field_remappings.get(&event.event_type) {

                let mut activity = NftMarketplaceActivity {
                    txn_version: event.transaction_version,
                    index: event.event_index,
                    marketplace: self.marketplace_name.clone(),
                    contract_address: event.account_address.clone(),
                    block_timestamp: txn_timestamp,
                    raw_event_type: event.event_type.to_string(),
                    json_data: serde_json::to_value(&event).unwrap(),
                    ..Default::default()
                };

                // Step 1: Create the appropriate second model based on event type
                let event_type_str = event.event_type.to_string();
                let mut secondary_model: Option<SecondaryModel> = match self
                    .marketplace_event_type_mapping
                    .get(&event_type_str)
                {
                    Some(MarketplaceEventType::PlaceListing) => {
                        activity.standard_event_type = MarketplaceEventType::PlaceListing.to_string();
                        Some(SecondaryModel::Listing(
                        CurrentNFTMarketplaceListing::build_default(
                            self.marketplace_name.clone(),
                            &event,
                            false,
                            ),
                        ))
                    },
                    Some(MarketplaceEventType::CancelListing) | Some(MarketplaceEventType::FillListing) => {
                        activity.standard_event_type = MarketplaceEventType::CancelListing.to_string();
                        Some(SecondaryModel::Listing(
                            CurrentNFTMarketplaceListing::build_default(
                                self.marketplace_name.clone(),
                                &event,
                                true,
                        ),
                        ))
                    },
                    Some(MarketplaceEventType::PlaceTokenOffer) => {
                        activity.standard_event_type = MarketplaceEventType::PlaceTokenOffer.to_string();
                        Some(SecondaryModel::TokenOffer(
                            CurrentNFTMarketplaceTokenOffer::build_default(
                                self.marketplace_name.clone(),
                                &event,
                                false,
                            ),
                        ))
                    },
                    Some(MarketplaceEventType::CancelTokenOffer) | Some(MarketplaceEventType::FillTokenOffer) => {
                        activity.standard_event_type = MarketplaceEventType::CancelTokenOffer.to_string();
                        Some(SecondaryModel::TokenOffer(CurrentNFTMarketplaceTokenOffer::build_default(
                            self.marketplace_name.clone(),
                            &event,
                            true,
                        ),
                        ))
                    },
                    Some(MarketplaceEventType::PlaceCollectionOffer) => {
                        activity.standard_event_type = MarketplaceEventType::PlaceCollectionOffer.to_string();
                        Some(SecondaryModel::CollectionOffer(
                            CurrentNFTMarketplaceCollectionOffer::build_default(
                                self.marketplace_name.clone(),
                                &event,
                                false,
                            ),
                        ))
                    },
                    Some(MarketplaceEventType::CancelCollectionOffer) | Some(MarketplaceEventType::FillCollectionOffer) => {
                        activity.standard_event_type = MarketplaceEventType::CancelCollectionOffer.to_string();
                        Some(SecondaryModel::CollectionOffer(
                            CurrentNFTMarketplaceCollectionOffer::build_default(
                                self.marketplace_name.clone(),
                                &event,
                                true,
                            ),
                        ))
                    },
                    Some(MarketplaceEventType::Unknown) => {
                        warn!("Skipping unrecognized event type '{}'", event_type_str);
                        continue;
                    },
                    None => {
                        warn!("No remappings found for event type '{}'", event_type_str);
                        continue;
                    },
                };

                // Step 2: Build model structs from the values obtained by the JsonPaths
                remappings.iter().try_for_each(|(json_path, db_mappings)| {
                    db_mappings.iter().try_for_each(|db_mapping| {
                        // Extract value, continue on error instead of failing
                        let extracted_value = match json_path.extract_from(&event.data) {
                            Ok(value) => value,
                            Err(e) => {
                                tracing::debug!(
                                    "Failed to extract value for path {}: {}",
                                    json_path.raw,
                                    e
                                );
                                return Ok::<(), anyhow::Error>(());
                            }
                        };

                        let value = extracted_value
                            .as_str()
                            .map(|s| s.to_string())
                            .or_else(|| extracted_value.as_u64().map(|n| n.to_string()))
                            .unwrap_or_default();

                        // Skip empty values
                        if value.is_empty() {
                            tracing::debug!("Skipping empty value for path {} for column {}", json_path.raw, db_mapping.db_column.column);
                            return Ok(());
                        }

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
                            None => {
                                tracing::warn!("Unknown table: {}", db_mapping.db_column.table);
                                return Ok(());
                            },
                        }

                        Ok(())
                    })
                })?;

                activities.push(activity);
                
                // Push the secondary model if it exists and is valid
                if let Some(model) = secondary_model {
                    if model.is_valid() {
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
        }
        println!("Finished processing all events. Activities: {}, Listings: {}, Token Offers: {}, Collection Offers: {}", 
            activities.len(), current_listings.len(), current_token_offers.len(), current_collection_offers.len());
        Ok((
            activities,
            current_listings,
            current_token_offers,
            current_collection_offers,
        ))
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

#[derive(Debug)]
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

    fn updated_at(&self) -> i64 {
        // Placeholder
        // this shouldn't be called
        panic!("SecondaryModel::updated_at should not be called");
    }

    fn get_field(&self, _column: &str) -> Option<String> {
        panic!("SecondaryModel::get_field should not be called");
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

// mod tests {
//     use super::*;
//     use crate::{
//         config::{marketplace_config::DbColumn, DbColumnConstant, DbSchemaColumnMetadata, EventRemapping},
//         utils::types::MoveType,
//     };
//     use aptos_indexer_processor_sdk::aptos_protos::transaction::v1::{Event, UserTransaction};
//     use sea_query::Value as SeaQueryValue;

//     fn generate_transaction() -> Arc<Transaction> {
//         Arc::new(Transaction {
//             version: 1,
//             block_height: 1,
//             txn_data: Some(TxnData::User(UserTransaction {
//                 request: None,
//                 events: vec![
//                     Event {
//                         key: Some(Default::default()),
//                         sequence_number: 0,
//                         r#type: Some(Default::default()),
//                         type_str: "0x0000000000000001::module::Event1".to_string(),
//                         data: "{\"field1\": \"1\", \"field2\": \"2\"}".to_string(),
//                     },
//                     Event {
//                         key: Some(Default::default()),
//                         sequence_number: 0,
//                         r#type: Some(Default::default()),
//                         type_str: "0x0000000000000001::module::Event2".to_string(),
//                         data: "{\"field1\": \"1\", \"field3\": \"3\"}".to_string(),
//                     },
//                     Event {
//                         key: Some(Default::default()),
//                         sequence_number: 0,
//                         r#type: Some(Default::default()),
//                         type_str: "0x0000000000000001::module::Event2".to_string(),
//                         data: "{\"field1\": \"1\", \"field3\": \"4\"}".to_string(),
//                     },
//                 ],
//             })),
//             timestamp: None,
//             info: None,
//             epoch: 1,
//             r#type: 1,
//             size_info: None,
//         })
//     }
//     #[test]
//     fn test_event_remapper_basic_listing() -> Result<()> {
//             // Create a basic marketplace config
//         let config = NFTMarketplaceConfig {
//             name: "test_marketplace".to_string(),
//             events: {
//                 let mut map = HashMap::new();
//                 map.insert(
//                     "listing_created".to_string(),
//                     EventRemapping {
//                         event_fields: {
//                             let mut fields = HashMap::new();
//                             fields.insert(
//                                 "$.data.price".to_string(),
//                                 vec![RemappableColumn {
//                                     db_column: DbColumn {
//                                         table: "current_nft_marketplace_listings".to_string(),
//                                         column: "price".to_string(),
//                                     },
//                                 }],
//                             );
//                             fields.insert(
//                                 "$.data.token_id".to_string(),
//                                 vec![RemappableColumn {
//                                     db_column: DbColumn {
//                                         table: "current_nft_marketplace_listings".to_string(),
//                                         column: "token_id".to_string(),
//                                     },
//                                 }],
//                             );
//                             fields
//                         },
//                     },
//                 );
//                 map
//             },
//             event_model_mapping: {
//                 let mut map = HashMap::new();
//                 map.insert(
//                     "listing_created".to_string(),
//                     MarketplaceModelType::Listing,
//                 );
//                 map
//             },
//             resources: {
//                 let mut map = HashMap::new();
//                 map
//             },
//         };

//         // Create the event remapper
//         let remapper = EventRemapper::new(&config)?;

//         // Create a test transaction with a listing event
//         let mut transaction = generate_transaction();

//         // Process the transaction
//         let (activities, listings, token_offers, collection_offers) = remapper.remap_events(transaction)?;

//         // Verify the results
//         assert_eq!(activities.len(), 1, "Should have one activity");
//         assert_eq!(listings.len(), 1, "Should have one listing");
//         assert_eq!(token_offers.len(), 0, "Should have no token offers");
//         assert_eq!(collection_offers.len(), 0, "Should have no collection offers");

//         // Verify listing details
//         let listing = &listings[0];
//         assert_eq!(listing.price, 1000000);
//         assert_eq!(listing.token_data_id, Some("token123".to_string()));
//         assert_eq!(listing.marketplace, "test_marketplace");

//         Ok(())
//     }
// }
