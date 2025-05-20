use crate::{
    models::nft_models::{
        CurrentNFTMarketplaceCollectionOffer, CurrentNFTMarketplaceListing,
        CurrentNFTMarketplaceTokenOffer, NftMarketplaceActivity,
    },
    postgres::postgres_utils::{execute_in_chunks, ArcDbPool},
    schema,
};
use ahash::HashMap;
use aptos_indexer_processor_sdk::{
    traits::{async_step::AsyncRunType, AsyncStep, NamedStep, Processable},
    types::transaction_context::TransactionContext,
    utils::errors::ProcessorError,
};
use diesel::{
    pg::{upsert::excluded, Pg},
    query_builder::QueryFragment,
    query_dsl::methods::FilterDsl,
    ExpressionMethods,
};
use tonic::async_trait;

pub struct DBWritingStep {
    pub db_pool: ArcDbPool,
}

impl DBWritingStep {
    pub fn new(db_pool: ArcDbPool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl Processable for DBWritingStep {
    type Input = (
        Vec<NftMarketplaceActivity>,
        Vec<CurrentNFTMarketplaceListing>,
        Vec<CurrentNFTMarketplaceTokenOffer>,
        Vec<CurrentNFTMarketplaceCollectionOffer>,
    );
    type Output = ();
    type RunType = AsyncRunType;

    async fn process(
        &mut self,
        input: TransactionContext<(
            Vec<NftMarketplaceActivity>,
            Vec<CurrentNFTMarketplaceListing>,
            Vec<CurrentNFTMarketplaceTokenOffer>,
            Vec<CurrentNFTMarketplaceCollectionOffer>,
        )>,
    ) -> Result<Option<TransactionContext<()>>, ProcessorError> {
        let (activities, listings, token_offers, collection_offers) = input.data;

        let mut deduped_activities: Vec<NftMarketplaceActivity> = activities
            .into_iter()
            .map(|activity| {
                (
                    (
                        activity.txn_version,
                        activity.index,
                        activity.marketplace.clone(),
                    ),
                    activity,
                )
            })
            .collect::<HashMap<_, _>>()
            .into_values()
            .collect();

        deduped_activities.sort_by(|a, b| {
            a.txn_version
                .cmp(&b.txn_version)
                .then(a.index.cmp(&b.index))
        });

        let mut deduped_listings: Vec<CurrentNFTMarketplaceListing> = listings
            .into_iter()
            .map(|listing| {
                let key = (listing.token_data_id.clone(), listing.marketplace.clone());
                (key, listing)
            })
            .collect::<HashMap<_, _>>()
            .into_values()
            .collect();
        deduped_listings.sort_by(|a, b| a.token_data_id.cmp(&b.token_data_id));

        let mut deduped_token_offers: Vec<CurrentNFTMarketplaceTokenOffer> = token_offers
            .into_iter()
            .map(|offer| {
                let key = (
                    offer.token_data_id.clone(),
                    offer.buyer.clone(),
                    offer.marketplace.clone(),
                );
                (key, offer)
            })
            .collect::<HashMap<_, _>>()
            .into_values()
            .collect();

        deduped_token_offers.sort_by(|a, b: &CurrentNFTMarketplaceTokenOffer| {
            let key_a = (&a.token_data_id, &a.buyer);
            let key_b = (&b.token_data_id, &b.buyer);
            key_a.cmp(&key_b)
        });

        // Deduplicate collection offers using offer_id
        let mut deduped_collection_offers: Vec<CurrentNFTMarketplaceCollectionOffer> =
            collection_offers
                .into_iter()
                .map(|offer| {
                    let key = (offer.collection_offer_id.clone(), offer.marketplace.clone());
                    (key, offer)
                })
                .collect::<HashMap<_, _>>()
                .into_values()
                .collect();

        deduped_collection_offers.sort_by(|a, b| a.collection_offer_id.cmp(&b.collection_offer_id));

        // Execute DB operations with sorted, deduplicated data
        let activities_result = execute_in_chunks(
            self.db_pool.clone(),
            insert_nft_marketplace_activities,
            &deduped_activities,
            200,
        );

        let listings_result = execute_in_chunks(
            self.db_pool.clone(),
            insert_current_nft_marketplace_listings,
            &deduped_listings,
            200,
        );

        let token_offers_result = execute_in_chunks(
            self.db_pool.clone(),
            insert_current_nft_marketplace_token_offers,
            &deduped_token_offers,
            200,
        );

        let collection_offers_result = execute_in_chunks(
            self.db_pool.clone(),
            insert_current_nft_marketplace_collection_offers,
            &deduped_collection_offers,
            200,
        );

        let (activities_result, listings_result, token_offers_result, collection_offers_result) = tokio::join!(
            activities_result,
            listings_result,
            token_offers_result,
            collection_offers_result
        );

        for result in [
            activities_result,
            listings_result,
            token_offers_result,
            collection_offers_result,
        ] {
            match result {
                Ok(_) => (),
                Err(e) => {
                    return Err(ProcessorError::DBStoreError {
                        message: format!("Failed to store: {e:?}"),
                        query: None,
                    })
                },
            }
        }

        Ok(Some(TransactionContext {
            data: (),
            metadata: input.metadata,
        }))
    }
}

impl AsyncStep for DBWritingStep {}

impl NamedStep for DBWritingStep {
    fn name(&self) -> String {
        "DBWritingStep".to_string()
    }
}

pub fn insert_nft_marketplace_activities(
    items_to_insert: Vec<NftMarketplaceActivity>,
) -> impl QueryFragment<Pg> + diesel::query_builder::QueryId + Send {
    use crate::schema::nft_marketplace_activities::dsl::*;

    diesel::insert_into(schema::nft_marketplace_activities::table)
        .values(items_to_insert)
        .on_conflict((txn_version, index, marketplace))
        .do_nothing()
}

pub fn insert_current_nft_marketplace_listings(
    items_to_insert: Vec<CurrentNFTMarketplaceListing>,
) -> impl QueryFragment<Pg> + diesel::query_builder::QueryId + Send {
    use crate::schema::current_nft_marketplace_listings::dsl::*;

    diesel::insert_into(schema::current_nft_marketplace_listings::table)
        .values(items_to_insert)
        .on_conflict((token_data_id, marketplace))
        .do_update()
        .set((
            listing_id.eq(excluded(listing_id)),
            collection_id.eq(excluded(collection_id)),
            seller.eq(excluded(seller)),
            price.eq(excluded(price)),
            token_amount.eq(excluded(token_amount)),
            token_name.eq(excluded(token_name)),
            is_deleted.eq(excluded(is_deleted)),
            contract_address.eq(excluded(contract_address)),
            last_transaction_timestamp.eq(excluded(last_transaction_timestamp)),
            last_transaction_version.eq(excluded(last_transaction_version)),
            standard_event_type.eq(excluded(standard_event_type)),
        ))
        .filter(last_transaction_version.le(excluded(last_transaction_version)))
}

pub fn insert_current_nft_marketplace_token_offers(
    items_to_insert: Vec<CurrentNFTMarketplaceTokenOffer>,
) -> impl QueryFragment<Pg> + diesel::query_builder::QueryId + Send {
    use crate::schema::current_nft_marketplace_token_offers::dsl::*;
    diesel::insert_into(schema::current_nft_marketplace_token_offers::table)
        .values(items_to_insert)
        .on_conflict((token_data_id, buyer, marketplace))
        .do_update()
        .set((
            offer_id.eq(excluded(offer_id)),
            collection_id.eq(excluded(collection_id)),
            buyer.eq(excluded(buyer)),
            price.eq(excluded(price)),
            token_amount.eq(excluded(token_amount)),
            token_name.eq(excluded(token_name)),
            is_deleted.eq(excluded(is_deleted)),
            contract_address.eq(excluded(contract_address)),
            last_transaction_version.eq(excluded(last_transaction_version)),
            last_transaction_timestamp.eq(excluded(last_transaction_timestamp)),
            standard_event_type.eq(excluded(standard_event_type)),
            bid_key.eq(excluded(bid_key)),
        ))
        .filter(last_transaction_version.le(excluded(last_transaction_version)))
}

pub fn insert_current_nft_marketplace_collection_offers(
    items_to_insert: Vec<CurrentNFTMarketplaceCollectionOffer>,
) -> impl QueryFragment<Pg> + diesel::query_builder::QueryId + Send {
    use crate::schema::current_nft_marketplace_collection_offers::dsl::*;

    diesel::insert_into(schema::current_nft_marketplace_collection_offers::table)
        .values(items_to_insert)
        .on_conflict((collection_offer_id, marketplace))
        .do_update()
        .set((
            collection_id.eq(excluded(collection_id)),
            buyer.eq(excluded(buyer)),
            price.eq(excluded(price)),
            remaining_token_amount.eq(excluded(remaining_token_amount)),
            is_deleted.eq(excluded(is_deleted)),
            contract_address.eq(excluded(contract_address)),
            last_transaction_version.eq(excluded(last_transaction_version)),
            last_transaction_timestamp.eq(excluded(last_transaction_timestamp)),
            token_data_id.eq(excluded(token_data_id)),
            standard_event_type.eq(excluded(standard_event_type)),
            bid_key.eq(excluded(bid_key)),
        ))
        .filter(last_transaction_version.le(excluded(last_transaction_version)))
}
