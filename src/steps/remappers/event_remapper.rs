use crate::{
    config::marketplace_config::{
        ContractToMarketplaceMap, MarketplaceEventConfigMappings, MarketplaceEventType,
    },
    models::nft_models::{
        CurrentNFTMarketplaceCollectionOffer, CurrentNFTMarketplaceListing,
        CurrentNFTMarketplaceTokenOffer, NftMarketplaceActivity,
    },
    utils::{
        marketplace_resource_utils::{
            CollectionMetadata, CollectionOfferEventMetadata, ListingEventMetadata, TokenMetadata,
            TokenOfferEventMetadata, TokenStandard,
        },
        parse_timestamp,
    },
};
use anyhow::Result;
use aptos_indexer_processor_sdk::utils::errors::ProcessorError;
use aptos_protos::transaction::v1::{transaction::TxnData, Transaction};
use std::{collections::HashMap, str::FromStr, sync::Arc};

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
        token_metadatas: &mut HashMap<String, TokenMetadata>,
        collection_offer_filled_metadatas: &mut HashMap<String, CollectionOfferEventMetadata>,
        _token_offer_filled_metadatas: &mut HashMap<String, TokenOfferEventMetadata>,
        _listing_filled_metadatas: &mut HashMap<String, ListingEventMetadata>,
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
                    token_metadatas,
                ) {
                    // populate map with any new listing_id generated from Place eevnts within the same batch
                    // for any subsequent cancel or fill events within the same batch, lookup this map before hitting the database.
                    // only fallback to a db lookup if the listing wasn't found in-memory (indicating the listing was placed in an earlier batch)
                    // if let Some(token_data_id) = &activity.token_data_id {
                    //     if activity.listing_id.is_some() {
                    //         batch_listing_map.insert(
                    //             token_data_id.clone(),
                    //             activity.listing_id.clone().unwrap(),
                    //         );
                    //     }
                    // }

                    // Handle current state updates based on activity type
                    match MarketplaceEventType::from_str(activity.standard_event_type.as_str())
                        .unwrap()
                    {
                        MarketplaceEventType::PlaceListing => {
                            let current_listing =
                                CurrentNFTMarketplaceListing::from_activity(&activity);
                            current_listings.push(current_listing);
                        },
                        MarketplaceEventType::CancelListing | MarketplaceEventType::FillListing => {
                            let current_listing =
                                CurrentNFTMarketplaceListing::from_activity(&activity);
                            current_listings.push(current_listing);
                        },
                        MarketplaceEventType::PlaceOffer => {
                            let current_token_offer =
                                CurrentNFTMarketplaceTokenOffer::from_activity(&activity, false);
                            current_token_offers.push(current_token_offer);
                        },
                        MarketplaceEventType::CancelOffer | MarketplaceEventType::FillOffer => {
                            let current_token_offer =
                                CurrentNFTMarketplaceTokenOffer::build_cancelled_or_filled_token_offer_from_activity(&activity);
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
                            let collection_metadata = CollectionMetadata {
                                collection_id: activity.collection_id.clone().unwrap_or_default(),
                                creator_address: activity
                                    .creator_address
                                    .clone()
                                    .unwrap_or_default(),
                                collection_name: activity
                                    .collection_name
                                    .clone()
                                    .unwrap_or_default(),
                                token_standard: TokenStandard::from_str(
                                    activity.token_standard.clone().unwrap_or_default().as_str(),
                                )
                                .unwrap(),
                            };

                            let collection_offer_filled_metadata = CollectionOfferEventMetadata {
                                collection_offer_id: activity.offer_id.clone().unwrap_or_default(),
                                collection_metadata,
                                price: activity.price,
                                buyer: activity.buyer.clone().unwrap_or_default(),
                                marketplace_name: activity.marketplace.clone(),
                                marketplace_contract_address: activity.contract_address.clone(),
                            };

                            let offer_id = activity.offer_id.clone().unwrap_or_default();
                            // info!("collection offer filled metadata {:?}", offer_id);
                            collection_offer_filled_metadatas
                                .insert(offer_id, collection_offer_filled_metadata);

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

        // Deduplicate activities
        let mut activities_map: HashMap<(i64, i64), NftMarketplaceActivity> = HashMap::new();
        for activity in activities {
            let key = (activity.txn_version, activity.index);
            activities_map.entry(key).or_insert(activity);
        }
        let deduped_activities: Vec<NftMarketplaceActivity> =
            activities_map.into_values().collect();

        Ok((
            deduped_activities,
            current_listings,
            current_token_offers,
            current_collection_offers,
        ))
    }
}
