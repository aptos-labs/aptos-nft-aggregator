use crate::models::nft_models::{
    CurrentNFTMarketplaceCollectionOffer, CurrentNFTMarketplaceListing,
    CurrentNFTMarketplaceTokenOffer, MarketplaceField, MarketplaceModel, NftMarketplaceActivity,
};
use aptos_indexer_processor_sdk::{
    traits::{AsyncRunType, AsyncStep, NamedStep, Processable},
    types::transaction_context::TransactionContext,
    utils::errors::ProcessorError,
};
use log::debug;
use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, Mutex},
};

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
        self,
    ) -> (
        Vec<NftMarketplaceActivity>,
        Vec<CurrentNFTMarketplaceListing>,
        Vec<CurrentNFTMarketplaceTokenOffer>,
        Vec<CurrentNFTMarketplaceCollectionOffer>,
    ) {
        (
            self.activities,
            self.listings.into_values().collect(),
            self.token_offers.into_values().collect(),
            self.collection_offers.into_values().collect(),
        )
    }
}

#[derive(Clone, Debug, Default)]
pub struct NFTReductionStep {
    accumulator: Arc<Mutex<NFTAccumulator>>,
}

impl NFTReductionStep {
    pub fn new() -> Self {
        Self {
            accumulator: Arc::new(Mutex::new(NFTAccumulator::default())),
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
        Vec<NftMarketplaceActivity>,
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
        let accumulator = self.accumulator.clone();
        let (
            activities,
            current_listings,
            current_token_offers,
            current_collection_offers,
            resource_updates,
        ) = transactions.data;

        let mut acc = accumulator.lock().expect("Failed to acquire lock");

        let mut token_updates: HashMap<String, CurrentNFTMarketplaceListing> = HashMap::new();
        let mut token_offer_updates: HashMap<String, CurrentNFTMarketplaceTokenOffer> =
            HashMap::new();
        let mut collection_offer_updates: HashMap<String, CurrentNFTMarketplaceCollectionOffer> =
            HashMap::new();

        activities.into_iter().for_each(|a| acc.add_activity(a));

        current_listings.into_iter().for_each(|listing| {
            let token_data_id = listing.token_data_id.clone();
            token_updates.insert(token_data_id, listing);
        });

        current_token_offers.into_iter().for_each(|offer| {
            let token_data_id = offer.token_data_id.clone();
            token_offer_updates.insert(token_data_id, offer);
        });

        current_collection_offers
            .into_iter()
            .for_each(|collection_offer| {
                if !collection_offer.collection_offer_id.is_empty() {
                    // If we have a token_data_id, also store it for resource updates
                    if let Some(token_data_id) = collection_offer.token_data_id.as_ref() {
                        println!("Token data id: {:#?}", token_data_id);
                        collection_offer_updates.insert(token_data_id.clone(), collection_offer);
                    } else {
                        // if token_data_id is not present, we don't need resource updates, but since this vector is used for the accumulator, we still need to store it otherwise, it won't be stored in the db.
                        collection_offer_updates.insert(
                            collection_offer.collection_offer_id.clone(),
                            collection_offer.clone(),
                        );
                    }
                } else {
                    println!("Skipping collection offer with empty collection_offer_id");
                }
            });

        // TODO: figure out how we can enrich activities with resource updates. for now, join can be used to enrich activities with other tables. as we are only using this to enrich fields (e.g. token_name) that are not critical, this should be fine.
        for (resource_address, partial_update) in &resource_updates {
            if let Some(listing) = token_updates.get_mut(resource_address) {
                merge_partial_update(listing, partial_update);
            }
            if let Some(offer) = token_offer_updates.get_mut(resource_address) {
                merge_partial_update(offer, partial_update);
            }
            if let Some(collection_offer) = collection_offer_updates.get_mut(resource_address) {
                merge_partial_update(collection_offer, partial_update);
            }
        }

        token_updates
            .into_iter()
            .for_each(|(_, listing)| acc.fold_listing(listing));
        token_offer_updates
            .into_iter()
            .for_each(|(_, offer)| acc.fold_token_offer(offer));
        collection_offer_updates
            .into_iter()
            .for_each(|(_, offer)| acc.fold_collection_offer(offer));

        let reduced_data = acc.clone().drain();

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
            debug!(
                "Field {} is not set in the event, using write_set_changes value",
                column
            );
            model.set_field(MarketplaceField::from_str(column).unwrap(), value.clone());
        } else {
            debug!("Field {} is set in the event, keeping event value", column);
        }
    }
}
