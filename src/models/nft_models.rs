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
use aptos_indexer_processor_sdk::utils::convert::{sha3_256, standardize_address};
use aptos_protos::transaction::v1::Event;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, str::FromStr};
use tracing::debug;
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
    pub token_data_id: String,
    pub token_name: Option<String>,
    pub token_standard: Option<String>,
    pub price: i64,
    pub token_amount: Option<i64>,
    pub buyer: Option<String>,
    pub seller: Option<String>,
    pub expiration_time: Option<String>,
    pub json_data: serde_json::Value,
    pub marketplace: String,
    pub contract_address: String,
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

        let token_data_id: Option<String> = Self::extract_token_data_id(
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
        token_metadatas: &mut HashMap<String, TokenMetadata>,
    ) -> Option<Self> {
        let metadata_info = Self::extract_metadata(config, event_data, txn_version)?;
        let pricing_info = Self::extract_pricing_info(config, event_data);
        let participant_info = Self::extract_participant_info(config, event_data);

        // Try to extract offer_id first, fallback to collection_offer_id if offer_id is None
        let offer_id = extract_string(&config.offer_id, event_data)
            .or_else(|| extract_string(&config.collection_offer_id, event_data));
        let listing_id = extract_string(&config.listing_id, event_data);

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
            token_data_id: metadata_info.token_data_id.unwrap_or_default(),
            token_name: metadata_info.token_name,
            token_standard: Some(metadata_info.token_standard.to_string()),
            price: pricing_info
                .price
                .unwrap_or_else(|| panic!("price is required for txn_version = {:?}", txn_version)),
            token_amount: pricing_info.token_amount,
            buyer: participant_info.buyer,
            seller: participant_info.seller,
            expiration_time: pricing_info.deadline,
            marketplace: config.marketplace.clone(),
            contract_address,
            json_data: event_data.clone(),
            block_timestamp,
        })
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
        event_mappings: &MarketplaceEventConfigMappings,
        contract_to_marketplace_map: &ContractToMarketplaceMap,
        token_metadatas: &mut HashMap<String, TokenMetadata>,
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

            if let Some(config) = event_mapping.get(&event_type) {
                return Self::from_event_config(
                    config,
                    &event_data,
                    event_type,
                    contract_address,
                    txn_version,
                    event_index,
                    block_timestamp,
                    token_metadatas,
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

        if creator_address.is_none() || collection_name.is_none() || token_name.is_none() {
            debug!("Missing fields for token data ID extraction");
            return None;
        }

        let token_data_id_type = TokenDataIdType::new(
            creator_address.clone().unwrap(),
            collection_name.clone().unwrap(),
            token_name.clone().unwrap(),
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
            debug!(
                "Missing fields for collection ID extraction {:?}",
                txn_version
            );
            return None;
        }

        let collection_data_id_type = CollectionDataIdType::new(
            creator_address.clone().unwrap(),
            collection_name.clone().unwrap(),
        );

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
}

#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(token_data_id))]
#[diesel(table_name = current_nft_marketplace_listings)]
pub struct CurrentNFTMarketplaceListing {
    pub token_data_id: String,
    pub listing_id: Option<String>,
    pub collection_id: Option<String>,
    pub seller: String,
    pub price: i64,
    pub token_amount: i64,
    pub token_standard: String,
    pub is_deleted: bool,
    pub marketplace: String,
    pub contract_address: String,
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
    ) -> Self {
        Self {
            token_data_id: token_v1_metadata.token_data_id.clone(),
            listing_id: None,
            collection_id: Some(token_v1_metadata.collection_id.clone()),
            seller: listing_metadata.seller.clone(),
            price: fixed_price_listing.price,
            token_amount: token_v1_container.amount,
            token_standard: TokenStandard::V1.to_string(),
            is_deleted: false,
            marketplace: marketplace_name.to_string(),
            contract_address: contract_address.to_string(),
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
    ) -> Self {
        Self {
            token_data_id: token_v2_metadata.token_data_id.clone(),
            listing_id: None,
            collection_id: Some(token_v2_metadata.collection_id.clone()),
            seller: listing_metadata.seller.clone(),
            price: fixed_price_listing.price,
            token_amount: 1,
            token_standard: TokenStandard::V2.to_string(),
            is_deleted: false,
            marketplace: marketplace_name.to_string(),
            contract_address: contract_address.to_string(),
            last_transaction_version: txn_version,
            last_transaction_timestamp: txn_timestamp,
        }
    }

    // TODO: This is a temporary function to build a listing filled metadata.
    pub fn build_listing_filled(
        listing_filled_metadata: &ListingEventMetadata,
        txn_version: i64,
        txn_timestamp: chrono::NaiveDateTime,
        marketplace_name: &str,
    ) -> Self {
        Self {
            token_data_id: listing_filled_metadata.listing_id.clone(),
            listing_id: None,
            collection_id: Some("temp_collection_id".to_string()), // Temp value
            seller: listing_filled_metadata.listing_metadata.seller.clone(),
            price: listing_filled_metadata.price,
            token_amount: 1,                  // Temp value
            token_standard: "V1".to_string(), // Temp value
            is_deleted: true,                 // Set to true since this is a filled/deleted listing
            marketplace: marketplace_name.to_string(),
            contract_address: "temp_contract_address".to_string(), // Temp value
            last_transaction_version: txn_version,
            last_transaction_timestamp: txn_timestamp,
        }
    }

    pub fn from_activity(activity: &NftMarketplaceActivity) -> Self {
        Self {
            token_data_id: activity.token_data_id.clone(),
            marketplace: activity.marketplace.clone(),
            listing_id: activity.listing_id.clone(),
            collection_id: activity.collection_id.clone(),
            seller: activity.seller.clone().unwrap_or_else(|| {
                panic!(
                    "seller is required for listing for txn_version = {:?}",
                    activity.txn_version
                )
            }),
            price: activity.price,
            token_amount: activity.token_amount.unwrap_or_else(|| {
                debug!(
                    "token_amount of listing event is missing for txn_version = {:?}",
                    activity.txn_version
                );
                0
            }),
            token_standard: activity.token_standard.clone().unwrap_or_default(),
            is_deleted: false,
            contract_address: activity.contract_address.clone(),
            last_transaction_version: activity.txn_version,
            last_transaction_timestamp: activity.block_timestamp,
        }
    }

    pub fn cancelled_or_filled_listing(activity: &NftMarketplaceActivity) -> Self {
        Self {
            token_data_id: activity.token_data_id.clone(),
            marketplace: activity.marketplace.clone(),
            listing_id: activity.listing_id.clone(),
            collection_id: activity.collection_id.clone(),
            seller: activity.seller.clone().unwrap_or_else(|| {
                panic!(
                    "seller is required for listing for txn_version = {:?}",
                    activity.txn_version
                )
            }),
            price: activity.price,
            token_amount: 0,
            token_standard: activity.token_standard.clone().unwrap_or_default(),
            is_deleted: true,
            contract_address: activity.contract_address.clone(),
            last_transaction_version: activity.txn_version,
            last_transaction_timestamp: activity.block_timestamp,
        }
    }
}

#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(token_data_id, buyer))]
#[diesel(table_name = current_nft_marketplace_token_offers)]
pub struct CurrentNFTMarketplaceTokenOffer {
    pub token_data_id: String,
    pub offer_id: Option<String>,
    pub buyer: String,
    pub collection_id: String,
    pub price: i64,
    pub token_amount: Option<i64>,
    pub token_name: Option<String>,
    pub is_deleted: bool,
    pub marketplace: String,
    pub token_standard: String,
    pub contract_address: String,
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
    ) -> Self {
        Self {
            token_data_id: token_metadata.token_data_id.clone(),
            offer_id: None,
            marketplace: marketplace_name.to_string(),
            buyer: token_offer_object.owner.clone(),
            collection_id: token_metadata.collection_id.clone(),
            price: token_offer_metadata.price,
            token_amount: Some(1),
            token_name: Some(token_metadata.token_name.clone()),
            is_deleted: false,
            token_standard: TokenStandard::V1.to_string(),
            contract_address: contract_address.to_string(),
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
    ) -> Self {
        Self {
            token_data_id: token_metadata.token_data_id.clone(),
            offer_id: None,
            buyer: token_offer_object.owner.clone(),
            collection_id: token_metadata.collection_id.clone(),
            price: token_offer_metadata.price,
            token_amount: Some(1),
            token_name: Some(token_metadata.token_name.clone()),
            is_deleted: false,
            token_standard: TokenStandard::V2.to_string(),
            marketplace: marketplace_name.to_string(),
            contract_address: contract_address.to_string(),
            last_transaction_version: txn_version,
            last_transaction_timestamp: txn_timestamp,
        }
    }

    pub fn from_activity(activity: &NftMarketplaceActivity, is_deleted: bool) -> Self {
        Self {
            token_data_id: activity.token_data_id.clone(),
            offer_id: activity.offer_id.clone(),
            buyer: activity.buyer.clone().unwrap_or_default(),
            collection_id: activity.collection_id.clone().unwrap_or_default(),
            price: activity.price,
            token_amount: activity.token_amount,
            token_name: activity.token_name.clone(),
            is_deleted,
            marketplace: activity.marketplace.clone(),
            token_standard: activity.token_standard.clone().unwrap_or_default(),
            contract_address: activity.contract_address.clone(),
            last_transaction_version: activity.txn_version,
            last_transaction_timestamp: activity.block_timestamp,
        }
    }

    pub fn build_cancelled_or_filled_token_offer_from_activity(
        activity: &NftMarketplaceActivity,
    ) -> Self {
        Self {
            token_data_id: activity.token_data_id.clone(),
            offer_id: activity.offer_id.clone(),
            buyer: activity.buyer.clone().unwrap_or_default(),
            collection_id: activity.collection_id.clone().unwrap_or_default(),
            price: activity.price,
            token_amount: activity.token_amount,
            token_name: activity.token_name.clone(),
            is_deleted: true,
            marketplace: activity.marketplace.clone(),
            token_standard: activity.token_standard.clone().unwrap_or_default(),
            contract_address: activity.contract_address.clone(),
            last_transaction_version: activity.txn_version,
            last_transaction_timestamp: activity.block_timestamp,
        }
    }
}

#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(collection_offer_id))]
#[diesel(table_name = current_nft_marketplace_collection_offers)]
pub struct CurrentNFTMarketplaceCollectionOffer {
    pub collection_offer_id: String,
    pub collection_id: String,
    pub buyer: String,
    pub price: i64,
    pub remaining_token_amount: Option<i64>,
    pub is_deleted: bool,
    pub token_standard: String,
    pub marketplace: String,
    pub contract_address: String,
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
    ) -> Self {
        Self {
            collection_offer_id: move_resource_address,
            collection_id: collection_offer_v1
                .collection_metadata
                .collection_id
                .clone(),
            buyer: collection_offer_object.owner.clone(),
            price: collection_offer_metadata.price,
            remaining_token_amount: Some(collection_offer_metadata.remaining_token_amount),
            is_deleted: false,
            token_standard: TokenStandard::V1.to_string(),
            marketplace: marketplace_name.to_string(),
            contract_address: contract_address.to_string(),
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
    ) -> Self {
        Self {
            collection_offer_id: move_resource_address,
            collection_id: collection_offer_v2.collection_address.clone(),
            buyer: collection_offer_object.owner.clone(),
            price: collection_offer_metadata.price,
            remaining_token_amount: Some(collection_offer_metadata.remaining_token_amount),
            is_deleted: false,
            token_standard: TokenStandard::V2.to_string(),
            marketplace: marketplace_name.to_string(),
            contract_address: contract_address.to_string(),
            last_transaction_version: txn_version,
            last_transaction_timestamp: txn_timestamp,
        }
    }

    pub fn build_collection_offer_filled(
        collection_offer_filled_metadata: &CollectionOfferEventMetadata,
        txn_version: i64,
        txn_timestamp: chrono::NaiveDateTime,
    ) -> Self {
        let collection_metadata = collection_offer_filled_metadata.collection_metadata.clone();
        Self {
            collection_offer_id: collection_offer_filled_metadata.collection_offer_id.clone(),
            collection_id: collection_metadata.collection_id.clone(),
            buyer: collection_offer_filled_metadata.buyer.clone(),
            price: collection_offer_filled_metadata.price,
            remaining_token_amount: Some(0),
            is_deleted: true,
            token_standard: collection_metadata.token_standard.to_string(),
            marketplace: collection_offer_filled_metadata
                .marketplace_name
                .to_string(),
            contract_address: collection_offer_filled_metadata
                .marketplace_contract_address
                .to_string(),
            last_transaction_version: txn_version,
            last_transaction_timestamp: txn_timestamp,
        }
    }

    pub fn from_activity(activity: &NftMarketplaceActivity, is_deleted: bool) -> Self {
        // if not collection offer id, then we need to build it using buyer, collection_id, or something
        let collection_offer_id = if activity.offer_id.is_none() {
            // use collection_id + buyer as PK if not provided
            let input = format!(
                "{}::{}",
                standardize_address(&activity.creator_address.clone().unwrap()),
                activity.buyer.clone().unwrap()
            );
            let hash = sha3_256(input.as_bytes());
            standardize_address(&hex::encode(hash))
        } else {
            activity.offer_id.clone().unwrap()
        };

        Self {
            collection_offer_id,
            collection_id: activity.collection_id.clone().unwrap_or_default(),
            buyer: activity.buyer.clone().unwrap_or_default(),
            price: activity.price,
            remaining_token_amount: if is_deleted {
                Some(0)
            } else {
                activity.token_amount
            },
            token_standard: activity.token_standard.clone().unwrap_or_default(),
            marketplace: activity.marketplace.clone(),
            contract_address: activity.contract_address.clone(),
            is_deleted,
            last_transaction_version: activity.txn_version,
            last_transaction_timestamp: activity.block_timestamp,
        }
    }
}
