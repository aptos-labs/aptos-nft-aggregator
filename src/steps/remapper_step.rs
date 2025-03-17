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
use std::sync::Arc;
use tonic::async_trait;

pub struct RemapResult {
    pub activities: Vec<NftMarketplaceActivity>,
    pub errors: Vec<String>,
}

pub struct ProcessStep {
    pub event_remapper: Arc<EventRemapper>,
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
                let event_remapper = self.event_remapper.clone();
                match event_remapper.remap_events(txn.clone()) {
                    Ok((activities, listings, token_offers, collection_offers)) => {
                        Ok((activities, listings, token_offers, collection_offers))
                    },
                    Err(e) => Err(ProcessorError::ProcessError {
                        message: format!("Error remapping events: {:#}", e),
                    }),
                }
            })
            .collect::<Result<Vec<_>, ProcessorError>>()?;

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
