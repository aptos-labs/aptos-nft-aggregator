use crate::{
    config::marketplace_config::MarketplaceEventType,
    schema::{
        current_nft_marketplace_collection_offers, current_nft_marketplace_listings,
        current_nft_marketplace_token_offers, nft_marketplace_activities,
    },
};
use aptos_indexer_processor_sdk::utils::convert::{sha3_256, standardize_address};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use tracing::warn;

pub const DEFAULT_SELLER: &str = "unknown";
pub const DEFAULT_BUYER: &str = "unknown";
/**
 * NftMarketplaceActivity is the main model for storing NFT marketplace activities.
*/
#[derive(Clone, Debug, Default, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(txn_version, index))]
#[diesel(table_name = nft_marketplace_activities)]
pub struct NftMarketplaceActivity {
    pub txn_version: i64,
    pub index: i64,
    pub listing_id: Option<String>,
    pub offer_id: Option<String>,
    pub raw_event_type: String,
    #[diesel(sql_type = Text)] // Ensure compatibility with PostgreSQL
    pub standard_event_type: MarketplaceEventType,
    pub creator_address: Option<String>,
    pub collection_id: Option<String>,
    pub collection_name: Option<String>,
    pub token_data_id: Option<String>,
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
    /// Dynamically sets a field in the activity struct
    pub fn set_field(&mut self, column_name: &str, value: String) {
        match column_name {
            "collection_id" => self.collection_id = Some(value),
            "token_data_id" => self.token_data_id = Some(value),
            "token_name" => self.token_name = Some(value),
            "creator_address" => self.creator_address = Some(value),
            "collection_name" => self.collection_name = Some(value),
            "price" => self.price = value.parse().unwrap_or(0), // Default to 0 if parsing fails
            "token_amount" => self.token_amount = value.parse().ok(),
            "buyer" => self.buyer = Some(value),
            "seller" => self.seller = Some(value),
            "expiration_time" => self.expiration_time = Some(value),
            "listing_id" => self.listing_id = Some(value),
            "offer_id" | "collection_offer_id" => self.offer_id = Some(value),
            _ => {
                eprintln!("Unknown column: {}", column_name);
            },
        }
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
    pub fn from_activity(activity: &NftMarketplaceActivity) -> Self {
        let is_deleted = activity.standard_event_type == MarketplaceEventType::CancelListing
            || activity.standard_event_type == MarketplaceEventType::FillListing;

        Self {
            token_data_id: activity.token_data_id.clone().unwrap_or_else(|| {
                panic!(
                    "token_data_id is required for listing for txn_version = {:?}",
                    activity.txn_version
                )
            }),
            marketplace: activity.marketplace.clone(),
            listing_id: activity.listing_id.clone(),
            collection_id: activity.collection_id.clone(),
            seller: activity.seller.clone().unwrap_or_else(|| {
                warn!(
                    "seller is not found for listing for txn_version = {:?}",
                    activity.txn_version
                );
                DEFAULT_SELLER.to_string()
            }),
            price: activity.price,
            token_amount: if is_deleted {
                0
            } else {
                activity.token_amount.unwrap_or_else(|| {
                    if activity.standard_event_type == MarketplaceEventType::PlaceListing {
                        1
                    } else {
                        0
                    }
                })
            },
            token_standard: activity.token_standard.clone().unwrap_or_default(),
            is_deleted,
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
    pub fn from_activity(activity: &NftMarketplaceActivity) -> Self {
        let is_deleted = activity.standard_event_type == MarketplaceEventType::CancelTokenOffer
            || activity.standard_event_type == MarketplaceEventType::FillTokenOffer;

        Self {
            token_data_id: activity.token_data_id.clone().unwrap_or_else(|| {
                panic!(
                    "token_data_id is required for token offer for txn_version = {:?}",
                    activity.txn_version
                )
            }),
            offer_id: activity.offer_id.clone(),
            buyer: activity.buyer.clone().unwrap_or_else(|| {
                warn!(
                    "buyer is not found for token offer for txn_version = {:?}",
                    activity.txn_version
                );
                DEFAULT_BUYER.to_string()
            }),
            collection_id: activity.collection_id.clone().unwrap_or_default(),
            price: activity.price,
            token_amount: if is_deleted {
                Some(0)
            } else {
                Some(activity.token_amount.unwrap_or_else(|| {
                    if activity.standard_event_type == MarketplaceEventType::PlaceTokenOffer {
                        1
                    } else {
                        0
                    }
                }))
            },
            token_name: activity.token_name.clone(),
            is_deleted,
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
    pub fn from_activity(activity: &NftMarketplaceActivity) -> Self {
        let is_deleted = activity.standard_event_type
            == MarketplaceEventType::CancelCollectionOffer
            || activity.standard_event_type == MarketplaceEventType::FillCollectionOffer;

        // if not collection offer id, then we need to build it using buyer, collection_id
        let collection_offer_id = if activity.offer_id.is_none() {
            // use collection_id + buyer as PK if not provided
            let input = format!(
                "{}::{}",
                standardize_address(&activity.collection_id.clone().unwrap()),
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
                Some(activity.token_amount.unwrap_or_else(|| {
                    if activity.standard_event_type == MarketplaceEventType::PlaceCollectionOffer {
                        1
                    } else {
                        0
                    }
                }))
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
