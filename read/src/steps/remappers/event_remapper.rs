use crate::{
    config::marketplace_config::{
        EventFieldRemappings, EventType, MarketplaceEventType, NFTMarketplaceConfig,
    },
    models::{
        nft_models::{
            CurrentNFTMarketplaceCollectionOffer, CurrentNFTMarketplaceListing,
            CurrentNFTMarketplaceTokenOffer, MarketplaceField, MarketplaceModel,
            NftMarketplaceActivity,
        },
        EventModel,
    },
    steps::{
        remappers::{SecondaryModel, TableType},
        HashableJsonPath,
    },
};
use anyhow::Result;
use aptos_indexer_processor_sdk::{
    aptos_indexer_transaction_stream::utils::time::parse_timestamp,
    aptos_protos::transaction::v1::{transaction::TxnData, Transaction},
    utils::{convert::standardize_address, extract::hash_str},
};
use std::{collections::HashMap, str::FromStr, sync::Arc};
use tracing::{debug, warn};

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
                        Ok(db_mapping.clone())
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
    /// 3. Creates marketplace activity for event
    /// 4. Updates current models (listings, token offers, collection offers)
    /// 5. Generate necessary id fields for models that don't have an id if possible
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

        let txn_timestamp =
            parse_timestamp(txn.timestamp.as_ref().unwrap(), txn.version as i64).naive_utc();

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

                let mut secondary_model: Option<SecondaryModel> =
                    match self.marketplace_event_type_mapping.get(&event_type_str) {
                        Some(MarketplaceEventType::PlaceListing) => {
                            activity.standard_event_type =
                                MarketplaceEventType::PlaceListing.to_string();
                            Some(SecondaryModel::Listing(
                                CurrentNFTMarketplaceListing::build_default(
                                    self.marketplace_name.clone(),
                                    &event,
                                    false,
                                    MarketplaceEventType::PlaceListing.to_string(),
                                ),
                            ))
                        },
                        Some(MarketplaceEventType::CancelListing) => {
                            activity.standard_event_type =
                                MarketplaceEventType::CancelListing.to_string();
                            Some(SecondaryModel::Listing(
                                CurrentNFTMarketplaceListing::build_default(
                                    self.marketplace_name.clone(),
                                    &event,
                                    true,
                                    MarketplaceEventType::CancelListing.to_string(),
                                ),
                            ))
                        },
                        Some(MarketplaceEventType::FillListing) => {
                            activity.standard_event_type =
                                MarketplaceEventType::FillListing.to_string();
                            Some(SecondaryModel::Listing(
                                CurrentNFTMarketplaceListing::build_default(
                                    self.marketplace_name.clone(),
                                    &event,
                                    true,
                                    MarketplaceEventType::FillListing.to_string(),
                                ),
                            ))
                        },
                        Some(MarketplaceEventType::PlaceTokenOffer) => {
                            activity.standard_event_type =
                                MarketplaceEventType::PlaceTokenOffer.to_string();
                            Some(SecondaryModel::TokenOffer(
                                CurrentNFTMarketplaceTokenOffer::build_default(
                                    self.marketplace_name.clone(),
                                    &event,
                                    false,
                                    MarketplaceEventType::PlaceTokenOffer.to_string(),
                                ),
                            ))
                        },
                        Some(MarketplaceEventType::CancelTokenOffer) => {
                            activity.standard_event_type =
                                MarketplaceEventType::CancelTokenOffer.to_string();
                            Some(SecondaryModel::TokenOffer(
                                CurrentNFTMarketplaceTokenOffer::build_default(
                                    self.marketplace_name.clone(),
                                    &event,
                                    true,
                                    MarketplaceEventType::CancelTokenOffer.to_string(),
                                ),
                            ))
                        },
                        Some(MarketplaceEventType::FillTokenOffer) => {
                            activity.standard_event_type =
                                MarketplaceEventType::FillTokenOffer.to_string();
                            Some(SecondaryModel::TokenOffer(
                                CurrentNFTMarketplaceTokenOffer::build_default(
                                    self.marketplace_name.clone(),
                                    &event,
                                    true,
                                    MarketplaceEventType::FillTokenOffer.to_string(),
                                ),
                            ))
                        },
                        Some(MarketplaceEventType::PlaceCollectionOffer) => {
                            activity.standard_event_type =
                                MarketplaceEventType::PlaceCollectionOffer.to_string();
                            Some(SecondaryModel::CollectionOffer(
                                CurrentNFTMarketplaceCollectionOffer::build_default(
                                    self.marketplace_name.clone(),
                                    &event,
                                    false,
                                    MarketplaceEventType::PlaceCollectionOffer.to_string(),
                                ),
                            ))
                        },
                        Some(MarketplaceEventType::CancelCollectionOffer) => {
                            activity.standard_event_type =
                                MarketplaceEventType::CancelCollectionOffer.to_string();
                            Some(SecondaryModel::CollectionOffer(
                                CurrentNFTMarketplaceCollectionOffer::build_default(
                                    self.marketplace_name.clone(),
                                    &event,
                                    true,
                                    MarketplaceEventType::CancelCollectionOffer.to_string(),
                                ),
                            ))
                        },
                        Some(MarketplaceEventType::FillCollectionOffer) => {
                            activity.standard_event_type =
                                MarketplaceEventType::FillCollectionOffer.to_string();
                            Some(SecondaryModel::CollectionOffer(
                                CurrentNFTMarketplaceCollectionOffer::build_default(
                                    self.marketplace_name.clone(),
                                    &event,
                                    true,
                                    MarketplaceEventType::FillCollectionOffer.to_string(),
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
                                debug!("Failed to extract value for path {}: {}", json_path.raw, e);
                                return Ok::<(), anyhow::Error>(());
                            },
                        };

                        let value = extracted_value
                            .as_str()
                            .map(|s| s.to_string())
                            .or_else(|| extracted_value.as_u64().map(|n| n.to_string()))
                            .unwrap_or_default();

                        if value.is_empty() {
                            debug!(
                                "Skipping empty value for path {} for column {}",
                                json_path.raw, db_mapping.column
                            );
                            return Ok(());
                        }

                        match TableType::from_str(db_mapping.table.as_str()) {
                            Some(TableType::Activities) => {
                                match MarketplaceField::from_str(db_mapping.column.as_str()) {
                                    Ok(field) => {
                                        activity.set_field(field, value);
                                    },
                                    Err(e) => {
                                        warn!(
                                            "Skipping invalid field {}: {}",
                                            db_mapping.column, e
                                        );
                                    },
                                }
                            },
                            Some(_) => {
                                if let Some(model) = &mut secondary_model {
                                    match MarketplaceField::from_str(db_mapping.column.as_str()) {
                                        Ok(field) => {
                                            model.set_field(field, value);
                                        },
                                        Err(e) => {
                                            warn!(
                                                "Skipping invalid field {}: {}",
                                                db_mapping.column, e
                                            );
                                        },
                                    }
                                }
                            },
                            None => {
                                warn!("Unknown table: {}", db_mapping.table);
                                return Ok(());
                            },
                        }

                        Ok(())
                    })
                })?;

                // After processing all field remappings, generate necessary id fields if needed for PK
                if let Some(model) = &mut secondary_model {
                    let creator_address = activity.creator_address.clone();
                    let collection_name = activity.collection_name.clone();
                    let token_name = activity.token_name.clone();

                    match model {
                        SecondaryModel::Listing(listing) => {
                            self.generate_and_set_ids(
                                listing,
                                &mut activity,
                                &creator_address,
                                &collection_name,
                                &token_name,
                            );
                        },
                        SecondaryModel::TokenOffer(token_offer) => {
                            self.generate_and_set_ids(
                                token_offer,
                                &mut activity,
                                &creator_address,
                                &collection_name,
                                &token_name,
                            );
                        },
                        SecondaryModel::CollectionOffer(collection_offer) => {
                            self.generate_and_set_ids(
                                collection_offer,
                                &mut activity,
                                &creator_address,
                                &collection_name,
                                &token_name,
                            );

                            // Handle collection_offer_id separately since it's specific to collection offers
                            if collection_offer.collection_offer_id.is_empty() {
                                if let Some(generated_collection_offer_id) =
                                    generate_collection_offer_id(
                                        creator_address,
                                        activity.buyer.clone(),
                                    )
                                {
                                    collection_offer.collection_offer_id =
                                        generated_collection_offer_id.clone();
                                    activity.set_field(
                                        MarketplaceField::CollectionOfferId,
                                        generated_collection_offer_id,
                                    );
                                }
                            }
                        },
                    }
                }

                // Pass only if secondary model is valid
                if let Some(model) = secondary_model {
                    if model.is_valid() {
                        match model {
                            SecondaryModel::Listing(listing) => {
                                activities.push(activity);
                                current_listings.push(listing);
                            },
                            SecondaryModel::TokenOffer(token_offer) => {
                                activities.push(activity);
                                current_token_offers.push(token_offer);
                            },
                            SecondaryModel::CollectionOffer(collection_offer) => {
                                activities.push(activity);
                                current_collection_offers.push(collection_offer);
                            },
                        }
                    } else {
                        debug!("Secondary model validation failed, skipping: {:?}", model);
                    }
                }
            }
        }

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
                debug!("No transaction data found for version {}", txn_version);
                return Ok(vec![]);
            },
        };
        let txn_timestamp =
            parse_timestamp(transaction.timestamp.as_ref().unwrap(), txn_version).naive_utc();
        let default = vec![];
        let raw_events = match txn_data {
            TxnData::User(tx_inner) => tx_inner.events.as_slice(),
            _ => &default,
        };
        EventModel::from_events(raw_events, txn_version, block_height, txn_timestamp)
    }

    // Helper function to generate and set IDs for a model
    fn generate_and_set_ids(
        &self,
        model: &mut impl MarketplaceModel,
        activity: &mut NftMarketplaceActivity,
        creator_address: &Option<String>,
        collection_name: &Option<String>,
        token_name: &Option<String>,
    ) {
        // Generate token_data_id if needed
        if model
            .get_field(MarketplaceField::TokenDataId)
            .unwrap_or_default()
            .is_empty()
        {
            let generated_token_data_id = generate_token_data_id(
                creator_address.clone(),
                collection_name.clone(),
                token_name.clone(),
            );
            if let Some(id) = generated_token_data_id {
                model.set_field(MarketplaceField::TokenDataId, id.clone());
                activity.set_field(MarketplaceField::TokenDataId, id);
            }
        }

        // Generate collection_id if needed
        if model
            .get_field(MarketplaceField::CollectionId)
            .unwrap_or_default()
            .is_empty()
        {
            if let Some(generated_collection_id) =
                generate_collection_id(creator_address.clone(), collection_name.clone())
            {
                model.set_field(
                    MarketplaceField::CollectionId,
                    generated_collection_id.clone(),
                );
                activity.set_field(MarketplaceField::CollectionId, generated_collection_id);
            }
        }
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
            let input = format!("{creator_address}::{collection}::{token}");
            let hash_str = hash_str(&input);
            Some(standardize_address(&hash_str))
        },
        _ => {
            debug!("Missing required fields for token data id generation - skipping");
            None
        },
    }
}

fn generate_collection_id(
    creator_address: Option<String>,
    collection_name: Option<String>,
) -> Option<String> {
    match (creator_address, collection_name) {
        (Some(creator), Some(collection)) if !creator.is_empty() && !collection.is_empty() => {
            let creator_address = standardize_address(&creator);
            let input = format!("{creator_address}::{collection}");
            let hash_str = hash_str(&input);
            Some(standardize_address(&hash_str))
        },
        _ => {
            debug!("Missing required fields for collection id generation - skipping");
            None
        },
    }
}

#[allow(dead_code)]
fn generate_collection_offer_id(
    creator_address: Option<String>,
    buyer: Option<String>,
) -> Option<String> {
    match (creator_address, buyer) {
        (Some(creator), Some(buyer)) if !creator.is_empty() && !buyer.is_empty() => {
            let creator_address = standardize_address(&creator);
            let buyer_address = standardize_address(&buyer);
            let input = format!("{creator_address}::{buyer_address}");
            let hash_str = hash_str(&input);
            Some(standardize_address(&hash_str))
        },
        _ => {
            debug!("Missing required fields for collection offer id generation - skipping");
            None
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::marketplace_config::{DbColumn, EventRemapping};
    use aptos_indexer_processor_sdk::aptos_protos::{
        transaction::v1::{Event, UserTransaction},
        util::timestamp::Timestamp,
    };

    fn create_db_column(table: &str, column: &str) -> DbColumn {
        DbColumn {
            table: table.to_string(),
            column: column.to_string(),
        }
    }

    fn create_marketplace_config(
        event_type: &str,
        fields: HashMap<String, Vec<DbColumn>>,
        event_model_type: MarketplaceEventType,
    ) -> NFTMarketplaceConfig {
        NFTMarketplaceConfig {
            name: "test_marketplace".to_string(),
            events: {
                let mut map = HashMap::new();
                map.insert(event_type.to_string(), EventRemapping {
                    event_fields: fields,
                });
                map
            },
            event_model_mapping: {
                let mut map = HashMap::new();
                map.insert(event_type.to_string(), event_model_type);
                map
            },
            resources: HashMap::new(),
        }
    }

    fn create_transaction(event_type: &str, event_data: serde_json::Value) -> Transaction {
        Transaction {
            version: 1,
            block_height: 1,
            txn_data: Some(TxnData::User(UserTransaction {
                request: None,
                events: vec![Event {
                    key: Some(Default::default()),
                    sequence_number: 0,
                    r#type: Some(Default::default()),
                    type_str: event_type.to_string(),
                    data: event_data.to_string(),
                }],
            })),
            timestamp: Some(Timestamp {
                seconds: 1,
                nanos: 0,
            }),
            info: None,
            epoch: 1,
            r#type: 1,
            size_info: None,
        }
    }

    fn create_listing_field_mappings() -> HashMap<String, Vec<DbColumn>> {
        let mut fields = HashMap::new();
        fields.insert("$.price".to_string(), vec![
            create_db_column("current_nft_marketplace_listings", "price"),
            create_db_column("nft_marketplace_activities", "price"),
        ]);
        fields.insert("$.token_metadata.token.vec[0].inner".to_string(), vec![
            create_db_column("current_nft_marketplace_listings", "token_data_id"),
            create_db_column("nft_marketplace_activities", "token_data_id"),
        ]);
        fields.insert("$.seller".to_string(), vec![create_db_column(
            "current_nft_marketplace_listings",
            "seller",
        )]);
        fields.insert("$.token_metadata.collection_name".to_string(), vec![
            create_db_column("current_nft_marketplace_listings", "collection_name"),
        ]);
        fields.insert("$.token_metadata.creator_address".to_string(), vec![
            create_db_column("current_nft_marketplace_listings", "creator_address"),
        ]);
        fields.insert("$.token_metadata.token_name".to_string(), vec![
            create_db_column("current_nft_marketplace_listings", "token_name"),
        ]);
        fields.insert("$.token_offer".to_string(), vec![create_db_column(
            "current_nft_marketplace_listings",
            "listing_id",
        )]);
        fields
    }

    fn build_test_token_data_id(creator: &str, collection: &str, token: &str) -> String {
        let creator_address = standardize_address(creator);
        let input = format!("{creator_address}::{collection}::{token}");
        let hash_str = hash_str(&input);
        standardize_address(&hash_str)
    }

    #[test]
    fn test_listing_placed_event() -> Result<()> {
        let event_type = "0x584b50b999c78ade62f8359c91b5165ff390338d45f8e55969a04e65d76258c9::events::ListingPlacedEvent";
        let event_data = serde_json::json!({
            "commission": "51000000",
            "price": "3400000000",
            "purchaser": "0x22113f16f9b7c6761ef14df757c016b8736a9023e8881cd5e11579b0b98ef562",
            "royalties": "142800000",
            "seller": "0xc60f124dc24f4ea97232bc5ead5f37252b7cbee47f48ef05932998050c414d14",
            "token_metadata": {
                "collection": {
                    "vec": [
                        {
                            "inner": "0xa2485c3b392d211770ed161e73a1097d21016c7dd41f53592434380b2aa14cba"
                        }
                    ]
                },
                "collection_name": "The Loonies",
                "creator_address": "0xf54f8f7ffc2b779d81b721b3d42fe9a53f96e1d3459a8001934307783d493725",
                "property_version": {
                    "vec": []
                },
                "token": {
                    "vec": [
                        {
                            "inner": "0xc821b5c1712fca97553c85830b91dc212cd2fcdd2a2490b65f945ed901d9f126"
                        }
                    ]
                },
                "token_name": "The Loonies #399"
            },
            "token_offer": "0x9d14c489b6f56ac55e8707022400c23bb83bd0b0cd486c862defccf6241a219e"
        });

        let config = create_marketplace_config(
            event_type,
            create_listing_field_mappings(),
            MarketplaceEventType::PlaceListing,
        );

        let remapper = EventRemapper::new(&config)?;
        let transaction = create_transaction(event_type, event_data);
        let (activities, listings, token_offers, collection_offers) =
            remapper.remap_events(transaction)?;

        // Verify results
        assert_eq!(activities.len(), 1, "Should have one activity");
        assert_eq!(listings.len(), 1, "Should have one listing");
        assert_eq!(token_offers.len(), 0, "Should have no token offers");
        assert_eq!(
            collection_offers.len(),
            0,
            "Should have no collection offers"
        );

        // Verify listing details
        let listing = &listings[0];
        assert_eq!(listing.price, 3400000000);
        assert_eq!(
            listing.token_data_id,
            "0xc821b5c1712fca97553c85830b91dc212cd2fcdd2a2490b65f945ed901d9f126"
        );
        assert_eq!(
            listing.seller,
            Some("0xc60f124dc24f4ea97232bc5ead5f37252b7cbee47f48ef05932998050c414d14".to_string())
        );
        assert_eq!(
            listing.listing_id.as_deref().unwrap(),
            "0x9d14c489b6f56ac55e8707022400c23bb83bd0b0cd486c862defccf6241a219e"
        );
        assert_eq!(listing.marketplace, "test_marketplace");
        assert!(!listing.is_deleted);

        Ok(())
    }

    #[test]
    fn test_listing_filled_event_v1_token() -> Result<()> {
        let event_type = "0x584b50b999c78ade62f8359c91b5165ff390338d45f8e55969a04e65d76258c9::events::ListingFilledEvent";
        let event_data = serde_json::json!({
            "buyer": "0x735507953f702ddad6dbf5a98de6fd3f57f50b89da9c68672414d8431f103726",
            "owner": "0x8c557bb0a12d47c1eda90dd4883b44674111b915fa39ff862e6a0a39140dcd4",
            "price": "398000000",
            "timestamp": "1739908340290288",
            "token_id": {
                "property_version": "1",
                "token_data_id": {
                    "collection": "Bruh Bears",
                    "creator": "0x43ec2cb158e3569842d537740fd53403e992b9e7349cc5d3dfaa5aff8faaef2",
                    "name": "Bruh Bear #3770"
                }
            }
        });

        // Update field mappings to match YAML config
        let mut fields = HashMap::new();
        fields.insert("$.token_metadata.token.vec[0].inner".to_string(), vec![
            create_db_column("nft_marketplace_activities", "token_data_id"),
            create_db_column("current_nft_marketplace_listings", "token_data_id"),
        ]);
        fields.insert("$.token_metadata.token_name".to_string(), vec![
            create_db_column("nft_marketplace_activities", "token_name"),
        ]);
        fields.insert("$.token_metadata.creator_address".to_string(), vec![
            create_db_column("nft_marketplace_activities", "creator_address"),
        ]);
        fields.insert("$.token_metadata.collection_name".to_string(), vec![
            create_db_column("nft_marketplace_activities", "collection_name"),
        ]);
        fields.insert("$.price".to_string(), vec![
            create_db_column("nft_marketplace_activities", "price"),
            create_db_column("current_nft_marketplace_listings", "price"),
        ]);
        fields.insert("$.purchaser".to_string(), vec![create_db_column(
            "nft_marketplace_activities",
            "buyer",
        )]);
        fields.insert("$.seller".to_string(), vec![
            create_db_column("nft_marketplace_activities", "seller"),
            create_db_column("current_nft_marketplace_listings", "seller"),
        ]);
        fields.insert("$.token_amount".to_string(), vec![
            create_db_column("nft_marketplace_activities", "token_amount"),
            create_db_column("current_nft_marketplace_listings", "token_amount"),
        ]);
        fields.insert("$.listing".to_string(), vec![
            create_db_column("nft_marketplace_activities", "listing_id"),
            create_db_column("current_nft_marketplace_listings", "listing_id"),
        ]);
        fields.insert("$.token_id.token_data_id.name".to_string(), vec![
            create_db_column("current_nft_marketplace_listings", "token_name"),
            create_db_column("nft_marketplace_activities", "token_name"),
        ]);
        fields.insert("$.token_id.token_data_id.creator".to_string(), vec![
            create_db_column("current_nft_marketplace_listings", "creator_address"),
            create_db_column("nft_marketplace_activities", "creator_address"),
        ]);
        fields.insert("$.token_id.token_data_id.collection".to_string(), vec![
            create_db_column("current_nft_marketplace_listings", "collection_name"),
            create_db_column("nft_marketplace_activities", "collection_name"),
        ]);
        fields.insert("$.buyer".to_string(), vec![
            create_db_column("nft_marketplace_activities", "buyer"),
            create_db_column("current_nft_marketplace_listings", "buyer"),
        ]);
        fields.insert("$.owner".to_string(), vec![
            create_db_column("nft_marketplace_activities", "seller"),
            create_db_column("current_nft_marketplace_listings", "seller"),
        ]);
        let config =
            create_marketplace_config(event_type, fields, MarketplaceEventType::FillListing);

        let remapper = EventRemapper::new(&config)?;
        let transaction = create_transaction(event_type, event_data);
        let (activities, listings, token_offers, collection_offers) =
            remapper.remap_events(transaction)?;

        // Verify results
        assert_eq!(activities.len(), 1, "Should have one activity");
        assert_eq!(listings.len(), 1, "Should have one listing");
        assert_eq!(token_offers.len(), 0, "Should have no token offers");
        assert_eq!(
            collection_offers.len(),
            0,
            "Should have no collection offers"
        );

        // Verify activity details
        let activity = &activities[0];
        assert_eq!(activity.price, 398000000);
        assert_eq!(
            activity.buyer.as_deref().unwrap(),
            "0x735507953f702ddad6dbf5a98de6fd3f57f50b89da9c68672414d8431f103726"
        );
        assert_eq!(
            activity.seller.as_deref().unwrap(),
            "0x8c557bb0a12d47c1eda90dd4883b44674111b915fa39ff862e6a0a39140dcd4"
        );
        assert_eq!(
            activity.creator_address.as_deref().unwrap(),
            "0x43ec2cb158e3569842d537740fd53403e992b9e7349cc5d3dfaa5aff8faaef2"
        );
        assert_eq!(activity.collection_name.as_deref().unwrap(), "Bruh Bears");
        assert_eq!(activity.token_name.as_deref().unwrap(), "Bruh Bear #3770");
        assert_eq!(activity.marketplace, "test_marketplace");
        assert_eq!(activity.standard_event_type, "fill_listing");

        // Verify listing details
        let listing = &listings[0];
        assert_eq!(listing.price, 398000000);
        assert_eq!(
            listing.seller,
            Some("0x8c557bb0a12d47c1eda90dd4883b44674111b915fa39ff862e6a0a39140dcd4".to_string())
        );
        assert_eq!(listing.marketplace, "test_marketplace");
        assert!(listing.is_deleted);

        Ok(())
    }

    #[test]
    fn test_listing_canceled_event_v2_token() -> Result<()> {
        let event_type = "0x584b50b999c78ade62f8359c91b5165ff390338d45f8e55969a04e65d76258c9::events::ListingCanceledEvent";
        let event_data = serde_json::json!({
            "listing": "0x560197dcdc27af1cadc1cc75b51d9f0e3a0f40d7a761397c13bfdb4097924c1f",
            "price": "3200000000",
            "seller": "0xecd896bfa7eae31fb5085dca8e2f3c88ea3577bd54fafeaeb4ad6ede1e13e81e",
            "token_metadata": {
                "collection": {
                    "vec": [
                        {
                            "inner": "0xa2485c3b392d211770ed161e73a1097d21016c7dd41f53592434380b2aa14cba"
                        }
                    ]
                },
                "collection_name": "The Loonies",
                "creator_address": "0xf54f8f7ffc2b779d81b721b3d42fe9a53f96e1d3459a8001934307783d493725",
                "property_version": {
                    "vec": []
                },
                "token": {
                    "vec": [
                        {
                            "inner": "0xa8b76ee68f7574dafb6f19988880c16571ccd10ac159a8684067a9fc0df293"
                        }
                    ]
                },
                "token_name": "The Loonies #3210"
            },
            "type": "fixed price"
        });

        // Create field mappings
        let mut fields = HashMap::new();
        fields.insert("$.token_metadata.token.vec[0].inner".to_string(), vec![
            create_db_column("nft_marketplace_activities", "token_data_id"),
            create_db_column("current_nft_marketplace_listings", "token_data_id"),
        ]);
        fields.insert("$.token_metadata.token_name".to_string(), vec![
            create_db_column("nft_marketplace_activities", "token_name"),
            create_db_column("current_nft_marketplace_listings", "token_name"),
        ]);
        fields.insert("$.token_metadata.creator_address".to_string(), vec![
            create_db_column("nft_marketplace_activities", "creator_address"),
            create_db_column("current_nft_marketplace_listings", "creator_address"),
        ]);
        fields.insert("$.token_metadata.collection_name".to_string(), vec![
            create_db_column("nft_marketplace_activities", "collection_name"),
            create_db_column("current_nft_marketplace_listings", "collection_name"),
        ]);
        fields.insert("$.price".to_string(), vec![
            create_db_column("nft_marketplace_activities", "price"),
            create_db_column("current_nft_marketplace_listings", "price"),
        ]);
        fields.insert("$.seller".to_string(), vec![
            create_db_column("nft_marketplace_activities", "seller"),
            create_db_column("current_nft_marketplace_listings", "seller"),
        ]);
        fields.insert("$.listing".to_string(), vec![
            create_db_column("nft_marketplace_activities", "listing_id"),
            create_db_column("current_nft_marketplace_listings", "listing_id"),
        ]);

        let config =
            create_marketplace_config(event_type, fields, MarketplaceEventType::CancelListing);

        let remapper = EventRemapper::new(&config)?;
        let transaction = create_transaction(event_type, event_data);
        let (activities, listings, token_offers, collection_offers) =
            remapper.remap_events(transaction)?;

        // Verify results
        assert_eq!(activities.len(), 1, "Should have one activity");
        assert_eq!(listings.len(), 1, "Should have one listing");
        assert_eq!(token_offers.len(), 0, "Should have no token offers");
        assert_eq!(
            collection_offers.len(),
            0,
            "Should have no collection offers"
        );

        // Verify activity details
        let activity = &activities[0];
        assert_eq!(activity.price, 3200000000);
        assert_eq!(
            activity.seller.as_deref().unwrap(),
            "0xecd896bfa7eae31fb5085dca8e2f3c88ea3577bd54fafeaeb4ad6ede1e13e81e"
        );
        assert_eq!(
            activity.creator_address.as_deref().unwrap(),
            "0xf54f8f7ffc2b779d81b721b3d42fe9a53f96e1d3459a8001934307783d493725"
        );
        assert_eq!(activity.collection_name.as_deref().unwrap(), "The Loonies");
        assert_eq!(activity.token_name.as_deref().unwrap(), "The Loonies #3210");
        assert_eq!(
            activity.listing_id.as_deref().unwrap(),
            "0x560197dcdc27af1cadc1cc75b51d9f0e3a0f40d7a761397c13bfdb4097924c1f"
        );
        assert_eq!(activity.marketplace, "test_marketplace");
        assert_eq!(activity.standard_event_type, "cancel_listing");

        // Verify listing details
        let listing = &listings[0];
        assert_eq!(listing.price, 3200000000);
        assert_eq!(
            listing.seller,
            Some("0xecd896bfa7eae31fb5085dca8e2f3c88ea3577bd54fafeaeb4ad6ede1e13e81e".to_string())
        );
        assert_eq!(
            listing.listing_id.as_deref().unwrap(),
            "0x560197dcdc27af1cadc1cc75b51d9f0e3a0f40d7a761397c13bfdb4097924c1f"
        );
        assert_eq!(listing.marketplace, "test_marketplace");
        assert!(listing.is_deleted);

        Ok(())
    }

    #[test]
    fn test_token_offer_placed_event() -> Result<()> {
        let event_type = "0x584b50b999c78ade62f8359c91b5165ff390338d45f8e55969a04e65d76258c9::events::TokenOfferPlacedEvent";
        let event_data = serde_json::json!({
            "price": "25000000",
            "purchaser": "0x62928b3712d452190346090807d5cfb40dabb54740cf1d2acfc5b4d3d9e0b370",
            "token_metadata": {
                "collection": {
                    "vec": []
                },
                "collection_name": "Aptos Dogs",
                "creator_address": "0xee814d743d2c3b4b1b8b30f3e0c84c7017df3154bda84c31958785f1d5b70e61",
                "property_version": {
                    "vec": [
                        "0"
                    ]
                },
                "token": {
                    "vec": []
                },
                "token_name": "AptosDogs #1596"
            },
            "token_offer": "0xdd69203952afa9962f3277f2be027fad3d21d57b986a61d674279d5e395323e"
        });

        // Create field mappings
        let mut fields = HashMap::new();
        fields.insert("$.token_metadata.token_name".to_string(), vec![
            create_db_column("nft_marketplace_activities", "token_name"),
            create_db_column("current_nft_marketplace_token_offers", "token_name"),
        ]);
        fields.insert("$.token_metadata.creator_address".to_string(), vec![
            create_db_column("nft_marketplace_activities", "creator_address"),
            create_db_column("current_nft_marketplace_token_offers", "creator_address"),
        ]);
        fields.insert("$.token_metadata.collection_name".to_string(), vec![
            create_db_column("nft_marketplace_activities", "collection_name"),
            create_db_column("current_nft_marketplace_token_offers", "collection_name"),
        ]);
        fields.insert("$.price".to_string(), vec![
            create_db_column("nft_marketplace_activities", "price"),
            create_db_column("current_nft_marketplace_token_offers", "price"),
        ]);
        fields.insert("$.purchaser".to_string(), vec![
            create_db_column("nft_marketplace_activities", "buyer"),
            create_db_column("current_nft_marketplace_token_offers", "buyer"),
        ]);
        fields.insert("$.token_offer".to_string(), vec![
            create_db_column("nft_marketplace_activities", "offer_id"),
            create_db_column("current_nft_marketplace_token_offers", "offer_id"),
        ]);

        let config =
            create_marketplace_config(event_type, fields, MarketplaceEventType::PlaceTokenOffer);

        let remapper = EventRemapper::new(&config)?;
        let transaction = create_transaction(event_type, event_data);
        let (activities, listings, token_offers, collection_offers) =
            remapper.remap_events(transaction)?;

        // Verify results
        assert_eq!(activities.len(), 1, "Should have one activity");
        assert_eq!(listings.len(), 0, "Should have no listings");
        assert_eq!(token_offers.len(), 1, "Should have one token offer");
        assert_eq!(
            collection_offers.len(),
            0,
            "Should have no collection offers"
        );

        // Verify activity details
        let activity = &activities[0];
        assert_eq!(activity.price, 25000000);
        assert_eq!(
            activity.buyer.as_deref().unwrap(),
            "0x62928b3712d452190346090807d5cfb40dabb54740cf1d2acfc5b4d3d9e0b370"
        );
        assert_eq!(
            activity.creator_address.as_deref().unwrap(),
            "0xee814d743d2c3b4b1b8b30f3e0c84c7017df3154bda84c31958785f1d5b70e61"
        );
        assert_eq!(activity.collection_name.as_deref().unwrap(), "Aptos Dogs");
        assert_eq!(activity.token_name.as_deref().unwrap(), "AptosDogs #1596");
        assert_eq!(
            activity.offer_id.as_deref().unwrap(),
            "0xdd69203952afa9962f3277f2be027fad3d21d57b986a61d674279d5e395323e"
        );
        assert_eq!(activity.marketplace, "test_marketplace");
        assert_eq!(activity.standard_event_type, "place_token_offer");

        // Verify token offer details
        let token_offer = &token_offers[0];
        let expected_token_data_id = build_test_token_data_id(
            "0xee814d743d2c3b4b1b8b30f3e0c84c7017df3154bda84c31958785f1d5b70e61",
            "Aptos Dogs",
            "AptosDogs #1596",
        );
        assert_eq!(token_offer.token_data_id, expected_token_data_id);
        assert_eq!(token_offer.price, 25000000);
        assert_eq!(
            token_offer.buyer,
            "0x62928b3712d452190346090807d5cfb40dabb54740cf1d2acfc5b4d3d9e0b370"
        );

        assert_eq!(
            token_offer.token_name.as_deref().unwrap(),
            "AptosDogs #1596"
        );
        assert_eq!(
            token_offer.offer_id.as_deref().unwrap(),
            "0xdd69203952afa9962f3277f2be027fad3d21d57b986a61d674279d5e395323e"
        );
        assert_eq!(token_offer.marketplace, "test_marketplace");
        assert!(!token_offer.is_deleted);

        Ok(())
    }

    #[test]
    fn test_token_offer_filled_event() -> Result<()> {
        let event_type = "0x584b50b999c78ade62f8359c91b5165ff390338d45f8e55969a04e65d76258c9::events::TokenOfferFilledEvent";
        let event_data = serde_json::json!({
            "commission": "51000000",
            "price": "3400000000",
            "purchaser": "0x22113f16f9b7c6761ef14df757c016b8736a9023e8881cd5e11579b0b98ef562",
            "royalties": "142800000",
            "seller": "0xc60f124dc24f4ea97232bc5ead5f37252b7cbee47f48ef05932998050c414d14",
            "token_metadata": {
                "collection": {
                    "vec": [
                        {
                            "inner": "0xa2485c3b392d211770ed161e73a1097d21016c7dd41f53592434380b2aa14cba"
                        }
                    ]
                },
                "collection_name": "The Loonies",
                "creator_address": "0xf54f8f7ffc2b779d81b721b3d42fe9a53f96e1d3459a8001934307783d493725",
                "property_version": {
                    "vec": []
                },
                "token": {
                    "vec": [
                        {
                            "inner": "0xc821b5c1712fca97553c85830b91dc212cd2fcdd2a2490b65f945ed901d9f126"
                        }
                    ]
                },
                "token_name": "The Loonies #399"
            },
            "token_offer": "0x9d14c489b6f56ac55e8707022400c23bb83bd0b0cd486c862defccf6241a219e"
        });

        // Create field mappings
        let mut fields = HashMap::new();
        fields.insert("$.token_metadata.token.vec[0].inner".to_string(), vec![
            create_db_column("nft_marketplace_activities", "token_data_id"),
            create_db_column("current_nft_marketplace_token_offers", "token_data_id"),
        ]);
        fields.insert("$.token_metadata.token_name".to_string(), vec![
            create_db_column("nft_marketplace_activities", "token_name"),
            create_db_column("current_nft_marketplace_token_offers", "token_name"),
        ]);
        fields.insert("$.token_metadata.creator_address".to_string(), vec![
            create_db_column("nft_marketplace_activities", "creator_address"),
            create_db_column("current_nft_marketplace_token_offers", "creator_address"),
        ]);
        fields.insert("$.token_metadata.collection_name".to_string(), vec![
            create_db_column("nft_marketplace_activities", "collection_name"),
            create_db_column("current_nft_marketplace_token_offers", "collection_name"),
        ]);
        fields.insert("$.price".to_string(), vec![
            create_db_column("nft_marketplace_activities", "price"),
            create_db_column("current_nft_marketplace_token_offers", "price"),
        ]);
        fields.insert("$.seller".to_string(), vec![
            create_db_column("nft_marketplace_activities", "seller"),
            create_db_column("current_nft_marketplace_token_offers", "seller"),
        ]);
        fields.insert("$.purchaser".to_string(), vec![
            create_db_column("nft_marketplace_activities", "buyer"),
            create_db_column("current_nft_marketplace_token_offers", "buyer"),
        ]);
        fields.insert("$.token_offer".to_string(), vec![
            create_db_column("nft_marketplace_activities", "offer_id"),
            create_db_column("current_nft_marketplace_token_offers", "offer_id"),
        ]);

        let config =
            create_marketplace_config(event_type, fields, MarketplaceEventType::FillTokenOffer);

        let remapper = EventRemapper::new(&config)?;
        let transaction = create_transaction(event_type, event_data);
        let (activities, listings, token_offers, collection_offers) =
            remapper.remap_events(transaction)?;

        // Verify results
        assert_eq!(activities.len(), 1, "Should have one activity");
        assert_eq!(listings.len(), 0, "Should have no listings");
        assert_eq!(token_offers.len(), 1, "Should have one token offer");
        assert_eq!(
            collection_offers.len(),
            0,
            "Should have no collection offers"
        );

        // Verify activity details
        let activity = &activities[0];
        assert_eq!(activity.price, 3400000000);
        assert_eq!(
            activity.buyer.as_deref().unwrap(),
            "0x22113f16f9b7c6761ef14df757c016b8736a9023e8881cd5e11579b0b98ef562"
        );
        assert_eq!(
            activity.seller.as_deref().unwrap(),
            "0xc60f124dc24f4ea97232bc5ead5f37252b7cbee47f48ef05932998050c414d14"
        );
        assert_eq!(
            activity.creator_address.as_deref().unwrap(),
            "0xf54f8f7ffc2b779d81b721b3d42fe9a53f96e1d3459a8001934307783d493725"
        );
        assert_eq!(activity.collection_name.as_deref().unwrap(), "The Loonies");
        assert_eq!(activity.token_name.as_deref().unwrap(), "The Loonies #399");
        assert_eq!(
            activity.token_data_id.as_deref().unwrap(),
            "0xc821b5c1712fca97553c85830b91dc212cd2fcdd2a2490b65f945ed901d9f126"
        );
        assert_eq!(
            activity.offer_id.as_deref().unwrap(),
            "0x9d14c489b6f56ac55e8707022400c23bb83bd0b0cd486c862defccf6241a219e"
        );
        assert_eq!(activity.marketplace, "test_marketplace");
        assert_eq!(activity.standard_event_type, "fill_token_offer");

        // Verify token offer details
        let token_offer = &token_offers[0];
        assert_eq!(token_offer.price, 3400000000);
        assert_eq!(
            token_offer.buyer,
            "0x22113f16f9b7c6761ef14df757c016b8736a9023e8881cd5e11579b0b98ef562"
        );
        assert_eq!(
            token_offer.token_data_id,
            "0xc821b5c1712fca97553c85830b91dc212cd2fcdd2a2490b65f945ed901d9f126"
        );
        assert_eq!(
            token_offer.offer_id.as_deref().unwrap(),
            "0x9d14c489b6f56ac55e8707022400c23bb83bd0b0cd486c862defccf6241a219e"
        );
        assert_eq!(token_offer.marketplace, "test_marketplace");
        assert!(token_offer.is_deleted);

        Ok(())
    }
}
