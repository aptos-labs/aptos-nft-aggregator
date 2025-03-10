use crate::{
    config::marketplace_config::{
        ContractToMarketplaceMap, MarketplaceEventConfig, MarketplaceEventConfigMappings,
    },
    schema::{
        current_nft_marketplace_collection_offers, current_nft_marketplace_listings,
        current_nft_marketplace_token_offers, nft_marketplace_activities,
    },
    steps::extract_string,
    utils::marketplace_resource_utils::{
        CollectionDataIdType, CollectionOfferEventMetadata, CollectionOfferMetadata,
        CollectionOfferV1, CollectionOfferV2, FixedPriceListing, ListingEventMetadata,
        ListingMetadata, ListingTokenV1Container, ObjectCore, ParticipantInfo, PricingInfo,
        TokenDataIdType, TokenMetadata, TokenMetadataInfo, TokenOfferMetadata, TokenStandard,
    },
};
use aptos_indexer_processor_sdk::utils::convert::standardize_address;
use aptos_protos::transaction::v1::Event;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, str::FromStr};
use tracing::{debug, warn};

/**
 * NftMarketplaceActivity is the main model for storing NFT marketplace activities.
*/
#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(txn_version, index))]
#[diesel(table_name = nft_marketplace_activities)]
pub struct NftMarketplaceActivity {
    pub txn_version: i64,
    pub index: i64,
    pub listing_id: Option<String>,
    pub offer_id: Option<String>,
    pub raw_event_type: String,
    pub standard_event_type: String,
    pub creator_address: Option<String>,
    pub collection_id: Option<String>,
    pub collection_name: Option<String>,
    pub token_data_id: Option<String>,
    pub token_name: Option<String>,
    pub token_standard: Option<String>,
    pub price: Option<i64>,
    pub token_amount: Option<i64>,
    pub buyer: Option<String>,
    pub seller: Option<String>,
    pub deadline: Option<String>,
    pub fee_schedule_id: Option<String>,
    pub coin_type: Option<String>,
    pub marketplace: String,
    pub contract_address: String,
    pub entry_function_id_str: Option<String>,
    pub json_data: serde_json::Value,
    pub block_timestamp: NaiveDateTime,
}

impl NftMarketplaceActivity {
    fn extract_metadata(
        config: &MarketplaceEventConfig,
        event_data: &Value,
        txn_version: i64,
    ) -> Option<TokenMetadataInfo> {
        let creator_address = extract_string(&config.creator_address, event_data);
        let collection_name = extract_string(&config.collection_name, event_data);
        let token_name = extract_string(&config.token_name, event_data);

        let token_data_id = Self::extract_token_data_id(
            config,
            event_data,
            &creator_address,
            &collection_name,
            &token_name,
        );

        let collection_id = Self::extract_collection_id(
            config,
            event_data,
            &creator_address,
            &collection_name,
            txn_version,
        );

        let token_standard = TokenStandard::from_str(
            &Self::determine_token_standard(event_data).unwrap_or_default(),
        )
        .unwrap();

        Some(TokenMetadataInfo {
            token_data_id,
            collection_id,
            creator_address,
            collection_name,
            token_name,
            token_standard,
        })
    }

    fn extract_pricing_info(config: &MarketplaceEventConfig, event_data: &Value) -> PricingInfo {
        PricingInfo {
            price: extract_string(&config.price, event_data).and_then(|s| s.parse::<i64>().ok()),
            token_amount: extract_string(&config.token_amount, event_data)
                .and_then(|s| s.parse::<i64>().ok()),
            deadline: extract_string(&config.deadline, event_data),
        }
    }

    fn extract_participant_info(
        config: &MarketplaceEventConfig,
        event_data: &Value,
    ) -> ParticipantInfo {
        ParticipantInfo {
            buyer: extract_string(&config.buyer, event_data).map(|s| standardize_address(&s)),
            seller: extract_string(&config.seller, event_data).map(|s| standardize_address(&s)),
        }
    }

    pub fn from_event_config(
        config: &MarketplaceEventConfig,
        event_data: &Value,
        event_type: String,
        contract_address: String,
        txn_version: i64,
        event_index: i64,
        block_timestamp: NaiveDateTime,
        entry_function_id_str: Option<String>,
        coin_type: Option<String>,
        token_metadatas: &mut HashMap<String, TokenMetadata>,
        fee_schedule_id: Option<String>,
    ) -> Option<Self> {
        let metadata_info = Self::extract_metadata(config, event_data, txn_version)?;
        let pricing_info = Self::extract_pricing_info(config, event_data);
        let participant_info = Self::extract_participant_info(config, event_data);

        let mut offer_id = extract_string(&config.offer_id, event_data);
        let mut listing_id = extract_string(&config.listing_id, event_data);

        if listing_id.is_none() && (config.event_type.as_str() == "place_listing") {
            if let Some(token_data_id) = &metadata_info.token_data_id {
                listing_id = Some(Self::generate_listing_or_offer_id(
                    &config.marketplace,
                    token_data_id,
                    txn_version,
                    event_index,
                    true,
                ));
            }
        }

        // handle offer_id for place offerId and no offer_id found
        if offer_id.is_none()
            && (config.event_type.as_str() == "place_offer"
                || config.event_type.as_str() == "place_collection_offer")
        {
            if let Some(token_data_id) = &metadata_info.token_data_id {
                offer_id = Some(Self::generate_listing_or_offer_id(
                    &config.marketplace,
                    token_data_id,
                    txn_version,
                    event_index,
                    false,
                ));
            }
        }

        Self::update_token_metadata_cache(token_metadatas, &metadata_info);

        Some(Self {
            txn_version,
            index: event_index,
            listing_id,
            offer_id,
            raw_event_type: event_type,
            standard_event_type: config.event_type.as_str().to_string(),
            creator_address: metadata_info.creator_address,
            collection_id: metadata_info.collection_id,
            collection_name: metadata_info.collection_name,
            token_data_id: metadata_info.token_data_id,
            token_name: metadata_info.token_name,
            token_standard: Some(metadata_info.token_standard.to_string()),
            price: pricing_info.price,
            token_amount: pricing_info.token_amount,
            buyer: participant_info.buyer,
            seller: participant_info.seller,
            deadline: pricing_info.deadline,
            fee_schedule_id,
            coin_type,
            marketplace: config.marketplace.clone(),
            contract_address,
            entry_function_id_str,
            json_data: event_data.clone(),
            block_timestamp,
        })
    }

    fn generate_listing_or_offer_id(
        marketplace: &str,
        token_data_id: &str,
        txn_version: i64,
        event_index: i64,
        is_listing: bool,
    ) -> String {
        if is_listing {
            format!(
                "{}:listing:{}:{}:{}",
                marketplace, token_data_id, txn_version, event_index
            )
        } else {
            format!(
                "{}:offer:{}:{}:{}",
                marketplace, token_data_id, txn_version, event_index
            )
        }
    }

    fn update_token_metadata_cache(
        token_metadatas: &mut HashMap<String, TokenMetadata>,
        metadata_info: &TokenMetadataInfo,
    ) {
        if let Some(token_data_id) = &metadata_info.token_data_id {
            token_metadatas.insert(token_data_id.clone(), TokenMetadata {
                collection_id: metadata_info.collection_id.clone().unwrap_or_default(),
                token_data_id: token_data_id.clone(),
                creator_address: metadata_info.creator_address.clone().unwrap_or_default(),
                collection_name: metadata_info.collection_name.clone().unwrap_or_default(),
                token_name: metadata_info.token_name.clone().unwrap_or_default(),
                token_standard: metadata_info.token_standard.clone(),
            });
        }
    }

    pub fn from_event(
        event: &Event,
        txn_version: i64,
        event_index: i64,
        block_timestamp: NaiveDateTime,
        entry_function_id_str: &Option<String>,
        event_mappings: &MarketplaceEventConfigMappings,
        contract_to_marketplace_map: &ContractToMarketplaceMap,
        token_metadatas: &mut HashMap<String, TokenMetadata>,
        coin_type: Option<String>,
    ) -> Option<Self> {
        let contract_address = event.type_str.clone();
        let marketplace_name =
            contract_to_marketplace_map
                .get(&contract_address)
                .or_else(|| {
                    debug!(
                        "Marketplace not found for the given contract address: {}",
                        contract_address
                    );
                    None
                })?;

        if let Some(event_mapping) = event_mappings.get(marketplace_name) {
            let event_type = event.type_str.to_string();
            let event_data: Value = serde_json::from_str(event.data.as_str()).unwrap();
            let fee_schedule_id = event.key.as_ref().map(|key| key.account_address.clone());

            if let Some(config) = event_mapping.get(&event_type) {
                return Self::from_event_config(
                    config,
                    &event_data,
                    event_type,
                    contract_address,
                    txn_version,
                    event_index,
                    block_timestamp,
                    entry_function_id_str.clone(),
                    coin_type,
                    token_metadatas,
                    fee_schedule_id,
                );
            }
        }
        None
    }

    fn extract_token_data_id(
        config: &MarketplaceEventConfig,
        event_data: &Value,
        creator_address: &Option<String>,
        collection_name: &Option<String>,
        token_name: &Option<String>,
    ) -> Option<String> {
        // check if it's a v2 token
        if let Some(inner_str) = extract_string(&config.token_inner, event_data) {
            return Some(standardize_address(&inner_str));
        }

        if creator_address.is_none() && collection_name.is_none() && token_name.is_none() {
            debug!("Missing fields for token data ID extraction");
            return None;
        }

        let token_data_id_type = TokenDataIdType::new(
            creator_address.clone(),
            collection_name.clone(),
            token_name.clone(),
        );

        // We use token_data_id as the main identifier for v1 tokens instead of data_id, which is similar but includes the property version.
        // The property version is irrelevant for NFTs as it indicates whether the token is fungible or not.
        Some(token_data_id_type.to_hash())
    }

    fn extract_inner_collection(
        config: &MarketplaceEventConfig,
        event_data: &Value,
    ) -> Option<String> {
        extract_string(&config.collection_inner, event_data)
    }

    // TODO: check what to do with collection_id for tradeport, which has token info in the resource
    fn extract_collection_id(
        config: &MarketplaceEventConfig,
        event_data: &Value,
        creator_address: &Option<String>,
        collection_name: &Option<String>,
        txn_version: i64,
    ) -> Option<String> {
        // check if it's a v2 collection
        if let Some(inner_str) = Self::extract_inner_collection(config, event_data) {
            return Some(standardize_address(&inner_str));
        }

        // if any of fields are None, we should return None and probably lean into resource handling
        if creator_address.is_none() || collection_name.is_none() {
            // check if resource address exists for token inner (e.g. 2386809975)
            // if let Some(token_inner) = extract_string(&config.token_inner, event_data) {
            // we will handle this in resource
            warn!(
                "Missing fields for collection ID extraction {:?}",
                txn_version
            );
            return None;
        }

        let collection_data_id_type =
            CollectionDataIdType::new(creator_address.clone(), collection_name.clone());

        Some(collection_data_id_type.to_hash())
    }

    /// Determines the token standard based on the event data.
    fn determine_token_standard(event_data: &Value) -> Option<String> {
        Some(
            if event_data.get("token_metadata").is_some()
                || event_data.get("collection_metadata").is_some()
                || event_data.get("collection").is_some()
                || event_data.get("token").is_some()
            {
                "v2"
            } else {
                "v1"
            }
            .to_string(),
        )
    }

    // fn handle_filled_events(
    //     &self,
    //     event: &Event,
    //     listing_filled_metadatas: &mut HashMap<String, ListingEventMetadata>,
    //     token_offer_filled_metadatas: &mut HashMap<String, TokenOfferEventMetadata>,
    //     collection_offer_filled_metadatas: &mut HashMap<String, CollectionOfferEventMetadata>,
    // ) {
    //     match MarketplaceEventType::from_str(self.standard_event_type.as_str()).unwrap() {
    //         MarketplaceEventType::FillListing => {
    //             if let Some(metadata) = self.create_listing_metadata(event) {
    //                 listing_filled_metadatas.insert(self.token_data_id.clone().unwrap_or_default(), metadata);
    //             }
    //         }
    //         MarketplaceEventType::FillOffer => {
    //             if let Some(metadata) = self.create_token_offer_metadata(event) {
    //                 token_offer_filled_metadatas.insert(self.token_data_id.clone().unwrap_or_default(), metadata);
    //             }
    //         }
    //         MarketplaceEventType::FillCollectionOffer => {
    //             if let Some((collection_offer_id, metadata)) = self.create_collection_offer_metadata(event) {
    //                 collection_offer_filled_metadatas.insert(collection_offer_id, metadata);
    //             }
    //         }
    //         _ => {}
    //     }
    // }

    // fn create_collection_metadata(&self) -> CollectionMetadata {
    //     CollectionMetadata {
    //         collection_id: self.collection_id.clone().unwrap_or_default(),
    //         creator_address: self.creator_address.clone().unwrap_or_default(),
    //         collection_name: self.collection_name.clone().unwrap_or_default(),
    //         token_standard: TokenStandard::from_str(
    //             self.token_standard.clone().unwrap_or_default().as_str()
    //         ).unwrap(),
    //     }
    // }

    // fn create_listing_metadata(&self, event: &Event) -> Option<ListingEventMetadata> {
    //     let collection_metadata = self.create_collection_metadata();
    //     let fee_schedule_id = self.extract_fee_schedule_id(event);

    //     Some(ListingEventMetadata {
    //         token_data_id: self.token_data_id.clone().unwrap_or_default(),
    //         collection_metadata,
    //         price: self.price.unwrap_or_default(),
    //         seller: self.seller.clone().unwrap_or_default(),
    //         fee_schedule_id,
    //         marketplace_name: self.marketplace.clone(),
    //         marketplace_contract_address: self.contract_address.clone(),
    //     })
    // }

    // fn create_token_offer_metadata(&self, event: &Event) -> Option<TokenOfferEventMetadata> {
    //     let collection_metadata = self.create_collection_metadata();
    //     let fee_schedule_id = self.extract_fee_schedule_id(event);

    //     Some(TokenOfferEventMetadata {
    //         token_data_id: self.token_data_id.clone().unwrap_or_default(),
    //         collection_metadata,
    //         price: self.price.unwrap_or_default(),
    //         buyer: self.buyer.clone().unwrap_or_default(),
    //         fee_schedule_id,
    //         marketplace_name: self.marketplace.clone(),
    //         marketplace_contract_address: self.contract_address.clone(),
    //     })
    // }

    // fn create_collection_offer_metadata(&self, event: &Event) -> Option<(String, CollectionOfferEventMetadata)> {
    //     let collection_metadata = self.create_collection_metadata();
    //     let fee_schedule_id = self.extract_fee_schedule_id(event);

    //     // Extract collection_offer_id from json_data
    //     let collection_offer_id = self.json_data
    //         .get("collection_offer")
    //         .and_then(|v| v.as_str())
    //         .unwrap_or_default()
    //         .to_string();

    //     let metadata = CollectionOfferEventMetadata {
    //         collection_offer_id: self.collection_id.clone().unwrap_or_default(),
    //         collection_metadata,
    //         price: self.price.unwrap_or_default(),
    //         buyer: self.buyer.clone().unwrap_or_default(),
    //         fee_schedule_id,
    //         marketplace_name: self.marketplace.clone(),
    //         marketplace_contract_address: self.contract_address.clone(),
    //     };

    //     Some((collection_offer_id, metadata))
    // }

    // fn extract_fee_schedule_id(&self, event: &Event) -> String {
    //     event.key
    //         .as_ref()
    //         .map(|key| key.account_address.clone())
    //         .unwrap_or_default()
    // }
}

#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(listing_id))]
#[diesel(table_name = current_nft_marketplace_listings)]
pub struct CurrentNFTMarketplaceListing {
    pub listing_id: String,
    pub marketplace: String,
    pub token_data_id: String,
    pub collection_id: String, // optional?
    pub fee_schedule_id: String,
    pub price: Option<i64>,
    pub token_amount: Option<i64>,
    pub token_standard: String,
    pub seller: String,
    pub is_deleted: bool,
    pub coin_type: Option<String>,
    pub contract_address: String,
    pub entry_function_id_str: String,
    pub last_transaction_version: i64,
    pub last_transaction_timestamp: NaiveDateTime,
}

impl CurrentNFTMarketplaceListing {
    pub fn new_v1_listing(
        token_v1_metadata: &TokenMetadata,
        listing_metadata: &ListingMetadata,
        fixed_price_listing: &FixedPriceListing,
        token_v1_container: &ListingTokenV1Container,
        marketplace_name: &str,
        contract_address: &str,
        txn_version: i64,
        txn_timestamp: chrono::NaiveDateTime,
        entry_function_id_str: &str,
        coin_type: Option<String>,
    ) -> Self {
        Self {
            listing_id: format!(
                "{}_{}:{}",
                marketplace_name, "listing", token_v1_metadata.token_data_id
            ),
            token_data_id: token_v1_metadata.token_data_id.clone(),
            collection_id: token_v1_metadata.collection_id.clone(),
            fee_schedule_id: listing_metadata.fee_schedule_id.clone(),
            price: Some(fixed_price_listing.price),
            token_amount: Some(token_v1_container.amount),
            token_standard: TokenStandard::V1.to_string(),
            seller: listing_metadata.seller.clone(),
            is_deleted: false,
            coin_type,
            marketplace: marketplace_name.to_string(),
            contract_address: contract_address.to_string(),
            entry_function_id_str: entry_function_id_str.to_string(),
            last_transaction_version: txn_version,
            last_transaction_timestamp: txn_timestamp,
        }
    }

    pub fn new_v2_listing(
        token_v2_metadata: &TokenMetadata,
        listing_metadata: &ListingMetadata,
        fixed_price_listing: &FixedPriceListing,
        marketplace_name: &str,
        contract_address: &str,
        txn_version: i64,
        txn_timestamp: chrono::NaiveDateTime,
        entry_function_id_str: &str,
        coin_type: Option<String>,
    ) -> Self {
        Self {
            listing_id: format!(
                "{}_{}:{}",
                marketplace_name, "listing", token_v2_metadata.token_data_id
            ),
            token_data_id: token_v2_metadata.token_data_id.clone(),
            collection_id: token_v2_metadata.collection_id.clone(),
            fee_schedule_id: listing_metadata.fee_schedule_id.clone(),
            price: Some(fixed_price_listing.price),
            token_amount: Some(1),
            token_standard: TokenStandard::V2.to_string(),
            seller: listing_metadata.seller.clone(),
            is_deleted: false,
            coin_type,
            marketplace: marketplace_name.to_string(),
            contract_address: contract_address.to_string(),
            entry_function_id_str: entry_function_id_str.to_string(),
            last_transaction_version: txn_version,
            last_transaction_timestamp: txn_timestamp,
        }
    }

    // TODO: This is a temporary function to build a listing filled metadata.
    pub fn build_listing_filled(
        listing_filled_metadata: &ListingEventMetadata,
        txn_version: i64,
        txn_timestamp: chrono::NaiveDateTime,
        entry_function_id_str: &str,
        coin_type: Option<String>,
        marketplace_name: &str,
    ) -> Self {
        Self {
            listing_id: format!(
                "{}_{}:{}",
                marketplace_name, "listing", listing_filled_metadata.listing_id
            ),
            token_data_id: listing_filled_metadata.listing_id.clone(),
            collection_id: "temp_collection_id".to_string(), // Temp value
            fee_schedule_id: listing_filled_metadata.fee_schedule_id.clone(),
            price: Some(listing_filled_metadata.price),
            token_amount: Some(1),            // Temp value
            token_standard: "V1".to_string(), // Temp value
            seller: listing_filled_metadata.listing_metadata.seller.clone(),
            is_deleted: true, // Set to true since this is a filled/deleted listing
            coin_type,
            marketplace: marketplace_name.to_string(),
            contract_address: "temp_contract_address".to_string(), // Temp value
            entry_function_id_str: entry_function_id_str.to_string(),
            last_transaction_version: txn_version,
            last_transaction_timestamp: txn_timestamp,
        }
    }

    pub fn build_listing_plaes_events(activity: &NftMarketplaceActivity) -> Self {
        Self {
            marketplace: activity.marketplace.clone(),
            listing_id: activity.listing_id.clone().unwrap_or_default(),
            token_data_id: activity.token_data_id.clone().unwrap_or_default(),
            collection_id: activity.collection_id.clone().unwrap_or_default(),
            fee_schedule_id: activity.fee_schedule_id.clone().unwrap_or_default(),
            price: activity.price,
            token_amount: activity.token_amount,
            token_standard: activity.token_standard.clone().unwrap_or_default(),
            seller: activity.seller.clone().unwrap_or_default(),
            is_deleted: false,
            coin_type: activity.coin_type.clone(),
            contract_address: activity.contract_address.clone(),
            entry_function_id_str: activity.entry_function_id_str.clone().unwrap_or_default(),
            last_transaction_version: activity.txn_version,
            last_transaction_timestamp: activity.block_timestamp,
        }
    }

    pub fn build_cancel_or_fill_listing(
        activity: &NftMarketplaceActivity,
        batch_listing_map: &mut HashMap<String, String>,
    ) -> Option<Self> {
        let token_data_id = activity.token_data_id.clone().unwrap_or_else(|| {
            panic!(
                "Token Data Id not found for version {}",
                activity.txn_version
            )
        });

        // when any of fields required are not present here, we just don't create here. and delegate to resource remapper.
        // if collection_id is None  -> check if we have token_inner, populate the map with this inner. for now we just skip.
        // check if ti's a filled event -
        if activity.collection_id.is_none() || activity.token_data_id.is_none() {
            println!("missing collection_id or token_data_id for txn_version = {:?}, collection_id = {:?}, token_data_id = {:?}", 
                activity.txn_version, activity.collection_id, activity.token_data_id
            );
            return None;
        }

        // if found in the activity or foundi nthe map
        if activity.listing_id.is_some() || batch_listing_map.contains_key(&token_data_id) {
            return Some(Self {
                marketplace: activity.marketplace.clone(),
                listing_id: activity
                    .listing_id
                    .clone()
                    .unwrap_or_else(|| batch_listing_map.get(&token_data_id).cloned().unwrap()),
                token_data_id: activity.token_data_id.clone().unwrap(),
                collection_id: activity.collection_id.clone().unwrap(),
                fee_schedule_id: activity.fee_schedule_id.clone().unwrap(),
                price: activity.price,
                token_amount: activity.token_amount,
                token_standard: activity.token_standard.clone().unwrap(),
                seller: activity.seller.clone().unwrap(),
                is_deleted: true,
                coin_type: activity.coin_type.clone(),
                contract_address: activity.contract_address.clone(),
                entry_function_id_str: activity.entry_function_id_str.clone().unwrap(),
                last_transaction_version: activity.txn_version,
                last_transaction_timestamp: activity.block_timestamp,
            });
        }
        None
    }

    // pub fn mark_listing_as_deleted(
    //     conn: &mut PgConnection,
    //     token_data_id_event: &str,
    //     event_txn_version: i64,
    //     event_timestamp: NaiveDateTime,
    // ) -> diesel::result::QueryResult<usize> {
    //     use crate::schema::current_nft_marketplace_listings::dsl::*;

    //     diesel::update(
    //         current_nft_marketplace_listings.filter(
    //             token_data_id
    //                 .eq(token_data_id_event)
    //                 .and(is_deleted.eq(false)),
    //         ),
    //     )
    //     .set((
    //         is_deleted.eq(true),
    //         last_transaction_version.eq(event_txn_version),
    //         last_transaction_timestamp.eq(event_timestamp),
    //     ))
    //     .execute(conn)
    // }
}

#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(offer_id))]
#[diesel(table_name = current_nft_marketplace_token_offers)]
pub struct CurrentNFTMarketplaceTokenOffer {
    pub offer_id: String,
    pub marketplace: String,
    pub token_data_id: String,
    pub collection_id: String,
    pub fee_schedule_id: String,
    pub buyer: String,
    pub price: Option<i64>,
    pub token_amount: Option<i64>,
    pub token_name: Option<String>,
    pub is_deleted: bool,
    pub token_standard: String,
    pub coin_type: Option<String>,
    pub contract_address: String,
    pub entry_function_id_str: String,
    pub last_transaction_version: i64,
    pub last_transaction_timestamp: NaiveDateTime,
}

impl CurrentNFTMarketplaceTokenOffer {
    pub fn new_v1_token_offer(
        token_metadata: &TokenMetadata,
        token_offer_metadata: &TokenOfferMetadata,
        token_offer_object: &ObjectCore,
        marketplace_name: &str,
        contract_address: &str,
        txn_version: i64,
        txn_timestamp: chrono::NaiveDateTime,
        entry_function_id_str: &str,
        coin_type: Option<String>,
    ) -> Self {
        Self {
            offer_id: format!(
                "{}_{}:{}",
                marketplace_name, "offer", token_metadata.token_data_id
            ), // TODO: use trait
            marketplace: marketplace_name.to_string(),
            token_data_id: token_metadata.token_data_id.clone(),
            collection_id: token_metadata.collection_id.clone(),
            fee_schedule_id: token_offer_metadata.fee_schedule_id.clone(),
            buyer: token_offer_object.owner.clone(),
            price: Some(token_offer_metadata.price),
            token_amount: Some(1),
            token_name: Some(token_metadata.token_name.clone()),
            is_deleted: false,
            token_standard: TokenStandard::V1.to_string(),
            coin_type,
            contract_address: contract_address.to_string(),
            entry_function_id_str: entry_function_id_str.to_string(),
            last_transaction_version: txn_version,
            last_transaction_timestamp: txn_timestamp,
        }
    }

    pub fn new_v2_token_offer(
        token_metadata: &TokenMetadata,
        token_offer_metadata: &TokenOfferMetadata,
        token_offer_object: &ObjectCore,
        marketplace_name: &str,
        contract_address: &str,
        txn_version: i64,
        txn_timestamp: chrono::NaiveDateTime,
        entry_function_id_str: &str,
        coin_type: Option<String>,
    ) -> Self {
        Self {
            offer_id: format!(
                "{}_{}:{}",
                marketplace_name, "offer", token_metadata.token_data_id
            ),
            marketplace: marketplace_name.to_string(),
            token_data_id: token_metadata.token_data_id.clone(),
            collection_id: token_metadata.collection_id.clone(),
            fee_schedule_id: token_offer_metadata.fee_schedule_id.clone(),
            buyer: token_offer_object.owner.clone(),
            price: Some(token_offer_metadata.price),
            token_amount: Some(1),
            token_name: Some(token_metadata.token_name.clone()),
            is_deleted: false,
            token_standard: TokenStandard::V2.to_string(),
            coin_type,
            contract_address: contract_address.to_string(),
            entry_function_id_str: entry_function_id_str.to_string(),
            last_transaction_version: txn_version,
            last_transaction_timestamp: txn_timestamp,
        }
    }

    // Some of fields are from TokenOfferMetadata which means I can reuse the ones from events.
    // pub fn build_token_offer_filled(
    //     token_metadata: &TokenMetadata,
    //     txn_version: i64,
    //     txn_timestamp: chrono::NaiveDateTime,
    //     entry_function_id_str: &str,
    //     coin_type: Option<String>,
    //     fee_schedule_id: String,
    //     marketplace_name: &str,
    // ) -> Self {
    //     Self {
    //         token_data_id: token_metadata.token_data_id.clone(),
    //         collection_id: token_metadata.collection_id.clone(),
    //         fee_schedule_id: fee_schedule_id,
    //         buyer: token_offer_metadata.buyer.clone(),
    //         price: Some(token_offer_metadata.price),
    //         token_amount: Some(1),
    //         expiration_time: Some(token_offer_metadata.expiration_time),
    //         is_deleted: true,
    //         token_standard: token_metadata.token_standard.to_string(),
    //         coin_type,
    //         marketplace: marketplace_name.to_string(),
    //         contract_address: contract_address.to_string(),
    //         entry_function_id_str: entry_function_id_str.to_string(),
    //         last_transaction_version: txn_version,
    //         last_transaction_timestamp: txn_timestamp,
    //     }
    // }

    pub fn from_activity(activity: &NftMarketplaceActivity, is_deleted: bool) -> Self {
        Self {
            offer_id: activity.offer_id.clone().unwrap_or_default(),
            marketplace: activity.marketplace.clone(),
            token_data_id: activity.token_data_id.clone().unwrap_or_default(),
            collection_id: activity.collection_id.clone().unwrap_or_default(),
            fee_schedule_id: activity.fee_schedule_id.clone().unwrap_or_default(),
            buyer: activity.buyer.clone().unwrap_or_default(),
            price: activity.price,
            token_amount: activity.token_amount,
            token_name: activity.token_name.clone(),
            token_standard: activity.token_standard.clone().unwrap_or_default(),
            coin_type: activity.coin_type.clone(),
            contract_address: activity.contract_address.clone(),
            entry_function_id_str: activity.entry_function_id_str.clone().unwrap_or_default(),
            is_deleted,
            last_transaction_version: activity.txn_version,
            last_transaction_timestamp: activity.block_timestamp,
        }
    }

    pub fn build_cancel_or_fill_offer(
        activity: &NftMarketplaceActivity,
        batch_offer_map: &mut HashMap<String, String>,
    ) -> Option<Self> {
        // let token_data_id = activity.token_data_id.clone().expect(&format!("Token Data Id not found for version {}", activity.txn_version));
        // when any of fields required are not present here, we just don't create here. and delegate to resource remapper.
        // if collection_id is None  -> check if we have token_inner, populate the map with this inner. for now we just skip.
        // check if ti's a filled event -
        if activity.collection_id.is_none() || activity.token_data_id.is_none() {
            println!("missing collection_id or token_data_id for txn_version = {:?}, collection_id = {:?}, token_data_id = {:?}", 
                activity.txn_version, activity.collection_id, activity.token_data_id
            );
            return None;
        }

        // if found in the activity or found in the map
        // TODO: validate if this is correct, when same token_data_id exists
        if activity.offer_id.is_some()
            || (activity.token_data_id.is_some()
                && batch_offer_map.contains_key(&activity.token_data_id.clone().unwrap()))
        {
            let token_data_id = activity.token_data_id.clone().unwrap();

            return Some(Self {
                offer_id: activity
                    .offer_id
                    .clone()
                    .unwrap_or_else(|| batch_offer_map.get(&token_data_id).cloned().unwrap()),
                marketplace: activity.marketplace.clone(),
                token_data_id,
                collection_id: activity.collection_id.clone().unwrap(),
                fee_schedule_id: activity.fee_schedule_id.clone().unwrap_or_default(),
                buyer: activity.buyer.clone().unwrap_or_default(),
                price: activity.price,
                token_amount: activity.token_amount,
                token_name: activity.token_name.clone(),
                token_standard: activity.token_standard.clone().unwrap_or_default(),
                coin_type: activity.coin_type.clone(),
                contract_address: activity.contract_address.clone(),
                entry_function_id_str: activity.entry_function_id_str.clone().unwrap_or_default(),
                is_deleted: true,
                last_transaction_version: activity.txn_version,
                last_transaction_timestamp: activity.block_timestamp,
            });
        }
        None
    }
}

#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(collection_offer_id))]
#[diesel(table_name = current_nft_marketplace_collection_offers)]
pub struct CurrentNFTMarketplaceCollectionOffer {
    pub collection_offer_id: String,
    pub collection_id: String,
    pub fee_schedule_id: String,
    pub buyer: String,
    pub price: Option<i64>,
    pub remaining_token_amount: Option<i64>,
    pub is_deleted: bool,
    pub token_standard: String,
    pub coin_type: Option<String>,
    pub marketplace: String,
    pub contract_address: String,
    pub entry_function_id_str: String,
    pub last_transaction_version: i64,
    pub last_transaction_timestamp: NaiveDateTime,
}

impl CurrentNFTMarketplaceCollectionOffer {
    pub fn new_v1_collection_offer(
        move_resource_address: String,
        collection_offer_v1: &CollectionOfferV1,
        collection_offer_metadata: &CollectionOfferMetadata,
        collection_offer_object: &ObjectCore,
        marketplace_name: &str,
        contract_address: &str,
        txn_version: i64,
        txn_timestamp: chrono::NaiveDateTime,
        entry_function_id_str: &str,
        coin_type: Option<String>,
    ) -> Self {
        Self {
            collection_offer_id: move_resource_address,
            collection_id: collection_offer_v1
                .collection_metadata
                .collection_id
                .clone(),
            fee_schedule_id: collection_offer_metadata.fee_schedule_id.clone(),
            buyer: collection_offer_object.owner.clone(),
            price: Some(collection_offer_metadata.price),
            remaining_token_amount: Some(collection_offer_metadata.remaining_token_amount),
            is_deleted: false,
            token_standard: TokenStandard::V1.to_string(),
            coin_type,
            marketplace: marketplace_name.to_string(),
            contract_address: contract_address.to_string(),
            entry_function_id_str: entry_function_id_str.to_string(),
            last_transaction_version: txn_version,
            last_transaction_timestamp: txn_timestamp,
        }
    }

    pub fn new_v2_collection_offer(
        move_resource_address: String,
        collection_offer_v2: &CollectionOfferV2,
        collection_offer_metadata: &CollectionOfferMetadata,
        collection_offer_object: &ObjectCore,
        marketplace_name: &str,
        contract_address: &str,
        txn_version: i64,
        txn_timestamp: chrono::NaiveDateTime,
        entry_function_id_str: &str,
        coin_type: Option<String>,
    ) -> Self {
        Self {
            collection_offer_id: move_resource_address,
            collection_id: collection_offer_v2.collection_address.clone(),
            fee_schedule_id: collection_offer_metadata.fee_schedule_id.clone(),
            buyer: collection_offer_object.owner.clone(),
            price: Some(collection_offer_metadata.price),
            remaining_token_amount: Some(collection_offer_metadata.remaining_token_amount),
            is_deleted: false,
            token_standard: TokenStandard::V2.to_string(),
            coin_type,
            marketplace: marketplace_name.to_string(),
            contract_address: contract_address.to_string(),
            entry_function_id_str: entry_function_id_str.to_string(),
            last_transaction_version: txn_version,
            last_transaction_timestamp: txn_timestamp,
        }
    }

    pub fn build_collection_offer_filled(
        collection_offer_filled_metadata: &CollectionOfferEventMetadata,
        txn_version: i64,
        txn_timestamp: chrono::NaiveDateTime,
        entry_function_id_str: &str,
        coin_type: Option<String>,
    ) -> Self {
        let collection_metadata = collection_offer_filled_metadata.collection_metadata.clone();
        Self {
            collection_offer_id: collection_offer_filled_metadata.collection_offer_id.clone(),
            collection_id: collection_metadata.collection_id.clone(),
            fee_schedule_id: collection_offer_filled_metadata.fee_schedule_id.clone(),
            buyer: collection_offer_filled_metadata.buyer.clone(),
            price: Some(collection_offer_filled_metadata.price),
            remaining_token_amount: Some(0),
            is_deleted: true,
            token_standard: collection_metadata.token_standard.to_string(),
            coin_type,
            marketplace: collection_offer_filled_metadata
                .marketplace_name
                .to_string(),
            contract_address: collection_offer_filled_metadata
                .marketplace_contract_address
                .to_string(),
            entry_function_id_str: entry_function_id_str.to_string(),
            last_transaction_version: txn_version,
            last_transaction_timestamp: txn_timestamp,
        }
    }

    pub fn from_activity(activity: &NftMarketplaceActivity, is_deleted: bool) -> Self {
        Self {
            collection_offer_id: activity.offer_id.clone().unwrap_or_default(),
            collection_id: activity.collection_id.clone().unwrap_or_default(),
            fee_schedule_id: activity.fee_schedule_id.clone().unwrap_or_default(),
            buyer: activity.buyer.clone().unwrap_or_default(),
            price: activity.price,
            remaining_token_amount: activity.token_amount,
            token_standard: activity.token_standard.clone().unwrap_or_default(),
            marketplace: activity.marketplace.clone(),
            contract_address: activity.contract_address.clone(),
            entry_function_id_str: activity.entry_function_id_str.clone().unwrap_or_default(),
            coin_type: activity.coin_type.clone(),
            is_deleted,
            last_transaction_version: activity.txn_version,
            last_transaction_timestamp: activity.block_timestamp,
        }
    }

    pub fn build_cancel_or_fill_offer(
        activity: &NftMarketplaceActivity,
        batch_collection_offer_map: &mut HashMap<String, String>,
    ) -> Option<Self> {
        // let token_data_id = activity.token_data_id.clone().expect(&format!("Token Data Id not found for version {}", activity.txn_version));
        // when any of fields required are not present here, we just don't create here. and delegate to resource remapper.
        // if collection_id is None  -> check if we have token_inner, populate the map with this inner. for now we just skip.
        // check if ti's a filled event -
        if activity.collection_id.is_none() || activity.token_data_id.is_none() {
            println!("missing collection_id or token_data_id for txn_version = {:?}, collection_id = {:?}, token_data_id = {:?}", 
                activity.txn_version, activity.collection_id, activity.token_data_id
            );
            return None;
        }

        // if found in the activity or foundi nthe map
        if activity.offer_id.is_some()
            || (activity.token_data_id.is_some()
                && batch_collection_offer_map
                    .contains_key(&activity.token_data_id.clone().unwrap()))
        {
            let token_data_id = activity.token_data_id.clone().unwrap();
            return Some(Self {
                collection_offer_id: activity.offer_id.clone().unwrap_or_else(|| {
                    batch_collection_offer_map
                        .get(&token_data_id)
                        .cloned()
                        .unwrap()
                }),
                collection_id: activity.collection_id.clone().unwrap(),
                fee_schedule_id: activity.fee_schedule_id.clone().unwrap_or_default(),
                buyer: activity.buyer.clone().unwrap_or_default(),
                price: activity.price,
                remaining_token_amount: activity.token_amount,
                token_standard: activity.token_standard.clone().unwrap_or_default(),
                marketplace: activity.marketplace.clone(),
                contract_address: activity.contract_address.clone(),
                entry_function_id_str: activity.entry_function_id_str.clone().unwrap_or_default(),
                coin_type: activity.coin_type.clone(),
                is_deleted: true,
                last_transaction_version: activity.txn_version,
                last_transaction_timestamp: activity.block_timestamp,
            });
        }
        None
    }
}
