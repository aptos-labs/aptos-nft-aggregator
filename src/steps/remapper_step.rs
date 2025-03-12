use super::remappers::resource_remapper::ResourceMapper;
use crate::{
    models::nft_models::{
        CurrentNFTMarketplaceCollectionOffer, CurrentNFTMarketplaceListing,
        CurrentNFTMarketplaceTokenOffer, NftMarketplaceActivity,
    },
    steps::remappers::event_remapper::EventRemapper,
};
use anyhow::Result;
use aptos_indexer_processor_sdk::{
    traits::{AsyncRunType, AsyncStep, NamedStep, Processable},
    types::transaction_context::TransactionContext,
    utils::errors::ProcessorError,
};
use aptos_protos::transaction::v1::Transaction;
use std::{collections::HashMap, sync::Arc};
use tonic::async_trait;

pub struct RemapResult {
    pub activities: Vec<NftMarketplaceActivity>,
    pub errors: Vec<String>,
}

// impl EventRemapper {
pub struct ProcessStep {
    pub event_remapper: Arc<EventRemapper>,
    pub resource_mapper: Arc<ResourceMapper>,
}

impl ProcessStep {
    pub fn new(event_remapper: Arc<EventRemapper>, resource_mapper: Arc<ResourceMapper>) -> Self {
        Self {
            event_remapper,
            resource_mapper,
        }
    }
}

#[async_trait]
impl Processable for ProcessStep {
    type Input = Vec<Transaction>;
    type Output = (
        Vec<NftMarketplaceActivity>,
        Vec<CurrentNFTMarketplaceListing>,
        Vec<CurrentNFTMarketplaceTokenOffer>,
        Vec<CurrentNFTMarketplaceCollectionOffer>,
    );
    type RunType = AsyncRunType;

    async fn process(
        &mut self,
        transactions: TransactionContext<Vec<Transaction>>,
    ) -> Result<
        Option<
            TransactionContext<(
                Vec<NftMarketplaceActivity>,
                Vec<CurrentNFTMarketplaceListing>,
                Vec<CurrentNFTMarketplaceTokenOffer>,
                Vec<CurrentNFTMarketplaceCollectionOffer>,
            )>,
        >,
        ProcessorError,
    > {
        let results: Vec<(
            Vec<NftMarketplaceActivity>,
            Vec<CurrentNFTMarketplaceListing>,
            Vec<CurrentNFTMarketplaceTokenOffer>,
            Vec<CurrentNFTMarketplaceCollectionOffer>,
        )> = transactions
            .data
            .iter()
            .map(|txn| {
                // Clone remappers to bump Arc reference count
                let event_remapper = self.event_remapper.clone();
                let resource_mapper = self.resource_mapper.clone();

                let mut filled_collection_offers_from_events: HashMap<
                    String,
                    CurrentNFTMarketplaceCollectionOffer,
                > = HashMap::new();
                let mut filled_token_offers_from_events: HashMap<
                    String,
                    CurrentNFTMarketplaceTokenOffer,
                > = HashMap::new();
                let mut filled_listings_from_events: HashMap<String, CurrentNFTMarketplaceListing> =
                    HashMap::new();

                let (activities, mut listings, mut token_offers, mut collection_offers) =
                    match event_remapper.remap_events(
                        txn.clone(),
                        &mut filled_collection_offers_from_events,
                        &mut filled_token_offers_from_events,
                        &mut filled_listings_from_events,
                    ) {
                        Ok((event_activities, listings, token_offers, collection_offers)) => {
                            (event_activities, listings, token_offers, collection_offers)
                        },
                        Err(e) => {
                            // Log error and continue with empty vector
                            eprintln!("Error remapping events: {:#}", e);
                            (vec![], vec![], vec![], vec![])
                        },
                    };

                if let Ok(resource_result) = resource_mapper.remap_resources(
                    txn.clone(),
                    &mut filled_collection_offers_from_events,
                    &mut filled_token_offers_from_events,
                    &mut filled_listings_from_events,
                ) {
                    listings.extend(resource_result.listings);
                    token_offers.extend(resource_result.token_offers);
                    collection_offers.extend(resource_result.collection_offers);
                }
                (activities, listings, token_offers, collection_offers)
            })
            .collect();

        // Combine all activities and listings
        let mut all_activities: Vec<NftMarketplaceActivity> = Vec::new();
        let mut all_listings: Vec<CurrentNFTMarketplaceListing> = Vec::new();
        let mut all_token_offers: Vec<CurrentNFTMarketplaceTokenOffer> = Vec::new();
        let mut all_collection_offers: Vec<CurrentNFTMarketplaceCollectionOffer> = Vec::new();

        for (activities, listings, token_offers, collection_offers) in results {
            all_activities.extend(activities);
            all_listings.extend(listings);
            all_token_offers.extend(token_offers);
            all_collection_offers.extend(collection_offers);
        }

        Ok(Some(TransactionContext {
            data: (
                all_activities,
                all_listings,
                all_token_offers,
                all_collection_offers,
            ),
            metadata: transactions.metadata,
        }))
    }
}

impl AsyncStep for ProcessStep {}

impl NamedStep for ProcessStep {
    fn name(&self) -> String {
        "ProcessStep".to_string()
    }
}
