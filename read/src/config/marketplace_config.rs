// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::steps::HashableJsonPath;
use anyhow::Result;
use aptos_indexer_processor_sdk::utils::convert::standardize_address;
use diesel::{
    deserialize::{self, FromSql, FromSqlRow},
    expression::AsExpression,
    pg::{Pg, PgValue},
    serialize::{self, Output, ToSql},
    sql_types::Text,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io::Write};
use strum::{Display, EnumString};

// event_type -> json_path, db_column
pub type EventFieldRemappings = HashMap<EventType, HashMap<HashableJsonPath, Vec<DbColumn>>>;
// resource_type -> json_path, db_column
pub type ResourceFieldRemappings = HashMap<String, HashMap<HashableJsonPath, Vec<DbColumn>>>;

/// Maximum length of a token name in characters
pub const MAX_TOKEN_NAME_LENGTH: usize = 128;

pub type EventRemappingConfig = HashMap<String, EventRemapping>;
pub type ResourceRemappingConfig = HashMap<String, ResourceRemapping>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DbColumn {
    pub table: String,
    pub column: String,
}

/// Represents a marketplace and its configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NFTMarketplaceConfig {
    pub name: String,
    /// Maps event type strings to their corresponding MarketplaceEventType enum values.
    /// This mapping is used to standardize different marketplace event types across
    /// different NFT marketplaces into a standarzied event types for processing.
    /// For example, a "ListNFT" event from one marketplace might map to PlaceListing
    /// while another marketplace's "CreateListing" event would also map to PlaceListing.
    #[serde(default)]
    pub event_model_mapping: HashMap<String, MarketplaceEventType>,
    #[serde(default)]
    pub events: EventRemappingConfig,
    #[serde(default)]
    pub resources: ResourceRemappingConfig,
}

impl NFTMarketplaceConfig {
    /// Returns the name of the marketplace.
    pub fn get_name(&self) -> &'static str {
        // Intentionally leak the string to satisfy &'static str requirement
        // This is acceptable since processor names live for the application lifetime
        Box::leak(self.name.clone().into_boxed_str())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct EventRemapping {
    pub event_fields: HashMap<String, Vec<DbColumn>>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ResourceRemapping {
    pub resource_fields: HashMap<String, Vec<DbColumn>>,
}

#[derive(
    Debug,
    Clone,
    Default,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    AsExpression,
    FromSqlRow,
    EnumString,
    Display,
    Hash,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[diesel(sql_type = Text)]
pub enum MarketplaceEventType {
    // Listing events
    PlaceListing,
    CancelListing,
    FillListing,
    // Token offer events
    PlaceTokenOffer,
    CancelTokenOffer,
    FillTokenOffer,
    // Collection offer events
    PlaceCollectionOffer,
    CancelCollectionOffer,
    FillCollectionOffer,
    #[default]
    Unknown,
}

impl ToSql<Text, Pg> for MarketplaceEventType {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_all(self.to_string().as_bytes())?;
        Ok(serialize::IsNull::No)
    }
}

impl FromSql<Text, Pg> for MarketplaceEventType {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let s = std::str::from_utf8(bytes.as_bytes())?;
        s.parse::<MarketplaceEventType>()
            .map_err(|_| "Unrecognized MarketplaceEventType".into())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventType {
    address: String,
    module: String,
    r#struct: String,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}::{}::{}", self.address, self.module, self.r#struct)
    }
}

impl TryFrom<&str> for EventType {
    type Error = anyhow::Error;

    fn try_from(event_type: &str) -> Result<Self> {
        let parts: Vec<&str> = event_type.split("::").collect();
        if parts.len() < 3 {
            // With v1 events it is possible to emit primitives as events, e.g. just
            // emit an address or u64 as an event. We don't support this.
            anyhow::bail!("Unsupported event type: {}", event_type);
        }

        Ok(EventType {
            address: standardize_address(parts[0]),
            module: parts[1].to_string(),
            r#struct: parts[2..].join("::"), // Don't need to standardize generics because we won't support them
        })
    }
}

impl EventType {
    /// Returns true if the event type is a framework event. We don't always allow
    /// users to index framework events.
    //
    // WARNING: This code is only safe because we `standardize_address` in the
    // `try_from` implementation. If we add another way to instantiate an `EventType`,
    // it must also do this conversion.
    //
    // TODO: If we ever get a better Rust SDK, use AccountAddress instead.
    pub fn is_framework_event(&self) -> bool {
        // Convert address string to bytes. Skip "0x" prefix.
        let addr_bytes = hex::decode(&self.address[2..]).unwrap();

        // This is taken from AccountAddress::is_special.
        addr_bytes[..32 - 1].iter().all(|x| *x == 0) && addr_bytes[32 - 1] < 0b10000
    }

    pub fn get_struct(&self) -> &str {
        &self.r#struct
    }
}
