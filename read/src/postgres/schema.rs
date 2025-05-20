// @generated automatically by Diesel CLI.

diesel::table! {
    backfill_processor_status (backfill_alias) {
        #[max_length = 100]
        backfill_alias -> Varchar,
        #[max_length = 50]
        backfill_status -> Varchar,
        last_success_version -> Int8,
        last_updated -> Timestamp,
        last_transaction_timestamp -> Nullable<Timestamp>,
        backfill_start_version -> Int8,
        backfill_end_version -> Nullable<Int8>,
    }
}

diesel::table! {
    current_nft_marketplace_collection_offers (collection_offer_id, marketplace) {
        #[max_length = 128]
        collection_offer_id -> Varchar,
        #[max_length = 66]
        collection_id -> Nullable<Varchar>,
        #[max_length = 66]
        buyer -> Varchar,
        price -> Int8,
        remaining_token_amount -> Nullable<Int8>,
        is_deleted -> Bool,
        marketplace -> Varchar,
        contract_address -> Varchar,
        last_transaction_version -> Int8,
        last_transaction_timestamp -> Timestamp,
        standard_event_type -> Varchar,
        #[max_length = 66]
        token_data_id -> Nullable<Varchar>,
        expiration_time -> Nullable<Timestamp>,
        bid_key -> Nullable<Int8>,
    }
}

diesel::table! {
    current_nft_marketplace_listings (token_data_id, marketplace) {
        #[max_length = 66]
        token_data_id -> Varchar,
        #[max_length = 128]
        listing_id -> Nullable<Varchar>,
        #[max_length = 66]
        collection_id -> Nullable<Varchar>,
        #[max_length = 66]
        seller -> Nullable<Varchar>,
        price -> Int8,
        token_amount -> Nullable<Int8>,
        token_name -> Nullable<Varchar>,
        is_deleted -> Bool,
        marketplace -> Varchar,
        contract_address -> Varchar,
        last_transaction_version -> Int8,
        last_transaction_timestamp -> Timestamp,
        standard_event_type -> Varchar,
    }
}

diesel::table! {
    current_nft_marketplace_token_offers (token_data_id, buyer, marketplace) {
        #[max_length = 66]
        token_data_id -> Varchar,
        #[max_length = 128]
        offer_id -> Nullable<Varchar>,
        marketplace -> Varchar,
        #[max_length = 66]
        collection_id -> Nullable<Varchar>,
        #[max_length = 66]
        buyer -> Varchar,
        price -> Int8,
        token_amount -> Nullable<Int8>,
        token_name -> Nullable<Varchar>,
        is_deleted -> Bool,
        contract_address -> Varchar,
        last_transaction_version -> Int8,
        last_transaction_timestamp -> Timestamp,
        standard_event_type -> Varchar,
        expiration_time -> Nullable<Timestamp>,
        bid_key -> Nullable<Int8>,
    }
}

diesel::table! {
    nft_marketplace_activities (txn_version, index, marketplace) {
        txn_version -> Int8,
        index -> Int8,
        raw_event_type -> Varchar,
        standard_event_type -> Varchar,
        #[max_length = 66]
        creator_address -> Nullable<Varchar>,
        #[max_length = 66]
        collection_id -> Nullable<Varchar>,
        collection_name -> Nullable<Varchar>,
        #[max_length = 66]
        token_data_id -> Nullable<Varchar>,
        token_name -> Nullable<Varchar>,
        price -> Int8,
        token_amount -> Nullable<Int8>,
        #[max_length = 66]
        buyer -> Nullable<Varchar>,
        #[max_length = 66]
        seller -> Nullable<Varchar>,
        #[max_length = 128]
        listing_id -> Nullable<Varchar>,
        #[max_length = 128]
        offer_id -> Nullable<Varchar>,
        json_data -> Jsonb,
        marketplace -> Varchar,
        contract_address -> Varchar,
        block_timestamp -> Timestamp,
        expiration_time -> Nullable<Timestamp>,
        bid_key -> Nullable<Int8>,
    }
}

diesel::table! {
    processor_status (processor) {
        #[max_length = 100]
        processor -> Varchar,
        last_success_version -> Int8,
        last_updated -> Timestamp,
        last_transaction_timestamp -> Nullable<Timestamp>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    backfill_processor_status,
    current_nft_marketplace_collection_offers,
    current_nft_marketplace_listings,
    current_nft_marketplace_token_offers,
    nft_marketplace_activities,
    processor_status,
);
