use super::remappers::resource_remapper::ResourceMapper;
use crate::{
    config::marketplace_config::NFTMarketplaceConfig,
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
    event_remapper: Arc<EventRemapper>,
    resource_remapper: Arc<ResourceMapper>,
}

impl ProcessStep {
    pub fn new(config: NFTMarketplaceConfig) -> anyhow::Result<Self> {
        let event_remapper: Arc<EventRemapper> = EventRemapper::new(&config)?;
        let resource_remapper: Arc<ResourceMapper> = ResourceMapper::new(&config)?;
        Ok(Self {
            event_remapper,
            resource_remapper,
        })
    }
}

#[async_trait]
impl Processable for ProcessStep {
    type Input = Vec<Transaction>;
    type Output = (
        HashMap<i64, Vec<NftMarketplaceActivity>>,
        Vec<CurrentNFTMarketplaceListing>,
        Vec<CurrentNFTMarketplaceTokenOffer>,
        Vec<CurrentNFTMarketplaceCollectionOffer>,
        HashMap<String, HashMap<String, String>>,
    );
    type RunType = AsyncRunType;

    async fn process(
        &mut self,
        transactions: TransactionContext<Vec<Transaction>>,
    ) -> Result<
        Option<
            TransactionContext<(
                HashMap<i64, Vec<NftMarketplaceActivity>>,
                Vec<CurrentNFTMarketplaceListing>,
                Vec<CurrentNFTMarketplaceTokenOffer>,
                Vec<CurrentNFTMarketplaceCollectionOffer>,
                HashMap<String, HashMap<String, String>>,
            )>,
        >,
        ProcessorError,
    > {
        let results = transactions
            .data
            .par_iter()
            .map(|transaction| {
                let event_remapper = self.event_remapper.clone();
                let resource_remapper = self.resource_remapper.clone();
                let (activities, listings, token_offers, collection_offers) =
                    event_remapper.remap_events(transaction.clone())?;

                let resource_updates = resource_remapper.remap_resources(transaction.clone())?;

                Ok((
                    activities,
                    listings,
                    token_offers,
                    collection_offers,
                    resource_updates,
                ))
            })
            .collect::<anyhow::Result<Vec<_>>>()
            .map_err(|e| ProcessorError::ProcessError {
                message: format!("{:#}", e),
            })?;

        let (
            mut all_activities,
            mut all_listings,
            mut all_token_offers,
            mut all_collection_offers,
            mut all_resource_updates,
        ) = (
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            HashMap::<String, HashMap<String, String>>::new(),
        );

        for (activities, listings, token_offers, collection_offers, resource_updates) in results {
            all_activities.extend(activities);
            all_listings.extend(listings);
            all_token_offers.extend(token_offers);
            all_collection_offers.extend(collection_offers);

            // Merge resource_updates by key
            resource_updates.into_iter().for_each(|(key, value_map)| {
                all_resource_updates
                    .entry(key)
                    .or_default()
                    .extend(value_map);
            });
        }

        // iterate activities and crete a map of key txn_veesrion to activity, so it can be used later to be updated during reduction step
        let mut activities_map: HashMap<i64, Vec<NftMarketplaceActivity>> = HashMap::new();
        for activity in all_activities {
            activities_map
                .entry(activity.txn_version)
                .or_default()
                .push(activity);
        }

        Ok(Some(TransactionContext {
            data: (
                activities_map,
                all_listings,
                all_token_offers,
                all_collection_offers,
                all_resource_updates,
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

// // Store activity in the map only if it has a token_data_id
// // This ensures we can later match resources to activities
// // if it's empty it means it's v1
// if activity.token_data_id.is_none() {
//     activity.token_data_id = match generate_token_data_id(
//         activity.creator_address.clone(),
//         activity.collection_name.clone(),
//         activity.token_name.clone(),
//     ) {
//         Some(token_data_id) => Some(token_data_id),
//         None => {
//             debug!(
//                 "Failed to generate token data id for activity: {:#?}",
//                 activity
//             );
//             None
//         },
//     }
// }

// // Store activity in the map only if it has a collection_id
// // This ensures we can later match resources to activities
// if activity.collection_id.is_none() {
//     // only if we can generate a collection id
//     activity.collection_id = match generate_collection_id(
//         activity.creator_address.clone(),
//         activity.collection_name.clone(),
//     ) {
//         Some(collection_id) => Some(collection_id),
//         None => {
//             // V2 events may be missing data to generate collection id
//             debug!(
//                 "Failed to generate collection id for activity: {:#?}",
//                 activity
//             );
//             None
//         },
//     };
// }
