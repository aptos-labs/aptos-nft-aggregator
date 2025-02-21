use super::{
    extract_bigdecimal, extract_string,
    marketplace_config::{
        ContractToMarketplaceMap, MarketplaceEventConfig, MarketplaceEventConfigMappings,
    },
};
use crate::schema::{
    current_nft_marketplace_bids, current_nft_marketplace_collection_bids,
    current_nft_marketplace_listings, nft_marketplace_activities, nft_marketplace_bids,
    nft_marketplace_collection_bids, nft_marketplace_listings,
};
use aptos_indexer_processor_sdk::utils::convert::standardize_address;
use aptos_protos::transaction::v1::Event;
use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tracing::debug;

/**
 * NftMarketplaceActivity is the main model for storing NFT marketplace activities.
 *
*/
#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(txn_version, event_index))]
#[diesel(table_name = nft_marketplace_activities)]
pub struct NftMarketplaceActivity {
    pub txn_version: i64,
    pub event_index: i64,
    pub raw_event_type: String,
    pub standard_event_type: String,
    pub creator_address: Option<String>,
    pub collection_id: Option<String>,
    pub collection_name: Option<String>,
    pub token_data_id: Option<String>, // for v1, we don't use property version.
    pub token_name: Option<String>,
    pub token_standard: Option<String>,
    pub price: Option<BigDecimal>,
    pub token_amount: Option<BigDecimal>,
    pub buyer: Option<String>,
    pub seller: Option<String>,
    pub json_data: serde_json::Value,
    pub marketplace: String,
    pub contract_address: String,
    pub entry_function_id_str: Option<String>,
    pub transaction_timestamp: NaiveDateTime,
}

impl NftMarketplaceActivity {
    /// Constructs an `NftMarketplaceActivity` from an event.
    pub fn from_event(
        event: &Event,
        txn_version: i64,
        event_index: i64,
        timestamp: NaiveDateTime,
        entry_function_id_str: &Option<String>,
        event_mappings: &MarketplaceEventConfigMappings,
        contract_to_marketplace_map: &ContractToMarketplaceMap,
    ) -> Option<Self> {
        // Extract the contract address from the event type string.
        let contract_address = event.type_str.clone();
        let marketplace_name = match contract_to_marketplace_map.get(&contract_address) {
            Some(name) => name,
            None => {
                debug!(
                    "Marketplace not found for the given contract address: {}",
                    contract_address
                );
                return None;
            },
        };

        if let Some(event_mapping) = event_mappings.get(marketplace_name) {
            let event_type: String = event.type_str.to_string();
            let event_data: Value = serde_json::from_str(event.data.as_str()).unwrap();

            // Check if there is a configuration for the event type.
            if let Some(config) = event_mapping.get(&event_type) {
                let standard_event_type = config.event_type.as_str().to_string();

                // Extract various fields from the event data using helper functions.
                let creator_address = Self::extract_creator_address(config, &event_data);
                let price = extract_bigdecimal(&config.price, &event_data);
                let token_amount = extract_bigdecimal(&config.token_amount, &event_data);
                let collection_name = extract_string(&config.collection_name, &event_data);
                let token_name = extract_string(&config.token_name, &event_data);

                // Extract token data ID and collection ID.
                let token_data_id = Self::extract_token_data_id(
                    config,
                    &event_data,
                    &creator_address,
                    &collection_name,
                    &token_name,
                );

                let collection_id = Self::extract_collection_id(
                    config,
                    &event_data,
                    &creator_address,
                    &collection_name,
                );

                // Construct the `NftMarketplaceActivity` instance.
                let activity = Self {
                    txn_version,
                    event_index,
                    raw_event_type: event_type.clone(),
                    standard_event_type,
                    creator_address,
                    collection_id,
                    collection_name,
                    token_data_id,
                    token_name,
                    token_standard: Self::determine_token_standard(&event_data),
                    price: Some(price),
                    token_amount: Some(token_amount),
                    buyer: extract_string(&config.buyer, &event_data)
                        .map(|s| standardize_address(&s)),
                    seller: extract_string(&config.seller, &event_data)
                        .map(|s| standardize_address(&s)),
                    json_data: event_data,
                    marketplace: config.marketplace.clone(),
                    contract_address: contract_address.clone(),
                    entry_function_id_str: entry_function_id_str.clone(),
                    transaction_timestamp: timestamp,
                };
                Some(activity)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Extracts the creator address from the event data.
    fn extract_creator_address(
        config: &MarketplaceEventConfig,
        event_data: &Value,
    ) -> Option<String> {
        extract_string(&config.creator_address, event_data).map(|addr| standardize_address(&addr))
    }

    fn extract_token_data_id(
        config: &MarketplaceEventConfig,
        event_data: &Value,
        creator_address: &Option<String>,
        collection_name: &Option<String>,
        token_name: &Option<String>,
    ) -> Option<String> {
        if let Some(inner_str) = extract_string(&config.token_inner, event_data) {
            return Some(standardize_address(&inner_str));
        }

        if creator_address.is_none() || collection_name.is_none() || token_name.is_none() {
            debug!("Missing fields for token data ID extraction");
            return None;
        }

        let token_data_id_type = TokenDataIdType::new(
            creator_address.clone(),
            collection_name.clone(),
            token_name.clone(),
        );

        Some(token_data_id_type.to_hash())
    }

    fn extract_inner_collection(
        config: &MarketplaceEventConfig,
        event_data: &Value,
    ) -> Option<String> {
        extract_string(&config.collection_inner, event_data)
    }

    fn extract_collection_id(
        config: &MarketplaceEventConfig,
        event_data: &Value,
        creator_address: &Option<String>,
        collection_name: &Option<String>,
    ) -> Option<String> {
        if let Some(inner_str) = Self::extract_inner_collection(config, event_data) {
            return Some(standardize_address(&inner_str));
        }

        // if any of fields are None, we should return None
        if creator_address.is_none() || collection_name.is_none() {
            debug!("Missing fields for collection ID extraction");
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
}

#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(transaction_version,))]
#[diesel(table_name = nft_marketplace_listings)]
pub struct NftMarketplaceListing {
    pub transaction_version: i64,
    pub creator_address: Option<String>,
    pub token_name: Option<String>,
    pub token_data_id: Option<String>,
    pub collection_name: Option<String>,
    pub collection_id: Option<String>,
    pub price: Option<BigDecimal>,
    pub token_amount: Option<BigDecimal>,
    pub token_standard: Option<String>,
    pub seller: Option<String>,
    // pub buyer: Option<String>,
    pub marketplace: String,
    pub contract_address: String,
    pub entry_function_id_str: String,
    pub event_type: Option<String>,
    pub transaction_timestamp: NaiveDateTime,
}

#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(token_data_id))]
#[diesel(table_name = current_nft_marketplace_listings)]
pub struct CurrentNftMarketplaceListing {
    pub token_data_id: Option<String>,
    pub creator_address: Option<String>,
    pub token_name: Option<String>,
    pub collection_name: Option<String>,
    pub collection_id: Option<String>,
    pub price: Option<BigDecimal>,
    pub token_amount: Option<BigDecimal>,
    pub token_standard: Option<String>,
    pub seller: Option<String>,
    pub is_deleted: bool,
    pub marketplace: String,
    pub contract_address: String,
    pub entry_function_id_str: String,
    pub event_type: Option<String>,
    pub last_transaction_version: Option<i64>,
    pub last_transaction_timestamp: NaiveDateTime,
}

impl NftMarketplaceListing {
    pub fn from_activity(activity: &NftMarketplaceActivity) -> Self {
        // Handle Option fields with defaults or error handling
        let entry_function_id_str: String =
            activity.entry_function_id_str.clone().unwrap_or_default();
        Self {
            transaction_version: activity.txn_version,
            creator_address: activity.creator_address.clone(),
            token_name: activity.token_name.clone(),
            token_data_id: activity.token_data_id.clone(),
            collection_name: activity.collection_name.clone(),
            collection_id: activity.collection_id.clone(),
            price: activity.price.clone(),
            token_amount: activity.token_amount.clone(),
            token_standard: activity.token_standard.clone(),
            seller: activity.seller.clone(),
            marketplace: activity.marketplace.clone(),
            contract_address: activity.contract_address.clone(),
            entry_function_id_str,
            event_type: Some(activity.standard_event_type.clone()),
            transaction_timestamp: activity.transaction_timestamp,
        }
    }

    pub fn from_activity_to_current(
        activity: &NftMarketplaceActivity,
        is_deleted: bool,
    ) -> (Self, CurrentNftMarketplaceListing) {
        let listing = Self::from_activity(activity);

        let current_listing = CurrentNftMarketplaceListing {
            token_data_id: listing.token_data_id.clone(),
            creator_address: listing.creator_address.clone(),
            token_name: listing.token_name.clone(),
            collection_name: listing.collection_name.clone(),
            collection_id: listing.collection_id.clone(),
            price: listing.price.clone(),
            token_amount: listing.token_amount.clone(),
            token_standard: listing.token_standard.clone(),
            seller: listing.seller.clone(),
            is_deleted,
            marketplace: listing.marketplace.clone(),
            contract_address: listing.contract_address.clone(),
            entry_function_id_str: listing.entry_function_id_str.clone(),
            event_type: listing.event_type.clone(),
            last_transaction_version: Some(activity.txn_version),
            last_transaction_timestamp: listing.transaction_timestamp,
        };

        (listing, current_listing)
    }
}

// Non-current tables
#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(transaction_version, event_index))]
#[diesel(table_name = nft_marketplace_bids)]
pub struct NftMarketplaceBid {
    pub transaction_version: i64,
    pub event_index: i64,
    pub token_data_id: String,
    pub buyer: String,
    pub price: BigDecimal,
    pub creator_address: Option<String>,
    pub token_amount: Option<BigDecimal>,
    pub token_name: Option<String>,
    pub collection_name: Option<String>,
    pub collection_id: Option<String>,
    pub marketplace: String,
    pub contract_address: String,
    pub entry_function_id_str: String,
    pub event_type: String,
    pub transaction_timestamp: NaiveDateTime,
}

impl NftMarketplaceBid {
    pub fn from_activity(activity: &NftMarketplaceActivity) -> Self {
        Self {
            transaction_version: activity.txn_version,
            event_index: activity.event_index,
            token_data_id: activity.token_data_id.clone().unwrap_or_default(),
            buyer: activity.buyer.clone().unwrap_or_default(),
            price: activity.price.clone().unwrap_or_default(),
            creator_address: activity.creator_address.clone(),
            token_amount: activity.token_amount.clone(),
            token_name: activity.token_name.clone(),
            collection_name: activity.collection_name.clone(),
            collection_id: activity.collection_id.clone(),
            marketplace: activity.marketplace.clone(),
            contract_address: activity.contract_address.clone(),
            entry_function_id_str: activity.entry_function_id_str.clone().unwrap_or_default(),
            event_type: activity.standard_event_type.clone(),
            transaction_timestamp: activity.transaction_timestamp,
        }
    }

    pub fn from_activity_to_current(
        activity: &NftMarketplaceActivity,
        is_deleted: bool,
    ) -> (Self, CurrentNftMarketplaceBid) {
        let bid = Self::from_activity(activity);
        let current_bid = CurrentNftMarketplaceBid::from_activity(activity, is_deleted);
        (bid, current_bid)
    }
}

#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(transaction_version))]
#[diesel(table_name = nft_marketplace_collection_bids)]
pub struct NftMarketplaceCollectionBid {
    pub transaction_version: i64,
    pub event_index: Option<i64>,
    pub creator_address: Option<String>,
    pub collection_name: Option<String>,
    pub collection_id: Option<String>,
    pub price: BigDecimal,
    pub token_amount: Option<BigDecimal>,
    pub buyer: Option<String>,
    pub seller: Option<String>,
    pub marketplace: String,
    pub contract_address: String,
    pub entry_function_id_str: String,
    pub event_type: String,
    pub transaction_timestamp: NaiveDateTime,
}

impl NftMarketplaceCollectionBid {
    pub fn from_activity(activity: &NftMarketplaceActivity) -> Self {
        Self {
            transaction_version: activity.txn_version,
            event_index: Some(activity.event_index),
            creator_address: activity.creator_address.clone(),
            collection_name: activity.collection_name.clone(),
            collection_id: activity.collection_id.clone(),
            price: activity.price.clone().unwrap_or_default(),
            token_amount: activity.token_amount.clone(),
            buyer: activity.buyer.clone(),
            seller: activity.seller.clone(),
            marketplace: activity.marketplace.clone(),
            contract_address: activity.contract_address.clone(),
            entry_function_id_str: activity.entry_function_id_str.clone().unwrap_or_default(),
            event_type: activity.standard_event_type.clone(),
            transaction_timestamp: activity.transaction_timestamp,
        }
    }

    pub fn from_activity_to_current(
        activity: &NftMarketplaceActivity,
        is_deleted: bool,
    ) -> (Self, CurrentNftMarketplaceCollectionBid) {
        let bid = Self::from_activity(activity);
        let current_bid = CurrentNftMarketplaceCollectionBid::from_activity(activity, is_deleted);
        (bid, current_bid)
    }
}

#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(token_data_id, buyer, price))]
#[diesel(table_name = current_nft_marketplace_bids)]
pub struct CurrentNftMarketplaceBid {
    pub token_data_id: String,
    pub buyer: String,
    pub price: BigDecimal,
    pub creator_address: Option<String>,
    pub token_amount: Option<BigDecimal>,
    pub token_name: Option<String>,
    pub collection_name: Option<String>,
    pub collection_id: Option<String>,
    pub marketplace: String,
    pub contract_address: String,
    pub entry_function_id_str: String,
    pub is_deleted: bool,
    pub last_transaction_version: Option<i64>,
    pub last_transaction_timestamp: NaiveDateTime,
}

impl CurrentNftMarketplaceBid {
    pub fn from_activity(activity: &NftMarketplaceActivity, is_deleted: bool) -> Self {
        Self {
            token_data_id: activity.token_data_id.clone().unwrap_or_default(),
            buyer: activity.buyer.clone().unwrap_or_default(),
            price: activity.price.clone().unwrap_or_default(),
            creator_address: activity.creator_address.clone(),
            token_amount: activity.token_amount.clone(),
            token_name: activity.token_name.clone(),
            collection_name: activity.collection_name.clone(),
            collection_id: activity.collection_id.clone(),
            marketplace: activity.marketplace.clone(),
            contract_address: activity.contract_address.clone(),
            entry_function_id_str: activity.entry_function_id_str.clone().unwrap_or_default(),
            is_deleted,
            last_transaction_version: Some(activity.txn_version),
            last_transaction_timestamp: activity.transaction_timestamp,
        }
    }
}

#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(collection_id, buyer, price))]
#[diesel(table_name = current_nft_marketplace_collection_bids)]
pub struct CurrentNftMarketplaceCollectionBid {
    pub collection_id: String,
    pub buyer: Option<String>,
    pub price: BigDecimal,
    pub creator_address: Option<String>,
    pub token_amount: Option<BigDecimal>,
    pub collection_name: Option<String>,
    pub marketplace: String,
    pub contract_address: String,
    pub entry_function_id_str: String,
    pub coin_type: Option<String>,
    pub expiration_time: i64,
    pub is_deleted: bool,
    pub last_transaction_version: Option<i64>,
    pub last_transaction_timestamp: NaiveDateTime,
}

impl CurrentNftMarketplaceCollectionBid {
    pub fn from_activity(activity: &NftMarketplaceActivity, is_deleted: bool) -> Self {
        Self {
            collection_id: activity.collection_id.clone().unwrap_or_default(),
            buyer: activity.buyer.clone(),
            price: activity.price.clone().unwrap_or_default(),
            creator_address: activity.creator_address.clone(),
            token_amount: activity.token_amount.clone(),
            collection_name: activity.collection_name.clone(),
            marketplace: activity.marketplace.clone(),
            contract_address: activity.contract_address.clone(),
            entry_function_id_str: activity.entry_function_id_str.clone().unwrap_or_default(),
            coin_type: None,
            expiration_time: 0,
            is_deleted,
            last_transaction_version: Some(activity.txn_version),
            last_transaction_timestamp: activity.transaction_timestamp,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TokenDataIdType {
    creator: Option<String>,
    collection: Option<String>,
    name: Option<String>,
}

impl TokenDataIdType {
    fn new(creator: Option<String>, collection: Option<String>, name: Option<String>) -> Self {
        Self {
            creator,
            collection,
            name,
        }
    }

    fn to_hash(&self) -> String {
        let mut hasher = Sha256::new();

        hasher.update(format!(
            "{}::{}::{}",
            {
                let creator_address = self.creator.clone().unwrap_or_default();
                debug!("Standardizing creator address: {}", creator_address);
                standardize_address(&creator_address)
            },
            self.collection.clone().unwrap_or_default(),
            self.name.clone().unwrap_or_default()
        ));

        let result = hasher.finalize();

        format!("{:x}", result)
    }
}

// fn truncate_str(s: &str, max_len: usize) -> String {
//     if s.len() > max_len {
//         s[..max_len].to_string()
//     } else {
//         s.to_string()
//     }
// }

struct CollectionDataIdType {
    creator: Option<String>,
    collection_name: Option<String>,
}

impl CollectionDataIdType {
    fn new(creator: Option<String>, collection_name: Option<String>) -> Self {
        Self {
            creator,
            collection_name,
        }
    }

    fn to_hash(&self) -> String {
        let mut hasher = Sha256::new();

        hasher.update(format!(
            "{}::{}",
            {
                let creator_address = self.creator.clone().unwrap_or_default();
                debug!("Standardizing creator address: {}", creator_address);
                standardize_address(&creator_address)
            },
            self.collection_name.clone().unwrap_or_default()
        ));

        let result = hasher.finalize();
        format!("{:x}", result)
    }
}
