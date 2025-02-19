// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::HashableJsonPath;
use ahash::AHashMap;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

pub type MarketplaceEventConfigMapping = AHashMap<String, MarketplaceEventConfig>;
pub type MarketplaceEventConfigMappings = AHashMap<String, MarketplaceEventConfigMapping>;
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
    pub contract_address: String,
    pub event_config: EventConfig,
    pub listing_config: ListingConfig,
    pub offer_config: OfferConfig,
    pub collection_offer_config: CollectionOfferConfig,
}

impl NFTMarketplaceConfigs {
    pub fn get_event_mappings(&self) -> Result<MarketplaceEventConfigMappings> {
        let mut marketplace_to_events_map = AHashMap::new();

        for config in &self.marketplace_configs {
            let mut mapping = AHashMap::new();
            mapping.insert(
                config.listing_config.place_event.clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::PlaceListing,
                    config.marketplace_name.clone(),
                    config.contract_address.clone(),
                    None,
                    None,
                    None,
                )?,
            );
            mapping.insert(
                config.listing_config.cancel_event.clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::CancelListing,
                    config.marketplace_name.clone(),
                    config.contract_address.clone(),
                    None,
                    None,
                    None,
                )?,
            );
            mapping.insert(
                config.listing_config.fill_event.clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::FillListing,
                    config.marketplace_name.clone(),
                    config.contract_address.clone(),
                    None,
                    None,
                    None,
                )?,
            );
            mapping.insert(
                config.offer_config.place_event.clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::PlaceOffer,
                    config.marketplace_name.clone(),
                    config.contract_address.clone(),
                    None,
                    None,
                    None,
                )?,
            );
            mapping.insert(
                config.offer_config.cancel_event.clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::CancelOffer,
                    config.marketplace_name.clone(),
                    config.contract_address.clone(),
                    None,
                    None,
                    None,
                )?,
            );
            mapping.insert(
                config.offer_config.fill_event.clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::FillOffer,
                    config.marketplace_name.clone(),
                    config.contract_address.clone(),
                    None,
                    None,
                    None,
                )?,
            );
            mapping.insert(
                config.collection_offer_config.place_event.clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::PlaceCollectionOffer,
                    config.marketplace_name.clone(),
                    config.contract_address.clone(),
                    config.collection_offer_config.collection_name.clone(),
                    config.collection_offer_config.creator_address.clone(),
                    config.collection_offer_config.deadline.clone(),
                )?,
            );
            mapping.insert(
                config.collection_offer_config.cancel_event.clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::CancelCollectionOffer,
                    config.marketplace_name.clone(),
                    config.contract_address.clone(),
                    config.collection_offer_config.collection_name.clone(),
                    config.collection_offer_config.creator_address.clone(),
                    config.collection_offer_config.deadline.clone(),
                )?,
            );
            mapping.insert(
                config.collection_offer_config.fill_event.clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::FillCollectionOffer,
                    config.marketplace_name.clone(),
                    config.contract_address.clone(),
                    config.collection_offer_config.collection_name.clone(),
                    config.collection_offer_config.creator_address.clone(),
                    config.collection_offer_config.deadline.clone(),
                )?,
            );
            
            marketplace_to_events_map.insert(config.contract_address.clone(), mapping);
        }
        Ok(marketplace_to_events_map)
    }
}

#[derive(Clone, Debug)]
pub struct MarketplaceEventConfig {
    pub event_type: MarketplaceEventType,
    pub marketplace: String,
    pub contract_address: String,
    pub creator_address: HashableJsonPath,
    pub collection_id: HashableJsonPath,
    pub token_name: HashableJsonPath,
    pub price: HashableJsonPath,
    pub token_amount: HashableJsonPath,
    pub buyer: HashableJsonPath,
    pub seller: HashableJsonPath,
    pub collection_name: HashableJsonPath,
    pub deadline: HashableJsonPath,
}

impl MarketplaceEventConfig {
    pub fn from_event_config(
        event_config: &EventConfig,
        event_type: MarketplaceEventType,
        marketplace_name: String,
        contract_address: String,
        collection_name: Option<String>,
        creator_address: Option<String>,
        deadline: Option<String>,
    ) -> Result<Self> {
        Ok(Self {
            event_type,
            marketplace: marketplace_name,
            contract_address,
            token_name: HashableJsonPath::new(event_config.token_name.clone())?,
            price: HashableJsonPath::new(event_config.price.clone())?,
            token_amount: HashableJsonPath::new(
                event_config.token_amount.clone()
            )?,
            seller: HashableJsonPath::new(event_config.seller.clone())?,
            buyer: HashableJsonPath::new(event_config.buyer.clone())?,
            collection_name: HashableJsonPath::new(
                if collection_name.is_some() {
                    collection_name
                } else {
                    event_config.collection_name.clone()
                },
            )?,
            creator_address: HashableJsonPath::new(
                if creator_address.is_some() {
                    creator_address
                } else {
                    event_config.creator_address.clone()
                },
            )?,
            collection_id: HashableJsonPath::new(
                event_config.collection_id.clone(),
            )?,
            deadline: HashableJsonPath::new(
                if deadline.is_some() {
                    deadline
                } else {
                    event_config.deadline.clone()
                },
            )?,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ListingConfig {
    pub cancel_event: String,
    pub fill_event: String,
    pub place_event: String,
    pub collection_name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OfferConfig {
    pub cancel_event: String,
    pub fill_event: String,
    pub place_event: String,
    // pub buyer: String,
    // pub seller: String,
    // pub price: String,
    // TODO: add more fields for struct
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionOfferConfig {
    pub cancel_event: String,
    pub fill_event: String,
    pub place_event: String,
    pub collection_name: Option<String>,
    pub creator_address: Option<String>,
    pub deadline: Option<String>,
    // TODO: add more fields for struct
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EventConfig {
    pub creator_address: Option<String>,
    pub collection_id: Option<String>,
    pub token_data_id: Option<String>,
    pub token_name: Option<String>,
    pub price: Option<String>,
    pub token_amount: Option<String>,
    pub buyer: Option<String>,
    pub seller: Option<String>,
    pub collection_name: Option<String>,
    pub deadline: Option<String>,
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
