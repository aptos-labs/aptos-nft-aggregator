use crate::schema::nft_marketplace_activities;
use diesel::prelude::*;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

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
    pub price: Option<bigdecimal::BigDecimal>,
    pub token_amount: Option<bigdecimal::BigDecimal>,
    pub buyer: Option<String>,
    pub seller: Option<String>,
    pub json_data: serde_json::Value,
    pub marketplace: String,
    pub contract_address: String,
    pub entry_function_id_str: String,
    pub transaction_timestamp: chrono::NaiveDateTime,
}
