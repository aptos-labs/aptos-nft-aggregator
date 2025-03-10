use crate::{
    models::nft_models::{
        CurrentNFTMarketplaceCollectionOffer, CurrentNFTMarketplaceListing,
        CurrentNFTMarketplaceTokenOffer, NftMarketplaceActivity,
    },
    postgres::postgres_utils::{execute_in_chunks, ArcDbPool},
    schema,
};
use ahash::{HashMap, HashMapExt};
use aptos_indexer_processor_sdk::{
    traits::{async_step::AsyncRunType, AsyncStep, NamedStep, Processable},
    types::transaction_context::TransactionContext,
    utils::errors::ProcessorError,
};
use diesel::{
    pg::{upsert::excluded, Pg},
    prelude::*,
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

        // Batch DB lookup for missing listing IDs
        let missing_token_data_ids: Vec<_> = deduped_activities
            .iter()
            .filter(|a| {
                a.listing_id.is_none()
                    && a.token_data_id.is_some()
                    && matches!(
                        a.standard_event_type.as_str(),
                        "cancel_listing" | "fill_listing"
                    )
            })
            .filter_map(|a| a.token_data_id.clone())
            .collect();

        let mut db_conn = match self.db_pool.get().await {
            Ok(conn) => conn,
            Err(e) => {
                return Err(ProcessorError::DBStoreError {
                    message: format!("{:#}", e),
                    query: None,
                })
            },
        };

        let existing_listings: HashMap<String, String> = if !missing_token_data_ids.is_empty() {
            #[derive(QueryableByName)]
            struct ListingResult {
                #[diesel(sql_type = diesel::sql_types::Text)]
                token_data_id: String,
                #[diesel(sql_type = diesel::sql_types::Text)]
                listing_id: String,
            }

            let query = diesel::sql_query(
                "SELECT token_data_id, listing_id FROM current_nft_marketplace_listings 
                 WHERE token_data_id = ANY($1) AND is_deleted = false",
            )
            .bind::<diesel::sql_types::Array<diesel::sql_types::Text>, _>(&missing_token_data_ids);

            diesel_async::RunQueryDsl::load::<ListingResult>(query, &mut *db_conn)
                .await
                .map_err(|e| ProcessorError::DBStoreError {
                    message: format!("Failed to load existing listings: {}", e),
                    query: None,
                })?
                .into_iter()
                .map(|r| (r.token_data_id, r.listing_id))
                .collect()
        } else {
            HashMap::new()
        };

        // Update missing listing IDs
        for activity in &mut deduped_activities {
            if activity.listing_id.is_none() {
                if let Some(ref token_data_id_value) = activity.token_data_id {
                    if let Some(found_id) = existing_listings.get(token_data_id_value) {
                        activity.listing_id = Some(found_id.clone());
                    }
                }
            }
        }

        // Deduplicate listings using listing_id
        let mut deduped_listings: Vec<CurrentNFTMarketplaceListing> = listings
            .into_iter()
            .fold(
                HashMap::<String, CurrentNFTMarketplaceListing>::new(),
                |mut acc, listing| {
                    match acc.get(&listing.listing_id) {
                        Some(existing)
                            if existing.last_transaction_timestamp
                                <= listing.last_transaction_timestamp =>
                        {
                            acc.insert(listing.listing_id.clone(), listing);
                        },
                        None => {
                            acc.insert(listing.listing_id.clone(), listing);
                        },
                        _ => {},
                    }
                    acc
                },
            )
            .into_values()
            .collect();

        deduped_listings.sort_by(|a, b| a.listing_id.cmp(&b.listing_id));

        // Deduplicate token offers using offer_id
        let mut deduped_token_offers: Vec<CurrentNFTMarketplaceTokenOffer> = token_offers
            .into_iter()
            .fold(
                HashMap::<String, CurrentNFTMarketplaceTokenOffer>::new(),
                |mut acc, offer| {
                    match acc.get(&offer.offer_id) {
                        Some(existing)
                            if existing.last_transaction_version
                                < offer.last_transaction_version =>
                        {
                            acc.insert(offer.offer_id.clone(), offer);
                        },
                        None => {
                            acc.insert(offer.offer_id.clone(), offer);
                        },
                        _ => {},
                    }
                    acc
                },
            )
            .into_values()
            .collect();

        deduped_token_offers.sort_by(|a, b| a.offer_id.cmp(&b.offer_id));

        // Deduplicate collection offers using offer_id
        let mut deduped_collection_offers: Vec<CurrentNFTMarketplaceCollectionOffer> =
            collection_offers
                .into_iter()
                .fold(
                    HashMap::<String, CurrentNFTMarketplaceCollectionOffer>::new(),
                    |mut acc, offer| {
                        match acc.get(&offer.collection_offer_id) {
                            Some(existing)
                                if existing.last_transaction_version
                                    < offer.last_transaction_version =>
                            {
                                acc.insert(offer.collection_offer_id.clone(), offer);
                            },
                            None => {
                                acc.insert(offer.collection_offer_id.clone(), offer);
                            },
                            _ => {},
                        }
                        acc
                    },
                )
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
            .do_nothing(),
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
            .on_conflict(listing_id)
            .do_update()
            .set((
                is_deleted.eq(excluded(is_deleted)),
                last_transaction_timestamp.eq(excluded(last_transaction_timestamp)),
                token_amount.eq(excluded(token_amount)),
                last_transaction_version.eq(excluded(last_transaction_version)),
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
            .on_conflict(offer_id) // offer_id ?
            .do_update()
            .set((
                is_deleted.eq(excluded(is_deleted)),
                last_transaction_timestamp.eq(excluded(last_transaction_timestamp)),
                token_amount.eq(excluded(token_amount)),
                last_transaction_version.eq(excluded(last_transaction_version)),
                price.eq(excluded(price)),
                // index.eq(excluded(index)),
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
                contract_address.eq(excluded(contract_address)),
                coin_type.eq(excluded(coin_type)),
            )),
        Some(" WHERE current_nft_marketplace_collection_offers.last_transaction_version < excluded.last_transaction_version "),
    )
}

// pub fn fetch_existing_listings(
//     conn: &mut DbPoolConnection<'_>,
//     token_data_ids: &[String],
// ) -> diesel::QueryResult<Vec<(String, String)>> {
//     use crate::schema::current_nft_marketplace_listings::dsl::*;

//     current_nft_marketplace_listings
//         .filter(is_deleted.eq(false))
//         .filter(token_data_id.eq_any(token_data_ids))
//         .select((token_data_id, listing_id))
//         .load::<(String, String)>(conn)
// }
