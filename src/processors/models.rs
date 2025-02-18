use super::{
    extract_bigdecimal, extract_string, marketplace_config::MarketplaceEventConfigMapping,
};
use crate::{
    processors::util::{CollectionMetadata, TokenMetadata},
    schema::{
        current_nft_marketplace_bids, current_nft_marketplace_collection_bids,
        current_nft_marketplace_listings, nft_marketplace_activities, nft_marketplace_bids,
        nft_marketplace_collection_bids, nft_marketplace_listings,
    },
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
use std::{collections::HashMap, fmt};

// const MAX_NAME_LENGTH: usize = 128;

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
    pub token_data_id: Option<String>,    // optional v1 from  v2 = standardzie resource address
    pub token_id: Option<String>,         // optional
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
    pub fn from_event(
        event: &Event,
        txn_version: i64,
        event_index: i64,
        timestamp: NaiveDateTime,
        entry_function_id_str: &Option<String>,
        event_mapping: &MarketplaceEventConfigMapping,
        token_metadatas: &mut HashMap<String, TokenMetadata>,
        collection_metadatas: &mut HashMap<String, CollectionMetadata>,
    ) -> Option<Self> {
        let event_type: String = event.type_str.to_string();
        println!("Event Type: {:?}", event_type);
        let event_data: Value = serde_json::from_str(event.data.as_str()).unwrap();
        println!("Event Data: {:?}", event_data);

        if let Some(config) = event_mapping.get(&event_type) {
            println!("Config: {:?}", config);
            let standard_event_type = config.event_type.as_str().to_string();

            // TODO: what default value should we use here?
            let creator_address = extract_string(&config.creator_address, &event_data)
                .map(|addr| standardize_address(&addr))
                .unwrap_or_else(|| "default_creator_address".to_string());
    

            // Handle price extraction with default value
            let price = extract_bigdecimal(&config.price, &event_data);

            // Handle token_amount extraction with default value
            let token_amount = extract_bigdecimal(&config.token_amount, &event_data);

            let collection_name =
                extract_string(&config.collection_name, &event_data).unwrap_or_default();
            let token_name = extract_string(&config.token_name, &event_data).unwrap_or_default();

            // TODO: what default value should we use here?
            let property_version: String = extract_string(&config.property_version, &event_data).unwrap_or_default();


            let collection_id = if !config.collection_id.raw.is_none() {
                let collection_id = standardize_address(
                    &extract_string(&config.collection_id, &event_data).unwrap(),
                );
                Some(collection_id)
            } else {
                let collection_data_id_type =
                    CollectionDataIdType::new(creator_address.clone(), collection_name.clone());
                let collection_id = collection_data_id_type.to_hash();
                Some(collection_id)
            };

            
            let token_v2: Option<&Vec<Value>> = event_data
                .get("token")
                .and_then(|t: &Value| t.get("vec").and_then(|v| v.as_array()));

            let (token_id, token_data_id) = if token_v2.is_some() {
                // let token_data_id = standardize_address(token_v2[0]["inner"].as_str().unwrap());
                ("token_id_v2".to_string(), "token_data_id_v2".to_string())
            } else {
                let token_data_id_type = TokenDataIdType::new(
                    creator_address.clone(),
                    collection_name.clone(),
                    token_name.clone(),
                );
                let token_data_id = token_data_id_type.to_hash();
                let token_id_type: TokenIdType =
                    TokenIdType::new(token_data_id.clone(), property_version.clone());
                let token_id: String = token_id_type.to_hash();
                (token_id, token_data_id)
            };

            let activity = Self {
                txn_version,
                event_index,
                raw_event_type: event_type.clone(),
                standard_event_type,
                creator_address: Some(creator_address),
                collection_id,
                collection_name: Some(collection_name),
                token_data_id: Some(token_data_id.clone()),
                token_id: Some(token_id.clone()),
                token_name: Some(token_name),
                token_standard: if token_v2.is_some() {
                    Some("v2".to_string())
                } else {
                    Some("v1".to_string())
                },
                price: Some(price),
                token_amount: Some(token_amount),
                buyer: extract_string(&config.buyer, &event_data).map(|s| standardize_address(&s)),
                seller: extract_string(&config.seller, &event_data)
                    .map(|s| standardize_address(&s)),
                json_data: event_data,
                marketplace: config.marketplace.clone(),
                contract_address: config.contract_address.clone(),
                entry_function_id_str: entry_function_id_str.clone(),
                transaction_timestamp: timestamp,
            };

            let token_metadata = TokenMetadata {
                collection_id: activity.collection_id.clone().unwrap_or_default(),
                token_data_id: activity.token_data_id.clone().unwrap_or_default(),
                creator_address: activity.creator_address.clone().unwrap_or_default(),
                collection_name: activity.collection_name.clone().unwrap_or_default(),
                token_name: activity.token_name.clone().unwrap_or_default(),
                property_version: None, // or extract if available
                token_standard: activity.token_standard.clone().unwrap_or_default(),
            };

            let collection_metadata = CollectionMetadata {
                collection_id: activity.collection_id.clone().unwrap_or_default(),
                creator_address: activity.creator_address.clone().unwrap_or_default(),
                collection_name: activity.collection_name.clone().unwrap_or_default(),
                token_standard: activity.token_standard.clone().unwrap_or_default(),
            };

            // TODO: check if this is correct
            token_metadatas.insert(token_metadata.token_data_id.clone(), token_metadata);
            collection_metadatas.insert(
                collection_metadata.collection_id.clone(),
                collection_metadata,
            );

            Some(activity)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(transaction_version,))]
#[diesel(table_name = nft_marketplace_listings)]
pub struct NftMarketplaceListing {
    pub transaction_version: i64,
    pub token_id: String,
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
    // pub event_type: String, // not available from wscs
    pub transaction_timestamp: NaiveDateTime,
}

#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(token_id))]
#[diesel(table_name = current_nft_marketplace_listings)]
pub struct CurrentNftMarketplaceListing {
    pub token_id: String,
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
    pub last_transaction_version: Option<i64>,
    pub last_transaction_timestamp: NaiveDateTime,
}

impl NftMarketplaceListing {
    pub fn from_activity(activity: &NftMarketplaceActivity) -> Self {
        // Handle Option fields with defaults or error handling
        let entry_function_id_str: String =
            activity.entry_function_id_str.clone().unwrap_or_default();
        Self {
            token_id: activity.token_id.clone().unwrap(),
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
            transaction_timestamp: activity.transaction_timestamp,
        }
    }

    pub fn from_activity_to_current(
        activity: &NftMarketplaceActivity,
    ) -> (Self, CurrentNftMarketplaceListing) {
        let listing = Self::from_activity(activity);

        let current_listing = CurrentNftMarketplaceListing {
            token_id: listing.token_id.clone(),
            token_data_id: listing.token_data_id.clone(),
            creator_address: listing.creator_address.clone(),
            token_name: listing.token_name.clone(),
            collection_name: listing.collection_name.clone(),
            collection_id: listing.collection_id.clone(),
            price: listing.price.clone(),
            token_amount: listing.token_amount.clone(),
            token_standard: listing.token_standard.clone(),
            seller: listing.seller.clone(),
            is_deleted: false,
            marketplace: listing.marketplace.clone(),
            contract_address: listing.contract_address.clone(),
            entry_function_id_str: listing.entry_function_id_str.clone(),
            last_transaction_version: None,
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
    pub token_id: Option<String>,
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
            token_id: activity.token_id.clone(),
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
}

#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(token_data_id, buyer, price))]
#[diesel(table_name = current_nft_marketplace_bids)]
pub struct CurrentNftMarketplaceBid {
    pub token_id: Option<String>,
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

#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(collection_id, buyer, price))]
#[diesel(table_name = current_nft_marketplace_collection_bids)]
pub struct CurrentNftMarketplaceCollectionBid {
    pub collection_offer_id: String,
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
#[derive(Debug, Clone, Deserialize, Serialize)]
struct TokenDataIdType {
    creator: String,
    collection: String,
    name: String,
}

impl TokenDataIdType {
    fn new(creator: String, collection: String, name: String) -> Self {
        Self {
            creator,
            collection,
            name,
        }
    }

    fn to_hash(&self) -> String {
        // Create a Sha256 object
        let mut hasher = Sha256::new();

        // Write input data
        hasher.update(format!(
            "{}::{}::{}",
            standardize_address(&self.creator),
            self.collection,
            self.name
        ));

        // Read hash digest and consume hasher
        let result = hasher.finalize();

        // Convert the result to a hexadecimal string
        format!("{:x}", result)
    }

    // fn get_collection_trunc(&self) -> String {
    //     truncate_str(&self.collection, MAX_NAME_LENGTH)
    // }

    // fn get_name_trunc(&self) -> String {
    //     truncate_str(&self.name, MAX_NAME_LENGTH)
    // }

    // fn get_collection_data_id_hash(&self) -> String {
    //     CollectionDataIdType::new(self.creator.clone(), self.collection.clone()).to_hash()
    // }

    // fn get_creator(&self) -> String {
    //     standardize_address(&self.creator)
    // }
}

// const MAX_NAME_LENGTH: usize = 128;

struct CollectionDataIdType {
    creator: String,
    name: String,
}

impl CollectionDataIdType {
    fn new(creator: String, name: String) -> Self {
        Self { creator, name }
    }

    fn to_hash(&self) -> String {
        let mut hasher = Sha256::new();

        hasher.update(format!(
            "{}::{}",
            standardize_address(&self.creator),
            self.name
        ));

        let result = hasher.finalize();
        format!("{:x}", result)
    }

    // fn get_name_trunc(&self) -> String {
    //     truncate_str(&self.name, MAX_NAME_LENGTH)
    // }

    // fn get_creator(&self) -> String {
    //     standardize_address(&self.creator)
    // }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TokenIdType {
    token_data_id: String,    // hash of token_data_id
    property_version: String, // String format of Decimal
}

impl TokenIdType {
    fn new(token_data_id: String, property_version: String) -> Self {
        Self {
            token_data_id,
            property_version,
        }
    }

    fn to_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("{}::{}", self.token_data_id, self.property_version));
        let result = hasher.finalize();
        format!("{:x}", result)
    }
}

impl fmt::Display for TokenIdType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", self.token_data_id, self.property_version)
    }
}
