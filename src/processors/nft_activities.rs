use serde::{Deserialize, Serialize};

// use crate::schema::nft_activities;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NftActivity {
    pub account_address: String,
    pub transaction_version: i64,
}
