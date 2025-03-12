// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::steps::{extract_string, HashableJsonPath};
use ahash::{AHashMap, HashMap, HashMapExt};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

pub type MarketplaceEventConfigMapping = AHashMap<String, MarketplaceEventConfig>;
pub type MarketplaceEventConfigMappings = AHashMap<String, MarketplaceEventConfigMapping>;
pub type MarketplaceResourceConfigMapping = AHashMap<String, MarketplaceResourceConfig>;
pub type MarketplaceResourceConfigMappings = AHashMap<String, MarketplaceResourceConfigMapping>;
pub type ContractToMarketplaceMap = AHashMap<String, String>;

/// Maximum length of a token name in characters
pub const MAX_TOKEN_NAME_LENGTH: usize = 128;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NFTMarketplaceConfigs {
    pub marketplace_configs: Vec<MarketplaceConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MarketplaceConfig {
    pub marketplace_name: String,
    pub event_config: EventConfig,
    #[serde(default)]
    pub resource_config: ResourceConfig,
}

impl NFTMarketplaceConfigs {
    pub fn get_mappings(
        &self,
    ) -> Result<(
        MarketplaceEventConfigMappings,
        MarketplaceResourceConfigMappings,
        ContractToMarketplaceMap,
    )> {
        let mut marketplace_to_events_map = AHashMap::new();
        let mut marketplace_to_resources_map = AHashMap::new();
        let mut contract_to_marketplace_map = AHashMap::new();

        for config in &self.marketplace_configs {
            let mut event_mapping: AHashMap<String, MarketplaceEventConfig> = AHashMap::new();
            event_mapping.insert(
                config.event_config.listing_config.place_event.clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::PlaceListing,
                    config.marketplace_name.clone(),
                    None,
                    None,
                    None,
                    config.event_config.listing_config.buyer.clone(),
                    config.event_config.listing_config.seller.clone(),
                )?,
            );
            event_mapping.insert(
                config.event_config.listing_config.cancel_event.clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::CancelListing,
                    config.marketplace_name.clone(),
                    None,
                    None,
                    None,
                    config.event_config.listing_config.buyer.clone(),
                    config.event_config.listing_config.seller.clone(),
                )?,
            );
            event_mapping.insert(
                config.event_config.listing_config.fill_event.clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::FillListing,
                    config.marketplace_name.clone(),
                    None,
                    None,
                    None,
                    config.event_config.listing_config.buyer.clone(),
                    config.event_config.listing_config.seller.clone(),
                )?,
            );
            event_mapping.insert(
                config.event_config.offer_config.place_event.clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::PlaceOffer,
                    config.marketplace_name.clone(),
                    None,
                    None,
                    None,
                    config.event_config.offer_config.buyer.clone(),
                    config.event_config.offer_config.seller.clone(),
                )?,
            );
            event_mapping.insert(
                config.event_config.offer_config.cancel_event.clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::CancelOffer,
                    config.marketplace_name.clone(),
                    None,
                    None,
                    None,
                    config.event_config.offer_config.buyer.clone(),
                    config.event_config.offer_config.seller.clone(),
                )?,
            );
            event_mapping.insert(
                config.event_config.offer_config.fill_event.clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::FillOffer,
                    config.marketplace_name.clone(),
                    None,
                    None,
                    None,
                    config.event_config.offer_config.buyer.clone(),
                    config.event_config.offer_config.seller.clone(),
                )?,
            );
            event_mapping.insert(
                config
                    .event_config
                    .collection_offer_config
                    .place_event
                    .event_type
                    .clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::PlaceCollectionOffer,
                    config.marketplace_name.clone(),
                    config
                        .event_config
                        .collection_offer_config
                        .place_event
                        .collection_name
                        .clone(),
                    config
                        .event_config
                        .collection_offer_config
                        .place_event
                        .creator_address
                        .clone(),
                    config.event_config.deadline.clone(),
                    None,
                    None,
                )?,
            );
            event_mapping.insert(
                config
                    .event_config
                    .collection_offer_config
                    .cancel_event
                    .event_type
                    .clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::CancelCollectionOffer,
                    config.marketplace_name.clone(),
                    config
                        .event_config
                        .collection_offer_config
                        .cancel_event
                        .collection_name
                        .clone(),
                    config
                        .event_config
                        .collection_offer_config
                        .cancel_event
                        .creator_address
                        .clone(),
                    config.event_config.deadline.clone(),
                    None,
                    None,
                )?,
            );
            event_mapping.insert(
                config
                    .event_config
                    .collection_offer_config
                    .fill_event
                    .event_type
                    .clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::FillCollectionOffer,
                    config.marketplace_name.clone(),
                    config
                        .event_config
                        .collection_offer_config
                        .fill_event
                        .collection_name
                        .clone(),
                    config
                        .event_config
                        .collection_offer_config
                        .fill_event
                        .creator_address
                        .clone(),
                    config.event_config.deadline.clone(),
                    None,
                    None,
                )?,
            );

            let mut resource_mapping: AHashMap<String, MarketplaceResourceConfig> = AHashMap::new();

            for resource_type in &config.resource_config.resource_types {
                resource_mapping.insert(
                    resource_type.resource_type.clone(),
                    MarketplaceResourceConfig::from_resource_config(&config.resource_config)?,
                );
            }

            let marketplace_name = config.marketplace_name.clone();

            for event in event_mapping.keys() {
                contract_to_marketplace_map.insert(event.clone(), marketplace_name.clone());
            }

            // Process resources - just extract the contract address once
            if let Some(first_resource) = config.resource_config.resource_types.first() {
                if let Some(contract_address) = first_resource.resource_type.split("::").next() {
                    contract_to_marketplace_map
                        .insert(contract_address.to_string(), marketplace_name.clone());
                }
            }
            marketplace_to_events_map.insert(marketplace_name.clone(), event_mapping);
            marketplace_to_resources_map.insert(marketplace_name.clone(), resource_mapping);
        }

        Ok((
            marketplace_to_events_map,
            marketplace_to_resources_map,
            contract_to_marketplace_map,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct MarketplaceEventConfig {
    pub event_type: MarketplaceEventType,
    pub marketplace: String,
    pub collection_id: HashableJsonPath,
    pub creator_address: HashableJsonPath,
    pub token_name: HashableJsonPath,
    pub collection_name: HashableJsonPath,
    pub price: HashableJsonPath,
    pub token_amount: HashableJsonPath,
    pub buyer: HashableJsonPath,
    pub seller: HashableJsonPath,
    pub deadline: HashableJsonPath,
    pub token_inner: HashableJsonPath,
    pub collection_inner: HashableJsonPath,
    pub listing_id: HashableJsonPath,
    pub offer_id: HashableJsonPath,
    pub collection_offer_id: HashableJsonPath,
}

impl MarketplaceEventConfig {
    /**
     * Some of fields are optional, it's due to the fact that the event config is shared across different event types
     * and some of the fields are not applicable to all event types
     */
    pub fn from_event_config(
        event_config: &EventConfig,
        event_type: MarketplaceEventType,
        marketplace_name: String,
        collection_name: Option<String>,
        creator_address: Option<String>,
        deadline: Option<String>,
        buyer: Option<String>,
        seller: Option<String>,
    ) -> Result<Self> {
        Ok(Self {
            event_type,
            marketplace: marketplace_name,
            collection_id: HashableJsonPath::new(event_config.collection_id.clone())?,
            creator_address: HashableJsonPath::new(
                creator_address.or_else(|| event_config.creator_address.clone()),
            )?,
            token_name: HashableJsonPath::new(event_config.token_name.clone())?,
            collection_name: HashableJsonPath::new(
                collection_name.or_else(|| event_config.collection_name.clone()),
            )?,
            price: HashableJsonPath::new(event_config.price.clone())?,
            token_amount: HashableJsonPath::new(event_config.token_amount.clone())?,
            buyer: HashableJsonPath::new(buyer.or_else(|| event_config.buyer.clone()))?,
            seller: HashableJsonPath::new(seller.or_else(|| event_config.seller.clone()))?,
            deadline: HashableJsonPath::new(deadline.or_else(|| event_config.deadline.clone()))?,
            token_inner: HashableJsonPath::new(event_config.token_inner.clone())?,
            collection_inner: HashableJsonPath::new(event_config.collection_inner.clone())?,
            listing_id: HashableJsonPath::new(event_config.listing_id.clone())?,
            offer_id: HashableJsonPath::new(event_config.offer_id.clone())?,
            collection_offer_id: HashableJsonPath::new(event_config.collection_offer_id.clone())?,
        })
    }
}

/**
 *
 * As you can see, some of the fields are optional and duplicated with the event config
 * The reason for this is that each event type has different event structure and contains different data, which makes us
 * need to handle it differently for different marketplaces and for different event types. This applies to other Event types as well such
 * as offer(bid) and collection offer(bid).
 */
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ListingConfig {
    pub cancel_event: String,
    pub fill_event: String,
    pub place_event: String,
    pub collection_name: Option<String>,
    pub buyer: Option<String>,
    pub seller: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OfferConfig {
    pub cancel_event: String,
    pub fill_event: String,
    pub place_event: String,
    pub buyer: Option<String>,
    pub seller: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionOfferConfig {
    pub cancel_event: CollectionEventParams,
    pub fill_event: CollectionEventParams,
    pub place_event: CollectionEventParams,
}

/// This is to give us more flexibility to handle different event structures
/// for collection offer events across different marketplaces
/// Because even within the same marketplace, the event structure might be different for different type of events
/// e.g. for tradeport, the collection name and creator address are part of the token data for Fill Event, while they are part of collection metadata for Place Event and Cancel Event
/// but for topaz, the collection name and creator address are part of the top level event data for place event, but part of the token data for cancel and fill events
/// So we need to handle it differently for different marketplaces
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionEventParams {
    pub event_type: String,
    pub collection_name: Option<String>,
    pub creator_address: Option<String>,
}

/**
 * This is the config for us to extract the data from the event for each marketplace
 * Some of fields are optional, it's due to the fact that the event config is shared across different event types
 * and some of the fields are not applicable to all event types
 * For example, Only Topaz has deadline for collection offer event, So we need to use Optional.
*/
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EventConfig {
    pub collection_id: Option<String>,
    pub token_name: Option<String>,
    pub creator_address: Option<String>,
    pub collection_name: Option<String>,
    pub price: Option<String>,
    pub token_amount: Option<String>,
    pub buyer: Option<String>,
    pub seller: Option<String>,
    pub deadline: Option<String>,
    pub token_inner: Option<String>,
    pub collection_inner: Option<String>,
    pub offer_id: Option<String>,
    pub listing_id: Option<String>,
    pub collection_offer_id: Option<String>,
    // Specific configs for different event types
    pub listing_config: ListingConfig,
    pub offer_config: OfferConfig,
    pub collection_offer_config: CollectionOfferConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum MarketplaceEventType {
    // Listing events
    PlaceListing,
    CancelListing,
    FillListing,
    // Direct offer events
    PlaceOffer,
    CancelOffer,
    FillOffer,
    // Collection offer events
    PlaceCollectionOffer,
    CancelCollectionOffer,
    FillCollectionOffer,
}

impl MarketplaceEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PlaceListing => "place_listing",
            Self::CancelListing => "cancel_listing",
            Self::FillListing => "fill_listing",
            Self::PlaceOffer => "place_offer",
            Self::CancelOffer => "cancel_offer",
            Self::FillOffer => "fill_offer",
            Self::PlaceCollectionOffer => "place_collection_offer",
            Self::CancelCollectionOffer => "cancel_collection_offer",
            Self::FillCollectionOffer => "fill_collection_offer",
        }
    }
}

impl FromStr for MarketplaceEventType {
    type Err = ();

    fn from_str(input: &str) -> Result<MarketplaceEventType, Self::Err> {
        match input {
            "place_listing" => Ok(MarketplaceEventType::PlaceListing),
            "cancel_listing" => Ok(MarketplaceEventType::CancelListing),
            "fill_listing" => Ok(MarketplaceEventType::FillListing),
            "place_offer" => Ok(MarketplaceEventType::PlaceOffer),
            "cancel_offer" => Ok(MarketplaceEventType::CancelOffer),
            "fill_offer" => Ok(MarketplaceEventType::FillOffer),
            "place_collection_offer" => Ok(MarketplaceEventType::PlaceCollectionOffer),
            "cancel_collection_offer" => Ok(MarketplaceEventType::CancelCollectionOffer),
            "fill_collection_offer" => Ok(MarketplaceEventType::FillCollectionOffer),
            _ => Err(()),
        }
    }
}

impl fmt::Display for MarketplaceEventType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Clone, Debug)]
pub struct MarketplaceResourceConfig {
    pub fields: Vec<(String, HashableJsonPath, bool)>, // (field_name, json_path, required)
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceConfig {
    pub resource_types: Vec<ResourceTypeConfig>,
    pub fields: Vec<ResourceFieldConfig>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceFieldConfig {
    pub field_name: String, // Name of the field (e.g., "collection_id", "token_name", etc.)
    pub primary_path: Option<String>, // Primary JSON path
    #[serde(default)]
    pub required: bool, // Whether this field is required
}

// Dynamic result structure using HashMap
pub struct ExtractedResourceData {
    // Map of field names to their extracted values
    pub fields: HashMap<String, Option<String>>,
}

impl ExtractedResourceData {
    // Convenience methods to get common fields
    pub fn get_field(&self, field_name: &str) -> Option<&String> {
        self.fields.get(field_name).and_then(|opt| opt.as_ref())
    }

    pub fn collection_id(&self) -> Option<&String> {
        self.get_field("collection_id")
    }

    pub fn token_name(&self) -> Option<&String> {
        self.get_field("token_name")
    }
}

impl MarketplaceResourceConfig {
    pub fn from_resource_config(resource_config: &ResourceConfig) -> Result<Self> {
        let mut fields = Vec::new();

        // Process all fields from config
        for field_config in &resource_config.fields {
            let json_path = HashableJsonPath::new(field_config.primary_path.clone())?;

            // Add field to the list
            fields.push((
                field_config.field_name.clone(),
                json_path,
                field_config.required,
            ));
        }

        Ok(Self { fields })
    }

    pub fn extract_resource_data(&self, data: &serde_json::Value) -> Result<ExtractedResourceData> {
        let mut result = ExtractedResourceData {
            fields: HashMap::new(),
        };

        // Extract all configured fields
        for (field_name, json_path, required) in &self.fields {
            let value = extract_string(json_path, data);
            // If field is required but missing, return error
            if *required && value.is_none() {
                return Err(anyhow::anyhow!("Required field '{}' not found", field_name));
            }

            result.fields.insert(field_name.clone(), value);
        }

        Ok(result)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceTypeConfig {
    pub resource_type: String,
    // pub resource_action: ResourceType,
}
