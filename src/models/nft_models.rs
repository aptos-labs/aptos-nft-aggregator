use crate::{
    config::marketplace_config::MarketplaceEventType, models::EventModel, schema::{
        current_nft_marketplace_collection_offers, current_nft_marketplace_listings,
        current_nft_marketplace_token_offers, nft_marketplace_activities,
    }
};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

pub const DEFAULT_SELLER: &str = "unknown";
pub const DEFAULT_BUYER: &str = "unknown";

const NFT_MARKETPLACE_ACTIVITIES_TABLE_NAME: &str = "nft_marketplace_activities";
const CURRENT_NFT_MARKETPLACE_LISTINGS_TABLE_NAME: &str = "current_nft_marketplace_listings";
const CURRENT_NFT_MARKETPLACE_TOKEN_OFFERS_TABLE_NAME: &str = "current_nft_marketplace_token_offers";
const CURRENT_NFT_MARKETPLACE_COLLECTION_OFFERS_TABLE_NAME: &str = "current_nft_marketplace_collection_offers";

/**
 * NftMarketplaceActivity is the main model for storing NFT marketplace activities.
*/
#[derive(Clone, Debug, Default, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(txn_version, index, marketplace))]
#[diesel(table_name = nft_marketplace_activities)]
pub struct NftMarketplaceActivity {
    pub txn_version: i64,
    pub index: i64,
    pub listing_id: Option<String>,
    pub offer_id: Option<String>,
    pub raw_event_type: String,
    #[diesel(sql_type = Text)] // Ensure compatibility with PostgreSQL
    pub standard_event_type: String,
    pub creator_address: Option<String>,
    pub collection_id: Option<String>,
    pub collection_name: Option<String>,
    pub token_data_id: Option<String>,
    pub token_name: Option<String>,
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

impl MarketplaceModel for NftMarketplaceActivity {
    fn set_field(&mut self, column_name: &str, value: String) {
        if !value.is_empty() {
            match column_name {
                "collection_id" => self.collection_id = Some(value),
                "token_data_id" => self.token_data_id = Some(value),
                "token_name" => self.token_name = Some(value),
                "creator_address" => self.creator_address = Some(value),
                "collection_name" => self.collection_name = Some(value),
                "price" => self.price = value.parse().unwrap_or(0),
                "token_amount" => self.token_amount = value.parse().ok(),
                "buyer" => self.buyer = Some(value),
                "seller" => self.seller = Some(value),
                "expiration_time" => self.expiration_time = Some(value),
                "listing_id" => self.listing_id = Some(value),
                "offer_id" | "collection_offer_id" => self.offer_id = Some(value),
                _ => {
                    tracing::debug!("Unknown column: {}", column_name);
                },
            }
        } else {
            tracing::debug!("Empty value for column: {}", column_name);
        }
    }

    fn is_valid(&self) -> bool {
        !self.marketplace.is_empty() && !self.contract_address.is_empty()
    }

    fn table_name(&self) -> &'static str {
        NFT_MARKETPLACE_ACTIVITIES_TABLE_NAME
    }

    fn updated_at(&self) -> i64 {
        self.block_timestamp.and_utc().timestamp()
    }

    fn get_field(&self, column: &str) -> Option<String> {
        match column {
            "collection_id" => Some(self.collection_id.clone().unwrap_or_default()),
            "token_data_id" => Some(self.token_data_id.clone().unwrap_or_default()),
            "token_name" => Some(self.token_name.clone().unwrap_or_default()),
            "creator_address" => Some(self.creator_address.clone().unwrap_or_default()),
            "collection_name" => Some(self.collection_name.clone().unwrap_or_default()),
            "price" => Some(self.price.to_string()),
            "token_amount" => Some(self.token_amount.unwrap_or_default().to_string()),
            "buyer" => Some(self.buyer.clone().unwrap_or_default()),
            "seller" => Some(self.seller.clone().unwrap_or_default()),
            "expiration_time" => Some(self.expiration_time.clone().unwrap_or_default()),
            "listing_id" => Some(self.listing_id.clone().unwrap_or_default()),
            "offer_id" => Some(self.offer_id.clone().unwrap_or_default()),
            "marketplace" => Some(self.marketplace.clone()),
            "contract_address" => Some(self.contract_address.clone()),
            "block_timestamp" => Some(self.block_timestamp.to_string()),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(token_data_id, marketplace))]
#[diesel(table_name = current_nft_marketplace_listings)]
pub struct CurrentNFTMarketplaceListing {
    pub token_data_id: String,
    pub listing_id: Option<String>,
    pub collection_id: Option<String>,
    pub seller: String,
    pub price: i64,
    pub token_amount: i64,
    pub is_deleted: bool,
    pub marketplace: String,
    pub contract_address: String,
    pub last_transaction_version: i64,
    pub last_transaction_timestamp: NaiveDateTime,
}

impl MarketplaceModel for CurrentNFTMarketplaceListing {
    fn set_field(&mut self, column_name: &str, value: String) {
        match column_name {
            "token_data_id" => self.token_data_id = value,
            "listing_id" => self.listing_id = Some(value),
            "collection_id" => self.collection_id = Some(value),
            "seller" => self.seller = value,
            "price" => self.price = value.parse().unwrap_or(0),
            "token_amount" => self.token_amount = value.parse().unwrap_or(0),
            "marketplace" => self.marketplace = value,
            "contract_address" => self.contract_address = value,
            _ => {
                eprintln!("Unknown column: {}", column_name);
            },
        }
    }

    // TODO: t here might be a case where the token_data_id is empty.
    fn is_valid(&self) -> bool {
        !self.token_data_id.is_empty()
    }

    fn table_name(&self) -> &'static str {
        CURRENT_NFT_MARKETPLACE_LISTINGS_TABLE_NAME
    }

    fn updated_at(&self) -> i64 {
        self.last_transaction_timestamp.and_utc().timestamp()
    }

    fn get_field(&self, column: &str) -> Option<String> {
        match column {
            "token_data_id" => Some(self.token_data_id.clone()),
            "listing_id" => Some(self.listing_id.clone().unwrap_or_default()),
            "collection_id" => Some(self.collection_id.clone().unwrap_or_default()),
            "seller" => Some(self.seller.clone()),
            "price" => Some(self.price.to_string()),
            "token_amount" => Some(self.token_amount.to_string()),
            _ => None,
        }
    }
}

impl CurrentNFTMarketplaceListing {
    pub fn build_default(marketplace_name: String, event: &EventModel, is_deleted: bool) -> Self {
        Self {
            token_data_id: String::new(),
            listing_id: None,
            collection_id: None,
            seller: String::new(),
            price: 0,
            token_amount: 0,
            is_deleted,
            marketplace: marketplace_name,
            contract_address: event.account_address.clone(),
            last_transaction_version: event.transaction_version,
            last_transaction_timestamp: event.block_timestamp,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(token_data_id, buyer, marketplace))]
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
    pub contract_address: String,
    pub last_transaction_version: i64,
    pub last_transaction_timestamp: NaiveDateTime,
}

impl MarketplaceModel for CurrentNFTMarketplaceTokenOffer {
    fn set_field(&mut self, column_name: &str, value: String) {
        match column_name {
            "token_data_id" => self.token_data_id = value,
            "offer_id" => self.offer_id = Some(value),
            "buyer" => self.buyer = value,
            "collection_id" => self.collection_id = value,
            "price" => self.price = value.parse().unwrap_or(0),
            "token_amount" => self.token_amount = value.parse().ok(),
            "token_name" => self.token_name = Some(value),
            "marketplace" => self.marketplace = value,
            "contract_address" => self.contract_address = value,
            _ => {
                eprintln!("Unknown column: {}", column_name);
            },
        }
    }

    // TODO: t here might be a case where the token_data_id is empty.
    fn is_valid(&self) -> bool {
        !self.token_data_id.is_empty() && !self.buyer.is_empty()
    }

    fn table_name(&self) -> &'static str {
        CURRENT_NFT_MARKETPLACE_TOKEN_OFFERS_TABLE_NAME
    }

    fn updated_at(&self) -> i64 {
        self.last_transaction_timestamp.and_utc().timestamp()
    }

    fn get_field(&self, column: &str) -> Option<String> {
        match column {
            "token_data_id" => Some(self.token_data_id.clone()),
            "token_name" => Some(self.token_name.clone().unwrap_or_default()),
            "buyer" => Some(self.buyer.clone()),
            "collection_id" => Some(self.collection_id.clone()),
            "price" => Some(self.price.to_string()),
            "token_amount" => Some(self.token_amount.unwrap_or_default().to_string()),
            "marketplace" => Some(self.marketplace.clone()),
            "contract_address" => Some(self.contract_address.clone()),
            "last_transaction_version" => Some(self.last_transaction_version.to_string()),
            "last_transaction_timestamp" => Some(self.last_transaction_timestamp.to_string()),
            _ => None,
        }
    }
}

impl CurrentNFTMarketplaceTokenOffer {
    pub fn build_default(marketplace_name: String, event: &EventModel, is_deleted: bool) -> Self {
        Self {
            token_data_id: String::new(),
            offer_id: None,
            buyer: String::new(),
            collection_id: String::new(),
            price: 0,
            token_amount: Some(0),
            token_name: None,
            is_deleted,
            marketplace: marketplace_name,
            contract_address: event.account_address.clone(),
            last_transaction_version: event.transaction_version,
            last_transaction_timestamp: event.block_timestamp,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(collection_offer_id, marketplace))]
#[diesel(table_name = current_nft_marketplace_collection_offers)]
pub struct CurrentNFTMarketplaceCollectionOffer {
    pub collection_offer_id: String,
    pub collection_id: String,
    pub token_data_id: Option<String>,
    pub buyer: String,
    pub price: i64,
    pub remaining_token_amount: Option<i64>,
    pub is_deleted: bool,
    pub marketplace: String,
    pub contract_address: String,
    pub last_transaction_version: i64,
    pub last_transaction_timestamp: NaiveDateTime,
}

impl MarketplaceModel for CurrentNFTMarketplaceCollectionOffer {
    fn set_field(&mut self, column_name: &str, value: String) {
        match column_name {
            "collection_offer_id" => self.collection_offer_id = value,
            "collection_id" => self.collection_id = value,
            "token_data_id" => self.token_data_id = Some(value),
            "buyer" => self.buyer = value,
            "price" => self.price = value.parse().unwrap_or(0),
            "remaining_token_amount" => self.remaining_token_amount = value.parse().ok(),
            "marketplace" => self.marketplace = value,
            "contract_address" => self.contract_address = value,
            _ => {
                eprintln!("Unknown column: {}", column_name);
            },
        }
    }

    fn is_valid(&self) -> bool {
        !self.collection_offer_id.is_empty()
    }

    fn table_name(&self) -> &'static str {
        CURRENT_NFT_MARKETPLACE_COLLECTION_OFFERS_TABLE_NAME
    }

    fn updated_at(&self) -> i64 {
        self.last_transaction_timestamp.and_utc().timestamp()
    }

    fn get_field(&self, column: &str) -> Option<String> {
        match column {
            "collection_offer_id" => Some(self.collection_offer_id.clone()),
            "collection_id" => Some(self.collection_id.clone()),
            "price" => Some(self.price.to_string()),
            "remaining_token_amount" => {
                Some(self.remaining_token_amount.unwrap_or_default().to_string())
            },
            "marketplace" => Some(self.marketplace.clone()),
            "contract_address" => Some(self.contract_address.clone()),
            "buyer" => Some(self.buyer.clone()),
            "token_data_id" => Some(self.token_data_id.clone().unwrap_or_default()),
            _ => None,
        }
    }
}

impl CurrentNFTMarketplaceCollectionOffer {
    pub fn build_default(marketplace_name: String, event: &EventModel, is_deleted: bool) -> Self {
        Self {
            collection_offer_id: String::new(),
            collection_id: String::new(),
            token_data_id: None,
            buyer: String::new(),
            price: 0,
            remaining_token_amount: if is_deleted { Some(0) } else { None },
            is_deleted,
            marketplace: marketplace_name,
            contract_address: event.account_address.clone(),
            last_transaction_version: event.transaction_version,
            last_transaction_timestamp: event.block_timestamp,
        }
    }
}

pub trait MarketplaceModel {
    fn set_field(&mut self, column: &str, value: String);
    fn is_valid(&self) -> bool;
    fn table_name(&self) -> &'static str;
    fn updated_at(&self) -> i64; // Returns last updated timestamp
    fn get_field(&self, column: &str) -> Option<String>;
}
