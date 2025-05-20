use crate::{
    config::marketplace_config::MarketplaceEventType,
    models::nft_models::{
        CurrentNFTMarketplaceCollectionOffer, CurrentNFTMarketplaceListing,
        CurrentNFTMarketplaceTokenOffer, MarketplaceField, MarketplaceModel,
        NftMarketplaceActivity,
    },
};
use aptos_indexer_processor_sdk::{
    traits::{AsyncRunType, AsyncStep, NamedStep, Processable},
    types::transaction_context::TransactionContext,
    utils::errors::ProcessorError,
};
use log::debug;
use std::{collections::HashMap, mem, str::FromStr};

#[derive(Clone, Debug, Default)]
pub struct NFTAccumulator {
    activities: Vec<NftMarketplaceActivity>,
    listings: HashMap<String, CurrentNFTMarketplaceListing>,
    token_offers: HashMap<String, CurrentNFTMarketplaceTokenOffer>,
    collection_offers: HashMap<String, CurrentNFTMarketplaceCollectionOffer>,
}

impl NFTAccumulator {
    pub fn fold_listing(&mut self, listing: CurrentNFTMarketplaceListing) {
        let key = format!("{}::{}", listing.marketplace, listing.token_data_id);
        self.listings.insert(key, listing);
    }

    pub fn fold_token_offer(&mut self, offer: CurrentNFTMarketplaceTokenOffer) {
        let key = format!(
            "{}::{}::{}",
            offer.marketplace, offer.token_data_id, offer.buyer
        );
        self.token_offers.insert(key, offer);
    }

    pub fn fold_collection_offer(&mut self, offer: CurrentNFTMarketplaceCollectionOffer) {
        let key = format!("{}::{}", offer.marketplace, offer.collection_offer_id);
        self.collection_offers.insert(key, offer);
    }

    pub fn add_activity(&mut self, activity: NftMarketplaceActivity) {
        self.activities.push(activity);
    }

    pub fn drain(
        &mut self,
    ) -> (
        Vec<NftMarketplaceActivity>,
        Vec<CurrentNFTMarketplaceListing>,
        Vec<CurrentNFTMarketplaceTokenOffer>,
        Vec<CurrentNFTMarketplaceCollectionOffer>,
    ) {
        (
            mem::take(&mut self.activities),
            self.listings.drain().map(|(_, v)| v).collect(),
            self.token_offers.drain().map(|(_, v)| v).collect(),
            self.collection_offers.drain().map(|(_, v)| v).collect(),
        )
    }
}

#[derive(Clone, Debug, Default)]
pub struct NFTReductionStep
where
    Self: Sized + Send + 'static,
{
    accumulator: NFTAccumulator,
}

impl NFTReductionStep {
    pub fn new() -> Self {
        Self {
            accumulator: NFTAccumulator::default(),
        }
    }
}

pub type Tables = (
    Vec<NftMarketplaceActivity>,
    Vec<CurrentNFTMarketplaceListing>,
    Vec<CurrentNFTMarketplaceTokenOffer>,
    Vec<CurrentNFTMarketplaceCollectionOffer>,
);

#[async_trait::async_trait]
impl Processable for NFTReductionStep {
    type Input = (
        HashMap<i64, Vec<NftMarketplaceActivity>>,
        Vec<CurrentNFTMarketplaceListing>,
        Vec<CurrentNFTMarketplaceTokenOffer>,
        Vec<CurrentNFTMarketplaceCollectionOffer>,
        HashMap<String, HashMap<String, String>>,
    );
    type Output = (
        Vec<NftMarketplaceActivity>,
        Vec<CurrentNFTMarketplaceListing>,
        Vec<CurrentNFTMarketplaceTokenOffer>,
        Vec<CurrentNFTMarketplaceCollectionOffer>,
    );
    type RunType = AsyncRunType;

    async fn process(
        &mut self,
        transactions: TransactionContext<Self::Input>,
    ) -> Result<Option<TransactionContext<Self::Output>>, ProcessorError> {
        let (
            mut activities,
            current_listings,
            current_token_offers,
            current_collection_offers,
            resource_updates,
        ) = transactions.data;

        // Process listings with resource updates inline
        for listing in current_listings {
            if let Some(updates) = resource_updates.get(&listing.token_data_id) {
                let mut listing = listing;
                merge_partial_update(&mut listing, updates, &mut activities);
                self.accumulator.fold_listing(listing);
            } else {
                self.accumulator.fold_listing(listing);
            }
        }

        // Process token offers with resource updates inline
        for offer in current_token_offers {
            if let Some(updates) = resource_updates.get(&offer.token_data_id) {
                let mut offer = offer;
                merge_partial_update(&mut offer, updates, &mut activities);
                self.accumulator.fold_token_offer(offer);
            } else {
                self.accumulator.fold_token_offer(offer);
            }
        }

        // Process collection offers with resource updates inline
        for collection_offer in current_collection_offers {
            if !collection_offer.collection_offer_id.is_empty() {
                if let Some(token_data_id) = &collection_offer.token_data_id {
                    if let Some(updates) = resource_updates.get(token_data_id) {
                        let mut offer = collection_offer;
                        merge_partial_update(&mut offer, updates, &mut activities);
                        self.accumulator.fold_collection_offer(offer);
                    } else {
                        self.accumulator.fold_collection_offer(collection_offer);
                    }
                } else if let Some(updates) =
                    resource_updates.get(&collection_offer.collection_offer_id)
                {
                    let mut offer = collection_offer;
                    merge_partial_update(&mut offer, updates, &mut activities);
                    self.accumulator.fold_collection_offer(offer);
                } else {
                    self.accumulator.fold_collection_offer(collection_offer);
                }
            } else {
                debug!("Skipping collection offer with empty collection_offer_id");
            }
        }

        // process activities after all updates are applied
        for activities_vec_same_txn_version in activities.into_values() {
            for activity in activities_vec_same_txn_version {
                self.accumulator.add_activity(activity);
            }
        }

        let reduced_data = self.accumulator.drain();

        Ok(Some(TransactionContext {
            data: reduced_data,
            metadata: transactions.metadata,
        }))
    }
}

impl AsyncStep for NFTReductionStep {}

impl NamedStep for NFTReductionStep {
    fn name(&self) -> String {
        "NFTReductionStep".to_string()
    }
}

fn merge_partial_update<T: MarketplaceModel>(
    model: &mut T,
    partial_update: &HashMap<String, String>,
    activities: &mut HashMap<i64, Vec<NftMarketplaceActivity>>,
) {
    for (column, value) in partial_update {
        // Only update if the field is not set in the event or is empty
        if model
            .get_field(MarketplaceField::from_str(column).unwrap())
            .is_none()
            || model
                .get_field(MarketplaceField::from_str(column).unwrap())
                .is_some_and(|v| v.is_empty())
        {
            if let Some(activities_vec) = activities.get_mut(&model.get_txn_version()) {
                // Try to find matching activity based on token_data_id or collection_id
                if let Some(matching_activity) = activities_vec.iter_mut().find(|activity| {
                    // we should first check if it's one of collection offer types
                    let standard_event_type = model.get_standard_event_type();
                    if standard_event_type == MarketplaceEventType::PlaceCollectionOffer.to_string()
                        || standard_event_type
                            == MarketplaceEventType::CancelCollectionOffer.to_string()
                        || standard_event_type
                            == MarketplaceEventType::FillCollectionOffer.to_string()
                    {
                        // Match on collection_offer_id for collection offers
                        model
                            .get_field(MarketplaceField::CollectionOfferId)
                            .and_then(|collection_offer_id| {
                                activity.offer_id.as_ref().map(|activity_offer_id| {
                                    &collection_offer_id == activity_offer_id
                                })
                            })
                            .unwrap_or(false)
                    } else {
                        // Match on token_data_id for other events
                        model
                            .get_field(MarketplaceField::TokenDataId)
                            .and_then(|model_id| {
                                activity
                                    .token_data_id
                                    .as_ref()
                                    .map(|activity_id| &model_id == activity_id)
                            })
                            .unwrap_or(false)
                    }
                }) {
                    matching_activity
                        .set_field(MarketplaceField::from_str(column).unwrap(), value.clone());
                }
            }
            model.set_field(MarketplaceField::from_str(column).unwrap(), value.clone());
        }
    }
}
