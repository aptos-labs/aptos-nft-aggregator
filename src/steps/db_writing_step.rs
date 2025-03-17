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

        // Deduplicate and reduce data
        let mut deduped_activities: Vec<NftMarketplaceActivity> = activities
            .into_iter()
            .map(|activity| ((activity.txn_version, activity.index), activity))
            .collect::<HashMap<_, _>>()
            .into_values()
            .collect();

        // Sort activities by primary key to prevent deadlocks
        deduped_activities.sort_by(|a, b| {
            a.txn_version
                .cmp(&b.txn_version)
                .then(a.index.cmp(&b.index))
        });

        // Deduplicate listings using token_data_id
        let mut deduped_listings: Vec<CurrentNFTMarketplaceListing> = listings
            .into_iter()
            .map(|listing| {
                let key = listing.token_data_id.clone();
                (key, listing)
            })
            .collect::<HashMap<_, _>>()
            .into_values()
            .collect();
        deduped_listings.sort_by(|a, b| a.token_data_id.cmp(&b.token_data_id));

        // Deduplicate token offers using token_data_id and buyer
        let mut deduped_token_offers: Vec<CurrentNFTMarketplaceTokenOffer> = token_offers
            .into_iter()
            .map(|offer| {
                let key = (offer.token_data_id.clone(), offer.buyer.clone());
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
                    let key = offer.collection_offer_id.clone();
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
                        message: format!("Failed to store : {:?}", e),
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
) -> (
    impl QueryFragment<Pg> + diesel::query_builder::QueryId + Send,
    Option<&'static str>,
) {
    use crate::schema::nft_marketplace_activities::dsl::*;
    (
        diesel::insert_into(schema::nft_marketplace_activities::table)
            .values(items_to_insert)
            .on_conflict((txn_version, index))
            .do_update()
            .set((
                token_amount.eq(excluded(token_amount)),
                buyer.eq(excluded(buyer)),
                seller.eq(excluded(seller)),
                expiration_time.eq(excluded(expiration_time)),
                listing_id.eq(excluded(listing_id)),
                offer_id.eq(excluded(offer_id)),
                raw_event_type.eq(excluded(raw_event_type)),
                standard_event_type.eq(excluded(standard_event_type)),
                creator_address.eq(excluded(creator_address)),
                collection_id.eq(excluded(collection_id)),
                collection_name.eq(excluded(collection_name)),
                token_data_id.eq(excluded(token_data_id)),
                token_name.eq(excluded(token_name)),
                token_standard.eq(excluded(token_standard)),
                price.eq(excluded(price)),
            )),
        None,
    )
}

pub fn insert_current_nft_marketplace_listings(
    items_to_insert: Vec<CurrentNFTMarketplaceListing>,
) -> (
    impl QueryFragment<Pg> + diesel::query_builder::QueryId + Send,
    Option<&'static str>,
) {
    use crate::schema::current_nft_marketplace_listings::dsl::*;

    (
        diesel::insert_into(schema::current_nft_marketplace_listings::table)
            .values(items_to_insert)
            .on_conflict(token_data_id)
            .do_update()
            .set((
                is_deleted.eq(excluded(is_deleted)),
                last_transaction_timestamp.eq(excluded(last_transaction_timestamp)),
                token_amount.eq(excluded(token_amount)),
                last_transaction_version.eq(excluded(last_transaction_version)),
                price.eq(excluded(price)),
            )),
        Some(" WHERE current_nft_marketplace_listings.last_transaction_timestamp <= excluded.last_transaction_timestamp "),
    )
}

pub fn insert_current_nft_marketplace_token_offers(
    items_to_insert: Vec<CurrentNFTMarketplaceTokenOffer>,
) -> (
    impl QueryFragment<Pg> + diesel::query_builder::QueryId + Send,
    Option<&'static str>,
) {
    use crate::schema::current_nft_marketplace_token_offers::dsl::*;
    (
        diesel::insert_into(schema::current_nft_marketplace_token_offers::table)
            .values(items_to_insert)
            .on_conflict((token_data_id, buyer))
            .do_update()
            .set((
                is_deleted.eq(excluded(is_deleted)),
                last_transaction_timestamp.eq(excluded(last_transaction_timestamp)),
                token_amount.eq(excluded(token_amount)),
                last_transaction_version.eq(excluded(last_transaction_version)),
                price.eq(excluded(price)),
            )),
        Some(" WHERE current_nft_marketplace_token_offers.last_transaction_version < excluded.last_transaction_version "),
    )
}

pub fn insert_current_nft_marketplace_collection_offers(
    items_to_insert: Vec<CurrentNFTMarketplaceCollectionOffer>,
) -> (
    impl QueryFragment<Pg> + diesel::query_builder::QueryId + Send,
    Option<&'static str>,
) {
    use crate::schema::current_nft_marketplace_collection_offers::dsl::*;

    (
        diesel::insert_into(schema::current_nft_marketplace_collection_offers::table)
            .values(items_to_insert)
            .on_conflict(collection_offer_id)
            .do_update()
            .set((
                is_deleted.eq(excluded(is_deleted)),
                last_transaction_timestamp.eq(excluded(last_transaction_timestamp)),
                remaining_token_amount.eq(excluded(remaining_token_amount)),
                last_transaction_version.eq(excluded(last_transaction_version)),
                price.eq(excluded(price)),
            )),
        Some(" WHERE current_nft_marketplace_collection_offers.last_transaction_version < excluded.last_transaction_version "),
    )
}
