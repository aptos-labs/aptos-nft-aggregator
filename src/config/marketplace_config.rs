// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{models::nft_models::{CurrentNFTMarketplaceCollectionOffer, CurrentNFTMarketplaceListing, CurrentNFTMarketplaceTokenOffer, MarketplaceModel}, steps::HashableJsonPath};
use ahash::AHashMap;
use anyhow::Result;
use aptos_indexer_processor_sdk::{
    aptos_protos::transaction::v1::Event as EventPB, utils::convert::standardize_address,
};
use diesel::{
    deserialize::{self, FromSql, FromSqlRow},
    expression::AsExpression,
    pg::{Pg, PgValue},
    serialize::{self, Output, ToSql},
    sql_types::Text,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io::Write, str::FromStr};
use strum::{Display, EnumString};
use anyhow::{Context};
use chrono::NaiveDateTime;

pub type MarketplaceEventMappings = AHashMap<String, (String, MarketplaceEventType)>;
// marketplace_name -> TableMapping
pub type TableMappings = AHashMap<String, TableMapping>; // (table_name, column_name, source, resource_type, path, event_type)

pub type FieldRemappings = HashMap<EventType, HashMap<HashableJsonPath, Vec<RemappableColumn>>>;

// table_name -> column_name, source, resource_type, path, event_type
pub type TableMapping =
    AHashMap<String, Vec<(String, String, Option<String>, HashableJsonPath, String)>>;

/// Maximum length of a token name in characters
pub const MAX_TOKEN_NAME_LENGTH: usize = 128;
pub const ALL_EVENTS: &str = "all_events";
pub const EVENTS: &str = "events";
pub const POSTGRES_NUMERIC_DATA_TYPE: &str = "NUMERIC";

/// Top-level marketplace configurations
// #[derive(Clone, Debug, Deserialize, Serialize)]
// #[serde(deny_unknown_fields)]
// pub struct NFTMarketplaceConfig {
//     pub marketplaces: Vec<MarketplaceConfig>,
// }

pub type EventRemappingConfig = HashMap<String, EventRemapping>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DbColumn {
    pub table: String,
    pub column: String,
}

/// Represents a marketplace and its configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
// #[serde(deny_unknown_fields)]
pub struct NFTMarketplaceConfig {
    pub name: String,
    #[serde(default)]
    pub events: EventRemappingConfig,
    // pub resources: ResourceRemappingConfig,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct EventRemapping {
    pub event_fields: HashMap<String, Vec<DbColumn>>,
    // #[serde(default)]
    // pub event_metadata: EventMetadataRemappingConfig,
    // #[serde(default)]
    // pub constant_values: Vec<DbColumnConstant>,
}


// #[derive(Clone, Debug, Default, Deserialize, Serialize)]
// pub struct EventMetadataRemappingConfig {
//     sequence_number: Vec<DbColumn>,
//     creation_number: Vec<DbColumn>,
//     account_address: Vec<DbColumn>,
//     event_type: Vec<DbColumn>,
//     event_index: Vec<DbColumn>,
// }

// pub type EventMetadataRemapping = HashMap<EventMetadata, Vec<DbColumn>>;

// impl From<EventMetadataRemappingConfig> for EventMetadataRemapping {
//     fn from(value: EventMetadataRemappingConfig) -> Self {
//         let mut map = HashMap::new();
//         map.insert(EventMetadata::SequenceNumber, value.sequence_number);
//         map.insert(EventMetadata::CreationNumber, value.creation_number);
//         map.insert(EventMetadata::AccountAddress, value.account_address);
//         map.insert(EventMetadata::EventType, value.event_type);
//         map.insert(EventMetadata::EventIndex, value.event_index);
//         map
//     }
// }


// #[derive(Clone, Debug, Eq, PartialEq, Hash, Deserialize, Serialize, TS)]
// #[serde(rename_all = "snake_case")]
// pub enum EventMetadata {
//     SequenceNumber,
//     CreationNumber,
//     AccountAddress,
//     EventType,
//     EventIndex,
// }

// impl From<&EventMetadata> for ColumnType {
//     fn from(val: &EventMetadata) -> Self {
//         match val {
//             EventMetadata::SequenceNumber => ColumnType::custom(POSTGRES_NUMERIC_DATA_TYPE),
//             EventMetadata::CreationNumber => ColumnType::custom(POSTGRES_NUMERIC_DATA_TYPE),
//             EventMetadata::AccountAddress => ColumnType::String(StringLen::N(66)),
//             EventMetadata::EventType => ColumnType::Text,
//             EventMetadata::EventIndex => ColumnType::custom(POSTGRES_NUMERIC_DATA_TYPE),
//         }
//     }
// }

/// Represents a column in the database and its metadata
pub struct RemappableColumn {
    pub db_column: DbColumn,
    // column_metadata: ColumnMetadata,
    // move_type: MoveType,
}

impl RemappableColumn {
    pub fn new(db_column: DbColumn) -> Self {
        Self {
            db_column,
        }
    }
}

struct TableColumn {
    table: String,
    column: String,
}

// impl NFTMarketplaceConfig {
//     pub fn get_mappings(&self) -> Result<(MarketplaceEventMappings, TableMappings)> {
//         // let mut event_mappings = AHashMap::new();
//         // let mut table_mappings = AHashMap::new();
//         let mut field_remappings: FieldRemappings = HashMap::new();

//         let marketplace_name = self.name.clone();

//         for (event_type, event_remapping) in &self.events {
//             let event_type: EventType = event_type.as_str().try_into()?;
//             let mut db_mappings_for_event = HashMap::new();
    
//             for (json_path, db_mappings) in &event_remapping.event_fields {
//                 let json_path = HashableJsonPath::new(json_path)?;
//                 let db_mappings = db_mappings
//                     .iter()
//                     .map(|db_mapping| {
//                         // let (column_metadata, move_type) =
//                         //     get_metadata_and_move_type(db_schema, db_mapping)?;
//                         Ok(RemappableColumn::new(
//                             db_mapping.clone(),
//                         ))
//                     })
//                     .collect::<anyhow::Result<Vec<_>>>()?;

//             db_mappings_for_event.insert(json_path, db_mappings);
            
//             }

//             field_remappings.insert(event_type.clone(), db_mappings_for_event);
            
            
//             // TODO: we can revisit if we want to use this later
//             // Create metadata remapper for each event type
//             // metadata_remappers.insert(
//             //     event_type.clone(),
//             //     MetadataRemapper::new(event_remapping.event_metadata(), EventMetadataExtractor),
//             // );
//         }

//     }
// }

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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EventModel {
    pub sequence_number: i64,
    pub creation_number: i64,
    pub account_address: String,
    pub transaction_version: i64,
    pub transaction_block_height: i64,
    pub event_type: EventType,
    pub data: serde_json::Value,
    pub event_index: i64,
    pub block_timestamp: NaiveDateTime,
}

impl EventModel {
    /// This function can return an error if we unexpectedly fail to parse the event
    /// data in a recoverable way we shouldn't ignore, e.g. the event data is not valid
    /// JSON. It can return None if the event data is something we purposely don't
    /// handle, for example if the event type is a primitive like `address`.
    pub fn from_event(
        event: &EventPB,
        transaction_version: i64,
        transaction_block_height: i64,
        event_index: i64,
        block_timestamp: NaiveDateTime,
    ) -> Result<Option<Self>> {
        let t: &str = event.type_str.as_ref();
        let event_type = match EventType::try_from(t) {
            Ok(event_type) => event_type,
            Err(_) => {
                // It is fine to skip these events without logging because we explicitly
                // don't support primitive event types and don't let people configure
                // their processor to index them.
                return Ok(None);
            },
        };
        let event_key = event.key.as_ref().context("Event should have a key")?;
        
        Ok(Some(EventModel {
            account_address: standardize_address(event_key.account_address.as_str()),
            creation_number: event_key.creation_number as i64,
            sequence_number: event.sequence_number as i64,
            transaction_version,
            transaction_block_height,
            event_type,
            // We continue to panic here because we want to fail fast in this case
            // since the event data should _always_ be valid JSON.
            data: serde_json::from_str(event.data.as_str())
                .context("Event data should be valid JSON")?,
            event_index,
            block_timestamp,
        }))
    }

    /// If we fail to parse an event, we log and skip it. So this function can't fail.
    pub fn from_events(
        events: &[EventPB],
        transaction_version: i64,
        transaction_block_height: i64,
        block_timestamp: NaiveDateTime,
    ) -> Result<Vec<Self>> {
        let mut result = Vec::new();
        for (index, event) in events.iter().enumerate() {
            match Self::from_event(
                event,
                transaction_version,
                transaction_block_height,
                index as i64,
                block_timestamp,
            ) {
                Ok(Some(event_model)) => result.push(event_model),
                Ok(None) => continue,
                Err(e) => {
                    return Err(e.context(format!(
                        "Failed to parse event type {} at version {}",
                        event.type_str, transaction_version
                    )));
                },
            }
        }
        Ok(result)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum MarketplaceModelType {
    Listing,
    TokenOffer,
    CollectionOffer,
}

impl MarketplaceModelType {
    fn from_event_type(event_type: &str) -> Option<Self> {
        match event_type {
            "ListingPlacedEvent" | "ListingCanceledEvent" | "ListingFilledEvent" => {
                Some(MarketplaceModelType::Listing)
            },
            "TokenOfferPlacedEvent" | "TokenOfferCanceledEvent" | "TokenOfferFilledEvent" => {
                Some(MarketplaceModelType::TokenOffer)
            },
            "CollectionOfferPlacedEvent" | "CollectionOfferCanceledEvent" | "CollectionOfferFilledEvent" => {
                Some(MarketplaceModelType::CollectionOffer)
            },
            _ => None,
        }
    }

    fn create_model(&self) -> Option<Box<dyn MarketplaceModel>> {
        match self {
            MarketplaceModelType::Listing => Some(Box::new(CurrentNFTMarketplaceListing::default())),
            MarketplaceModelType::TokenOffer => Some(Box::new(CurrentNFTMarketplaceTokenOffer::default())),
            MarketplaceModelType::CollectionOffer => Some(Box::new(CurrentNFTMarketplaceCollectionOffer::default())),
        }
    }
}