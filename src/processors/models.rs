use super::{
    extract_bigdecimal, extract_string, marketplace_config::MarketplaceEventConfigMapping,
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

#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(transaction_version, index))]
#[diesel(table_name = nft_marketplace_activities)]
pub struct NftMarketplaceActivity {
    pub transaction_version: i64,
    pub index: i64,
    pub raw_event_type: String,
    pub standard_event_type: String,
    pub creator_address: Option<String>,
    pub collection_id: Option<String>,
    pub collection_name: Option<String>,
    pub token_data_id: Option<String>,
    pub token_id: Option<String>,
    pub token_name: Option<String>,
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
        index: i64,
        timestamp: NaiveDateTime,
        entry_function_id_str: &Option<String>,
        event_mapping: &MarketplaceEventConfigMapping,
    ) -> Option<Self> {
        let event_type = event.type_str.to_string();

        let event_data: Value = serde_json::from_str(event.data.as_str()).unwrap();

        if let Some(config) = event_mapping.get(&event_type) {
            let standard_event_type = config.event_type.as_str().to_string();
            let creator_address = standardize_address(&extract_string(&config.creator_address, &event_data).unwrap());
            let collection_id = standardize_address(&extract_string(&config.collection_id, &event_data).unwrap());
            let token_name = extract_string(&config.token_name, &event_data).unwrap();
            let price = extract_bigdecimal(&config.price, &event_data).unwrap();
            let token_amount = extract_bigdecimal(&config.token_amount, &event_data).unwrap();
            let buyer = standardize_address(&extract_string(&config.buyer, &event_data).unwrap());

            let activity = Self {
                transaction_version: txn_version,
                index,
                raw_event_type: event_type.clone(),
                standard_event_type,
                creator_address: Some(creator_address),
                collection_id: Some(collection_id),
                collection_name: None, // Not available in event config
                token_data_id: None,
                token_id: None, // Not available in event config
                token_name: Some(token_name),
                price: Some(price),
                token_amount: Some(token_amount),
                buyer: Some(buyer),
                seller: None, // Not available in event config
                json_data: event_data,
                marketplace: "marketplace".to_string(), // TODO: Get from config
                contract_address: "contract".to_string(), // TODO: Get from config
                entry_function_id_str: entry_function_id_str.clone(),
                transaction_timestamp: timestamp,
            };
            Some(activity)
        } else {
            None
        }
    }
}
