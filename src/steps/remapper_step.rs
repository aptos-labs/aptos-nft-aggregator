use super::remappers::{event_remapper, resource_remapper::ResourceMapper};
use crate::{
    config::marketplace_config::{MarketplaceEventType, NFTMarketplaceConfig},
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
use rayon::iter::{
    IntoParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
};

pub struct RemapResult {
    pub activities: Vec<NftMarketplaceActivity>,
    pub errors: Vec<String>,
}

// impl EventRemapper {
pub struct ProcessStep {
    event_remapper: Arc<EventRemapper>,
    // pub resource_mapper: Arc<ResourceMapper>,
}

impl ProcessStep {
    pub fn new(config: NFTMarketplaceConfig) -> anyhow::Result<Self> {
        let event_remapper: Arc<EventRemapper> = EventRemapper::new(&config)?;
        Ok(Self { event_remapper })
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
        let results = transactions
            .data
            .par_iter()
            .map(|transaction| {
                let event_remapper = self.event_remapper.clone();
                
                // Run all remappers and combine their results
                let mut remapper_results = Vec::new();
                
                // Add event remapper
                remapper_results.push(event_remapper.remap_events(transaction.clone())?);
                
                // Add other remappers here like:
                // remapper_results.push(other_remapper.remap(transaction.clone())?);
                
                // Combine results from all remappers
                Ok(remapper_results.into_iter().fold(
                    (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
                    |(mut activities, mut listings, mut token_offers, mut collection_offers),
                     (new_activities, new_listings, new_token_offers, new_collection_offers)| {
                        activities.extend(new_activities);
                        listings.extend(new_listings);
                        token_offers.extend(new_token_offers);
                        collection_offers.extend(new_collection_offers);
                        (activities, listings, token_offers, collection_offers)
                    }
                ))
            })
            .collect::<anyhow::Result<Vec<_>>>()
            .map_err(|e| ProcessorError::ProcessError {
                message: format!("{:#}", e),
            })?;

        // Combine results from all transactions
        let (activities, listings, token_offers, collection_offers) = 
            results.into_iter().fold(
                (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
                |(mut activities, mut listings, mut token_offers, mut collection_offers),
                 (new_activities, new_listings, new_token_offers, new_collection_offers)| {
                    activities.extend(new_activities);
                    listings.extend(new_listings);
                    token_offers.extend(new_token_offers);
                    collection_offers.extend(new_collection_offers);
                    (activities, listings, token_offers, collection_offers)
                }
            );

        Ok(Some(TransactionContext {
            data: (activities, listings, token_offers, collection_offers),
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
