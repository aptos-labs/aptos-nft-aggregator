// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::HashableJsonPath;
use ahash::AHashMap;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

pub type MarketplaceEventConfigMapping = AHashMap<String, MarketplaceEventConfig>;
pub type MarketplaceEventConfigMappings = AHashMap<String, MarketplaceEventConfigMapping>;
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
    pub listing_config: ListingConfig,
    pub offer_config: OfferConfig,
    pub collection_offer_config: CollectionOfferConfig,
}

impl NFTMarketplaceConfigs {
    pub fn get_event_mappings(
        &self,
    ) -> Result<(MarketplaceEventConfigMappings, ContractToMarketplaceMap)> {
        let mut marketplace_to_events_map = AHashMap::new();
        let mut contract_to_marketplace_map = AHashMap::new();
        for config in &self.marketplace_configs {
            let mut mapping: AHashMap<String, MarketplaceEventConfig> = AHashMap::new();
            mapping.insert(
                config.listing_config.place_event.clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::PlaceListing,
                    config.marketplace_name.clone(),
                    None,
                    None,
                    None,
                    config.listing_config.buyer.clone(),
                    config.listing_config.seller.clone(),
                )?,
            );
            mapping.insert(
                config.listing_config.cancel_event.clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::CancelListing,
                    config.marketplace_name.clone(),
                    None,
                    None,
                    None,
                    config.listing_config.buyer.clone(),
                    config.listing_config.seller.clone(),
                )?,
            );
            mapping.insert(
                config.listing_config.fill_event.clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::FillListing,
                    config.marketplace_name.clone(),
                    None,
                    None,
                    None,
                    config.listing_config.buyer.clone(),
                    config.listing_config.seller.clone(),
                )?,
            );
            mapping.insert(
                config.offer_config.place_event.clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::PlaceOffer,
                    config.marketplace_name.clone(),
                    None,
                    None,
                    None,
                    config.offer_config.buyer.clone(),
                    config.offer_config.seller.clone(),
                )?,
            );
            mapping.insert(
                config.offer_config.cancel_event.clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::CancelOffer,
                    config.marketplace_name.clone(),
                    None,
                    None,
                    None,
                    config.offer_config.buyer.clone(),
                    config.offer_config.seller.clone(),
                )?,
            );
            mapping.insert(
                config.offer_config.fill_event.clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::FillOffer,
                    config.marketplace_name.clone(),
                    None,
                    None,
                    None,
                    config.offer_config.buyer.clone(),
                    config.offer_config.seller.clone(),
                )?,
            );
            mapping.insert(
                config
                    .collection_offer_config
                    .place_event
                    .event_type
                    .clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::PlaceCollectionOffer,
                    config.marketplace_name.clone(),
                    config
                        .collection_offer_config
                        .place_event
                        .collection_name
                        .clone(),
                    config
                        .collection_offer_config
                        .place_event
                        .creator_address
                        .clone(),
                    config.collection_offer_config.deadline.clone(),
                    None,
                    None,
                )?,
            );
            mapping.insert(
                config
                    .collection_offer_config
                    .cancel_event
                    .event_type
                    .clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::CancelCollectionOffer,
                    config.marketplace_name.clone(),
                    config
                        .collection_offer_config
                        .cancel_event
                        .collection_name
                        .clone(),
                    config
                        .collection_offer_config
                        .cancel_event
                        .creator_address
                        .clone(),
                    config.collection_offer_config.deadline.clone(),
                    None,
                    None,
                )?,
            );
            mapping.insert(
                config.collection_offer_config.fill_event.event_type.clone(),
                MarketplaceEventConfig::from_event_config(
                    &config.event_config,
                    MarketplaceEventType::FillCollectionOffer,
                    config.marketplace_name.clone(),
                    config
                        .collection_offer_config
                        .fill_event
                        .collection_name
                        .clone(),
                    config
                        .collection_offer_config
                        .fill_event
                        .creator_address
                        .clone(),
                    config.collection_offer_config.deadline.clone(),
                    None,
                    None,
                )?,
            );
            for event in mapping.keys() {
                contract_to_marketplace_map.insert(event.clone(), config.marketplace_name.clone());
            }
            marketplace_to_events_map.insert(config.marketplace_name.clone(), mapping.clone());
        }

        Ok((marketplace_to_events_map, contract_to_marketplace_map))
    }
}

#[derive(Clone, Debug)]
pub struct MarketplaceEventConfig {
    pub event_type: MarketplaceEventType,
    pub marketplace: String,
    pub creator_address: HashableJsonPath,
    pub collection_id: HashableJsonPath,
    pub token_name: HashableJsonPath,
    pub price: HashableJsonPath,
    pub token_amount: HashableJsonPath,
    pub buyer: HashableJsonPath,
    pub seller: HashableJsonPath,
    pub collection_name: HashableJsonPath,
    pub deadline: HashableJsonPath,
    pub token_inner: HashableJsonPath,
    pub collection_inner: HashableJsonPath,
}

impl MarketplaceEventConfig {
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
            token_name: HashableJsonPath::new(event_config.token_name.clone())?,
            price: HashableJsonPath::new(event_config.price.clone())?,
            token_amount: HashableJsonPath::new(event_config.token_amount.clone())?,
            seller: HashableJsonPath::new(
                if seller.is_some() {
                    seller
                } else {
                    event_config.seller.clone()
                },
            )?,
            buyer: HashableJsonPath::new(
                if buyer.is_some() {
                    buyer
                } else {
                    event_config.buyer.clone()
                },
            )?,
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
            collection_id: HashableJsonPath::new(event_config.collection_id.clone())?,
            deadline: HashableJsonPath::new(
                if deadline.is_some() {
                    deadline
                } else {
                    event_config.deadline.clone()
                },
            )?,
            token_inner: HashableJsonPath::new(event_config.token_inner.clone())?,
            collection_inner: HashableJsonPath::new(event_config.collection_inner.clone())?,
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
    pub deadline: Option<String>, // across all marketplaces, deadline is part of the event data for collection offer
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
    pub token_inner: Option<String>,
    pub collection_inner: Option<String>,
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
