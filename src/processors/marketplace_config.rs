// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::HashableJsonPath;
use ahash::AHashMap;
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub type MarketplaceEventConfigMapping = AHashMap<String, MarketplaceEventConfig>;
/// Maximum length of a token name in characters
pub const MAX_TOKEN_NAME_LENGTH: usize = 128;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NFTMarketplaceConfig {
    pub marketplace_name: String,
    pub contract_address: String,
    pub event_config: EventConfig,
    pub listing: ListingConfig,
    // pub offer: OfferConfig,
    // pub collection_offer: CollectionOfferConfig,
}

impl NFTMarketplaceConfig {
    pub fn get_event_mapping(&self) -> Result<MarketplaceEventConfigMapping> {
        let mut mapping = AHashMap::new();

        mapping.insert(
            self.listing.place_event.clone(),
            MarketplaceEventConfig::from_event_config(
                &self.event_config,
                MarketplaceEventType::PlaceListing,
            )?,
        );
        mapping.insert(
            self.listing.cancel_event.clone(),
            MarketplaceEventConfig::from_event_config(
                &self.event_config,
                MarketplaceEventType::CancelListing,
            )?,
        );
        mapping.insert(
            self.listing.fill_event.clone(),
            MarketplaceEventConfig::from_event_config(
                &self.event_config,
                MarketplaceEventType::FillListing,
            )?,
        );
        // mapping.insert(
        //     self.offer.place_event.clone(),
        //     MarketplaceEventConfig::from_event_config(
        //         &self.event_config,
        //         MarketplaceEventType::PlaceOffer,
        //     )?,
        // );
        // mapping.insert(
        //     self.offer.cancel_event.clone(),
        //     MarketplaceEventConfig::from_event_config(
        //         &self.event_config,
        //         MarketplaceEventType::CancelOffer,
        //     )?,
        // );
        // mapping.insert(
        //     self.offer.fill_event.clone(),
        //     MarketplaceEventConfig::from_event_config(
        //         &self.event_config,
        //         MarketplaceEventType::FillOffer,
        //     )?,
        // );
        // mapping.insert(
        //     self.collection_offer.place_event.clone(),
        //     MarketplaceEventConfig::from_event_config(
        //         &self.event_config,
        //         MarketplaceEventType::PlaceCollectionOffer,
        //     )?,
        // );
        // mapping.insert(
        //     self.collection_offer.cancel_event.clone(),
        //     MarketplaceEventConfig::from_event_config(
        //         &self.event_config,
        //         MarketplaceEventType::CancelCollectionOffer,
        //     )?,
        // );
        // mapping.insert(
        //     self.collection_offer.fill_event.clone(),
        //     MarketplaceEventConfig::from_event_config(
        //         &self.event_config,
        //         MarketplaceEventType::FillCollectionOffer,
        //     )?,
        // );
        Ok(mapping)
    }
}

#[derive(Clone, Debug)]
pub struct MarketplaceEventConfig {
    pub event_type: MarketplaceEventType,
    pub creator_address: HashableJsonPath,
    pub collection_id: HashableJsonPath,
    pub token_name: HashableJsonPath,
    pub price: HashableJsonPath,
    pub token_amount: HashableJsonPath,
    pub seller: HashableJsonPath,
}

impl MarketplaceEventConfig {
    pub fn from_event_config(
        event_config: &EventConfig,
        event_type: MarketplaceEventType,
    ) -> Result<Self> {
        Ok(Self {
            event_type,
            creator_address: HashableJsonPath::new(&event_config.creator_address)?,
            collection_id: HashableJsonPath::new(&event_config.collection_id)?,
            token_name: HashableJsonPath::new(&event_config.token_name)?,
            price: HashableJsonPath::new(&event_config.price)?,
            token_amount: HashableJsonPath::new(&event_config.token_amount)?,
            seller: HashableJsonPath::new(&event_config.seller)?,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ListingConfig {
    pub cancel_event: String,
    pub fill_event: String,
    pub place_event: String,
    // TODO: add more fields for struct
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OfferConfig {
    pub cancel_event: String,
    pub fill_event: String,
    pub place_event: String,
    // TODO: add more fields for struct
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionOfferConfig {
    pub cancel_event: String,
    pub fill_event: String,
    pub place_event: String,
    // TODO: add more fields for struct
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EventConfig {
    pub creator_address: String,
    pub collection_id: String,
    pub token_name: String,
    pub price: String,
    pub token_amount: String,
    pub seller: String,
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
