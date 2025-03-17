use crate::{
    config::marketplace_config::{
        ContractToMarketplaceMap, MarketplaceEventConfigMappings, MarketplaceEventType,
    },
    models::nft_models::{
        CurrentNFTMarketplaceCollectionOffer, CurrentNFTMarketplaceListing,
        CurrentNFTMarketplaceTokenOffer, NftMarketplaceActivity,
    },
    utils::parse_timestamp,
};
use anyhow::Result;
use aptos_indexer_processor_sdk::utils::errors::ProcessorError;
use aptos_protos::transaction::v1::{transaction::TxnData, Transaction};
use std::{str::FromStr, sync::Arc};

pub struct EventRemapper {
    pub event_mappings: Arc<MarketplaceEventConfigMappings>,
    pub contract_to_marketplace_map: Arc<ContractToMarketplaceMap>,
}

impl EventRemapper {
    pub fn new(
        event_mappings: Arc<MarketplaceEventConfigMappings>,
        contract_to_marketplace_map: Arc<ContractToMarketplaceMap>,
    ) -> Self {
        Self {
            event_mappings,
            contract_to_marketplace_map,
        }
    }

    /**
     * Remaps the fields of the events in the transaction to build a NftMarketplaceActivity
     */
    pub fn remap_events(
        &self,
        txn: Transaction,
    ) -> Result<
        (
            Vec<NftMarketplaceActivity>,
            Vec<CurrentNFTMarketplaceListing>,
            Vec<CurrentNFTMarketplaceTokenOffer>,
            Vec<CurrentNFTMarketplaceCollectionOffer>,
        ),
        ProcessorError,
    > {
        let mut activities: Vec<NftMarketplaceActivity> = Vec::new();
        let mut current_listings: Vec<CurrentNFTMarketplaceListing> = Vec::new();
        let mut current_token_offers: Vec<CurrentNFTMarketplaceTokenOffer> = Vec::new();
        let mut current_collection_offers: Vec<CurrentNFTMarketplaceCollectionOffer> = Vec::new();
        let txn_data = txn.txn_data.as_ref().unwrap();

        if let TxnData::User(tx_inner) = txn_data {
            let events = tx_inner.events.clone();
            let txn_timestamp =
                parse_timestamp(txn.timestamp.as_ref().unwrap(), txn.version as i64);

            for (event_index, event) in events.iter().enumerate() {
                if let Some(activity) = NftMarketplaceActivity::from_event(
                    event,
                    txn.version as i64,
                    event_index as i64,
                    txn_timestamp,
                    &self.event_mappings,
                    &self.contract_to_marketplace_map,
                ) {
                    match MarketplaceEventType::from_str(activity.standard_event_type.as_str())
                        .unwrap()
                    {
                        MarketplaceEventType::PlaceListing => {
                            let current_listing =
                                CurrentNFTMarketplaceListing::from_activity(&activity, false);
                            current_listings.push(current_listing);
                        },
                        MarketplaceEventType::CancelListing | MarketplaceEventType::FillListing => {
                            let current_listing =
                                CurrentNFTMarketplaceListing::from_activity(&activity, true);
                            current_listings.push(current_listing);
                        },
                        MarketplaceEventType::PlaceOffer => {
                            let current_token_offer =
                                CurrentNFTMarketplaceTokenOffer::from_activity(&activity, false);
                            current_token_offers.push(current_token_offer);
                        },
                        MarketplaceEventType::CancelOffer | MarketplaceEventType::FillOffer => {
                            let current_token_offer =
                                CurrentNFTMarketplaceTokenOffer::from_activity(&activity, true);
                            current_token_offers.push(current_token_offer);
                        },
                        MarketplaceEventType::PlaceCollectionOffer => {
                            let current_collection_offer =
                                CurrentNFTMarketplaceCollectionOffer::from_activity(
                                    &activity, false,
                                );
                            current_collection_offers.push(current_collection_offer);
                        },
                        MarketplaceEventType::CancelCollectionOffer => {
                            let current_collection_offer =
                                CurrentNFTMarketplaceCollectionOffer::from_activity(
                                    &activity, true,
                                );
                            current_collection_offers.push(current_collection_offer);
                        },
                        MarketplaceEventType::FillCollectionOffer => {
                            let current_collection_offer =
                                CurrentNFTMarketplaceCollectionOffer::from_activity(
                                    &activity, true,
                                );
                            current_collection_offers.push(current_collection_offer);
                        },
                    }
                    activities.push(activity);
                }
            }
        }

        Ok((
            activities,
            current_listings,
            current_token_offers,
            current_collection_offers,
        ))
    }
}
