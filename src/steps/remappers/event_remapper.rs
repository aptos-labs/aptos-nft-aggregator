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
use aptos_indexer_processor_sdk::utils::{
    errors::ProcessorError,
    extract::{
        get_clean_entry_function_payload_from_user_request, get_entry_function_from_user_request,
    },
};
use aptos_protos::transaction::v1::{move_type as pb_move_type, transaction::TxnData, Transaction};
use std::{collections::HashMap, str::FromStr, sync::Arc};

pub struct EventRemapper {
    pub event_mappings: Arc<MarketplaceEventConfigMappings>,
    pub contract_to_marketplace_map: Arc<ContractToMarketplaceMap>,
    // pub conn_pool: ArcDbPool,
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
        batch_listing_map: &mut HashMap<String, String>,
        // conn: &mut DbPoolConnection<'_>,
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
            let req = tx_inner
                .request
                .as_ref()
                .expect("Sends is not present in user txn");

            let entry_function_id = get_entry_function_from_user_request(req);

            let events = tx_inner.events.clone();
            let txn_timestamp =
                parse_timestamp(txn.timestamp.as_ref().unwrap(), txn.version as i64);

            let mut coin_type = None;
            if let Some(clean_payload) =
                get_clean_entry_function_payload_from_user_request(req, txn.version as i64)
            {
                if !clean_payload.type_arguments.is_empty() {
                    let extracted_move_type = Some(clean_payload.type_arguments[0].clone());
                    if let Some(move_type) = &extracted_move_type {
                        match move_type.content.as_ref().unwrap() {
                            pb_move_type::Content::Struct(struct_tag) => {
                                coin_type = Some(format!(
                                    "{}::{}::{}",
                                    struct_tag.address, struct_tag.module, struct_tag.name
                                ));
                            },
                            _ => {
                                println!("Skipping non-struct type");
                            },
                        }
                    }
                }
            }

            for (event_index, event) in events.iter().enumerate() {
                if let Some(activity) = NftMarketplaceActivity::from_event(
                    event,
                    txn.version as i64,
                    event_index as i64,
                    txn_timestamp,
                    &entry_function_id,
                    &self.event_mappings,
                    &self.contract_to_marketplace_map,
                    token_metadatas,
                    coin_type.clone(),
                ) {
                    // populate map with any new listing_id generated from Place eevnts within the same batch
                    // for any subsequent cancel or fill events within the same batch, lookup this map before hitting the database.
                    // only fallback to a db lookup if the listing wasn't found in-memory (indicating the listing was placed in an earlier batch)
                    if let Some(token_data_id) = &activity.token_data_id {
                        if activity.listing_id.is_some() {
                            batch_listing_map.insert(
                                token_data_id.clone(),
                                activity.listing_id.clone().unwrap(),
                            );
                        }
                    }

                    // Handle current state updates based on activity type
                    match MarketplaceEventType::from_str(activity.standard_event_type.as_str())
                        .unwrap()
                    {
                        MarketplaceEventType::PlaceListing => {
                            let current_listing =
                                CurrentNFTMarketplaceListing::build_listing_plaes_events(&activity);
                            current_listings.push(current_listing);
                        },
                        MarketplaceEventType::CancelListing | MarketplaceEventType::FillListing => {
                            if let Some(current_listing) =
                                CurrentNFTMarketplaceListing::build_cancel_or_fill_listing(
                                    &activity,
                                    batch_listing_map,
                                )
                            {
                                current_listings.push(current_listing);
                            }
                        },
                        MarketplaceEventType::PlaceOffer => {
                            let current_token_offer =
                                CurrentNFTMarketplaceTokenOffer::from_activity(&activity, false);
                            current_token_offers.push(current_token_offer);
                        },
                        MarketplaceEventType::CancelOffer | MarketplaceEventType::FillOffer => {
                            if let Some(current_token_offer) =
                                CurrentNFTMarketplaceTokenOffer::build_cancel_or_fill_offer(
                                    &activity,
                                    batch_listing_map,
                                )
                            {
                                current_token_offers.push(current_token_offer);
                            }
                        },
                        MarketplaceEventType::PlaceCollectionOffer => {
                            let current_collection_offer =
                                CurrentNFTMarketplaceCollectionOffer::from_activity(
                                    &activity, false,
                                );
                            current_collection_offers.push(current_collection_offer);
                        },
                        MarketplaceEventType::CancelCollectionOffer => {
                            if let Some(current_collection_offer) =
                                CurrentNFTMarketplaceCollectionOffer::build_cancel_or_fill_offer(
                                    &activity,
                                    batch_listing_map,
                                )
                            {
                                current_collection_offers.push(current_collection_offer);
                            }
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
                                price: activity.price.unwrap_or_default(),
                                buyer: activity.buyer.clone().unwrap_or_default(),
                                fee_schedule_id: activity
                                    .fee_schedule_id
                                    .clone()
                                    .unwrap_or_default(),
                                marketplace_name: activity.marketplace.clone(),
                                marketplace_contract_address: activity.contract_address.clone(),
                            };

                            // // TODO: we need to decide what to use as the key for the collection offer filled metadata
                            // let collection_offer_id = activity.json_data
                            //     .get("collection_offer")
                            //     .and_then(|v| v.as_str())
                            //     .unwrap_or_default()
                            //     .to_string();

                            let offer_id = activity.offer_id.clone().unwrap_or_default();
                            println!("collection offer filled metadata {:?}", offer_id);
                            collection_offer_filled_metadatas
                                .insert(offer_id, collection_offer_filled_metadata);
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
