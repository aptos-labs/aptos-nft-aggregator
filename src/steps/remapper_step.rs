use super::remappers::resource_remapper::ResourceMapper;
use crate::{
    config::marketplace_config::MarketplaceEventType,
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
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
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

    fn process_single_transaction(
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
        let mut activity_map: HashMap<
            String,
            HashMap<MarketplaceEventType, NftMarketplaceActivity>,
        > = HashMap::new();

        self.event_remapper
            .remap_events(txn.clone(), &mut activity_map)
            .map_err(|e| ProcessorError::ProcessError {
                message: format!("Error remapping events: {:#}", e),
            })?;

        if !activity_map.is_empty() {
            self.resource_mapper
                .remap_resources(txn.clone(), &mut activity_map)
                .map_err(|e| ProcessorError::ProcessError {
                    message: format!("Error remapping resources: {:#}", e),
                })?;
        }

        let mut final_activities = Vec::new();
        let mut current_listings = Vec::new();
        let mut current_token_offers = Vec::new();
        let mut current_collection_offers = Vec::new();

        for (_, event_activities) in activity_map {
            for (_, activity) in event_activities {
                final_activities.push(activity.clone());

                match activity.standard_event_type {
                    MarketplaceEventType::PlaceListing
                    | MarketplaceEventType::CancelListing
                    | MarketplaceEventType::FillListing => {
                        current_listings
                            .push(CurrentNFTMarketplaceListing::from_activity(&activity));
                    },
                    MarketplaceEventType::PlaceTokenOffer
                    | MarketplaceEventType::CancelTokenOffer
                    | MarketplaceEventType::FillTokenOffer => {
                        current_token_offers
                            .push(CurrentNFTMarketplaceTokenOffer::from_activity(&activity));
                    },
                    MarketplaceEventType::PlaceCollectionOffer
                    | MarketplaceEventType::CancelCollectionOffer
                    | MarketplaceEventType::FillCollectionOffer => {
                        current_collection_offers.push(
                            CurrentNFTMarketplaceCollectionOffer::from_activity(&activity),
                        );
                    },
                    _ => {
                        eprintln!("Unknown event type: {:?}", activity.standard_event_type);
                    },
                }
            }
        }

        Ok((
            final_activities,
            current_listings,
            current_token_offers,
            current_collection_offers,
        ))
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
        let results: Result<Vec<_>, ProcessorError> = transactions
            .data
            .par_iter()
            .map(|txn| self.process_single_transaction(txn.clone()))
            .collect();

        match results {
            Ok(results) => {
                let mut all_activities = Vec::new();
                let mut all_listings = Vec::new();
                let mut all_token_offers = Vec::new();
                let mut all_collection_offers = Vec::new();

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
            },
            Err(e) => {
                eprintln!("Error processing transactions: {:#}", e);
                Err(e)
            },
        }
    }
}

impl AsyncStep for ProcessStep {}

impl NamedStep for ProcessStep {
    fn name(&self) -> String {
        "ProcessStep".to_string()
    }
}
