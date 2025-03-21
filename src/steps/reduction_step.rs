use crate::models::nft_models::{
    CurrentNFTMarketplaceCollectionOffer, CurrentNFTMarketplaceListing,
    CurrentNFTMarketplaceTokenOffer, MarketplaceModel, NftMarketplaceActivity,
};
use aptos_indexer_processor_sdk::{
    traits::{AsyncRunType, AsyncStep, NamedStep, Processable},
    types::transaction_context::TransactionContext,
    utils::errors::ProcessorError,
};
use lazy_static::lazy_static;
use log::debug;
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex},
};

#[derive(Clone, Debug, Default)]
pub struct NFTAccumulator {
    activities: Vec<NftMarketplaceActivity>,
    listings: HashMap<String, CurrentNFTMarketplaceListing>,
    token_offers: HashMap<String, CurrentNFTMarketplaceTokenOffer>,
    collection_offers: HashMap<String, CurrentNFTMarketplaceCollectionOffer>,
}

// TODO: Revisit how we can reduce
impl NFTAccumulator {
    pub fn fold_listing(&mut self, listing: CurrentNFTMarketplaceListing) {
        // todo: use correct primary key
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
        // todo: use correct primary key
        let key = format!("{}::{}", offer.marketplace, offer.collection_offer_id);
        self.collection_offers.insert(key, offer);
    }

    pub fn add_activity(&mut self, activity: NftMarketplaceActivity) {
        // let key = format!("{}::{}::{}", activity.marketplace, activity.txn_version, activity.index);
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
        Vec<NftMarketplaceActivity>,               // From EventRemapper
        Vec<CurrentNFTMarketplaceListing>,         // From EventRemapper
        Vec<CurrentNFTMarketplaceTokenOffer>,      // From EventRemapper
        Vec<CurrentNFTMarketplaceCollectionOffer>, // From EventRemapper
        HashMap<String, HashMap<String, String>>,  // Partial updates from write_set_changes
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

        let mut token_updates: BTreeMap<String, CurrentNFTMarketplaceListing> = BTreeMap::new();
        let mut token_offer_updates: BTreeMap<String, CurrentNFTMarketplaceTokenOffer> =
            BTreeMap::new();
        let mut collection_offer_updates: BTreeMap<
            String,
            CurrentNFTMarketplaceCollectionOffer,
        > = BTreeMap::new();

        // Store all activities (event logs, no merging needed)
        activities
            .into_iter()
            .for_each(|a| acc.add_activity(a));

        // Process listings (keeping only the latest event per NFT)
        current_listings.into_iter().for_each(|listing| {
            let token_data_id = listing.token_data_id.clone();
            token_updates.insert(token_data_id, listing);
        });

        // Process token offers
        current_token_offers.into_iter().for_each(|offer| {
            let token_data_id = offer.token_data_id.clone();
            token_offer_updates.insert(token_data_id, offer);
        });

        // Process collection offers
        // we have to use resource_address as the key for collection offers
        current_collection_offers
            .into_iter()
            .for_each(|collection_offer| {
                println!("Collection offer: {:#?}", collection_offer);
                if let Some(token_data_id) = collection_offer.token_data_id.as_ref() {
                    println!("Token data id: {:#?}", token_data_id);
                    collection_offer_updates
                        .insert(token_data_id.clone(), collection_offer.clone());
                }
                // TODO: Revisit if we have to look at other addresses
            });

        // Apply write_set_changes (AFTER event updates)
        for (resource_address, partial_update) in &resource_updates {
            println!("Resource address: {}", resource_address);
            if let Some(listing) = token_updates.get_mut(resource_address) {
                println!("Found listing: {:#?}", listing);
                merge_partial_update(listing, partial_update);
            }
            if let Some(offer) = token_offer_updates.get_mut(resource_address) {
                println!("Found offer: {:#?}", offer);
                merge_partial_update(offer, partial_update);
            }
            if let Some(collection_offer) =
            collection_offer_updates.get_mut(resource_address)
            {
                println!(
                    "Found collection offer with token data id: {:#?}",
                    collection_offer
                );
                merge_partial_update(collection_offer, partial_update);
            }
        }

        // Store final updates in accumulator
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
    let table_name = model.table_name();
    println!("Table name: {}", table_name);
    for (column, value) in partial_update {
        println!("Column: {}", column);
        println!("Value: {}", value);

        // Only update if the field is not set in the event or is empty
        if model.get_field(column).is_none() || model.get_field(column).is_some_and(|v| v.is_empty()) {
            debug!("Field {} is not set in the event, using write_set_changes value", column);
            model.set_field(column, value.clone());
        } else {
            debug!("Field {} is set in the event, keeping event value", column);
        }
    }
}
