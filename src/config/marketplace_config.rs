// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::steps::HashableJsonPaths;
use ahash::AHashMap;
use anyhow::Result;
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
// use strum_macros::{EnumString, Display};

pub type MarketplaceEventMappings = AHashMap<String, (String, MarketplaceEventType)>;
// marketplace_name -> TableMapping
pub type TableMappings = AHashMap<String, TableMapping>; // (table_name, column_name, source, resource_type, path, event_type)

// table_name -> column_name, source, resource_type, path, event_type
pub type TableMapping =
    AHashMap<String, Vec<(String, String, Option<String>, HashableJsonPaths, String)>>;

/// Maximum length of a token name in characters
pub const MAX_TOKEN_NAME_LENGTH: usize = 128;
pub const ALL_EVENTS: &str = "all_events";
pub const EVENTS: &str = "events";

/// Top-level marketplace configurations
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NFTMarketplaceConfigs {
    pub marketplaces: Vec<MarketplaceConfig>,
}

/// Represents a marketplace and its configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MarketplaceConfig {
    pub name: String,
    pub event_types: Vec<EventType>,
    pub tables: HashMap<String, TableConfig>,
}

pub struct ResourceType {
    pub r#type: String,
}

pub struct ResourceField {
    pub field_name: String,
    pub path: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EventType {
    pub r#type: String, // "listing", "token_offer", "collection_offer"
    pub cancel: String,
    pub fill: String,
    pub place: String,
}

/// Represents configuration for tables within a marketplace
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TableConfig {
    pub columns: HashMap<String, ColumnConfig>,
}

/// Defines how a column should be extracted from an event or resource
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ColumnConfig {
    pub source: Option<String>,        // "write_set_changes"
    pub resource_type: Option<String>, // Only for "write_set_changes"
    pub path: Vec<String>,             // JSON path for extraction
    pub event_type: Option<String>, // The event type that requires write_set_changes if not provided, we will use all events
}

impl NFTMarketplaceConfigs {
    pub fn get_mappings(&self) -> Result<(MarketplaceEventMappings, TableMappings)> {
        let mut event_mappings = AHashMap::new();
        let mut table_mappings = AHashMap::new();

        for marketplace in &self.marketplaces {
            let marketplace_name = marketplace.name.clone();

            for event_type in &marketplace.event_types {
                event_mappings.insert(
                    event_type.place.clone(),
                    (
                        marketplace_name.clone(),
                        MarketplaceEventType::from_str(&format!("place_{}", event_type.r#type))
                            .unwrap(),
                    ),
                );
                event_mappings.insert(
                    event_type.cancel.clone(),
                    (
                        marketplace_name.clone(),
                        MarketplaceEventType::from_str(&format!("cancel_{}", event_type.r#type))
                            .unwrap(),
                    ),
                );
                event_mappings.insert(
                    event_type.fill.clone(),
                    (
                        marketplace_name.clone(),
                        MarketplaceEventType::from_str(&format!("fill_{}", event_type.r#type))
                            .unwrap(),
                    ),
                );
            }

            let mut table_mapping: TableMapping = AHashMap::new();

            for (table_name, table_config) in &marketplace.tables {
                let entries = table_mapping.entry(table_name.clone()).or_default();
                for (column_name, column_config) in &table_config.columns {
                    entries.push((
                        column_name.clone(),
                        column_config.source.clone().unwrap_or(EVENTS.to_string()),
                        column_config.resource_type.clone(),
                        HashableJsonPaths::new(column_config.path.clone())?,
                        column_config
                            .event_type
                            .clone()
                            .unwrap_or(ALL_EVENTS.to_string()),
                    ));
                }
            }
            if !table_mapping.is_empty() {
                table_mappings.insert(marketplace.name.clone(), table_mapping);
            }
        }

        Ok((event_mappings, table_mappings))
    }
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
    // Direct offer events
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
