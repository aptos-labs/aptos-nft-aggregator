use crate::{
    models::nft_models::{
        CurrentNFTMarketplaceCollectionOffer, CurrentNFTMarketplaceListing,
        CurrentNFTMarketplaceTokenOffer, NftMarketplaceActivity,
    },
    steps::remappers::event_remapper::EventRemapper,
    utils::marketplace_resource_utils::{
        CollectionOfferEventMetadata, ListingEventMetadata, TokenMetadata, TokenOfferEventMetadata,
    },
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
    // pub resource_mapper: Arc<ResourceMapper>,
}

impl ProcessStep {
    pub fn new(event_remapper: Arc<EventRemapper>) -> Self {
        Self { event_remapper }
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
                // let resource_mapper = self.resource_mapper.clone();

                // These are the metadata maps that we will need for resource remappers.
                // For those events reqruing wscs lookup, we just delegate to resource remapper to create models.
                let mut token_metadatas: HashMap<String, TokenMetadata> = HashMap::new();
                let mut collection_offer_filled_metadatas: HashMap<
                    String,
                    CollectionOfferEventMetadata,
                > = HashMap::new();
                let mut token_offer_filled_metadatas: HashMap<String, TokenOfferEventMetadata> =
                    HashMap::new();
                let mut listing_filled_metadatas: HashMap<String, ListingEventMetadata> =
                    HashMap::new();

                let (activities, listings, token_offers, collection_offers) = match event_remapper
                    .remap_events(
                        txn.clone(),
                        &mut token_metadatas,
                        &mut collection_offer_filled_metadatas,
                        &mut token_offer_filled_metadatas,
                        &mut listing_filled_metadatas,
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

                // only process with resource mapper if there are matching resource addresses?
                // We don't want to process resources all the time, but there are some cases where events are not enough to build the current state (e.g. exipiration time is not available in events)
                // However, we need to decide first if we want to include in our schema or not.
                // Just append resource results if successful
                // if let Ok(resource_result) = resource_mapper.remap_resources(
                //     txn.clone(),
                //     &mut token_metadatas,
                //     &mut collection_offer_filled_metadatas,
                //     &mut token_offer_filled_metadatas,
                //     &mut listing_filled_metadatas,
                // ) {
                //     listings.extend(resource_result.listings);
                //     token_offers.extend(resource_result.token_offers);
                //     collection_offers.extend(resource_result.collection_offers);
                // }

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
