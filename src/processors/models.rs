use super::{
    extract_bigdecimal, extract_string,
    marketplace_config::{
        ContractToMarketplaceMap, MarketplaceEventConfig, MarketplaceEventConfigMappings,
    },
};
use crate::schema::nft_marketplace_activities;
use aptos_indexer_processor_sdk::utils::convert::standardize_address;
use aptos_protos::transaction::v1::Event;
use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tracing::{debug, warn};

/**
 * NftMarketplaceActivity is the main model for storing NFT marketplace activities.
 *
*/
#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(txn_version, index))]
#[diesel(table_name = nft_marketplace_activities)]
pub struct NftMarketplaceActivity {
    pub txn_version: i64,
    pub index: i64,
    pub raw_event_type: String,
    pub standard_event_type: String,
    pub creator_address: Option<String>,
    pub collection_id: Option<String>,
    pub collection_name: Option<String>,
    pub token_data_id: Option<String>,
    pub token_name: Option<String>,
    pub token_standard: Option<String>,
    pub price: Option<BigDecimal>,
    pub token_amount: Option<BigDecimal>,
    pub buyer: Option<String>,
    pub seller: Option<String>,
    pub deadline: Option<String>,
    pub json_data: serde_json::Value,
    pub marketplace: String,
    pub contract_address: String,
    pub entry_function_id_str: Option<String>,
    pub block_timestamp: NaiveDateTime,
}

impl NftMarketplaceActivity {
    pub fn from_event(
        event: &Event,
        txn_version: i64,
        event_index: i64,
        block_timestamp: NaiveDateTime,
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
                let creator_address = extract_string(&config.creator_address, &event_data);
                let price = extract_bigdecimal(&config.price, &event_data);
                let token_amount = extract_bigdecimal(&config.token_amount, &event_data);
                let collection_name = extract_string(&config.collection_name, &event_data);
                let token_name = extract_string(&config.token_name, &event_data);
                let deadline = extract_string(&config.deadline, &event_data);
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
                    txn_version,
                );

                // Construct the `NftMarketplaceActivity` instance.
                let activity = Self {
                    txn_version,
                    index: event_index,
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
                    deadline,
                    json_data: event_data,
                    marketplace: config.marketplace.clone(),
                    contract_address: contract_address.clone(),
                    entry_function_id_str: entry_function_id_str.clone(),
                    block_timestamp,
                };
                Some(activity)
            } else {
                None
            }
        } else {
            None
        }
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

        // if it's a v1 token
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

        // if any of fields are None, we should return None
        if creator_address.is_none() || collection_name.is_none() {
            warn!(
                "Missing fields for collection ID extraction {:?}",
                txn_version
            );
            return None;
        }

        // if it's a v1 collection
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
