// @generated automatically by Diesel CLI.

diesel::table! {
    current_nft_marketplace_collection_offers (collection_offer_id) {
        #[max_length = 128]
        collection_offer_id -> Varchar,
        #[max_length = 66]
        collection_id -> Nullable<Varchar>,
        #[max_length = 66]
        fee_schedule_id -> Nullable<Varchar>,
        #[max_length = 66]
        buyer -> Nullable<Varchar>,
        price -> Nullable<Int8>,
        remaining_token_amount -> Nullable<Int8>,
        is_deleted -> Bool,
        #[max_length = 66]
        token_standard -> Nullable<Varchar>,
        #[max_length = 66]
        coin_type -> Nullable<Varchar>,
        marketplace -> Varchar,
        contract_address -> Varchar,
        entry_function_id_str -> Varchar,
        last_transaction_version -> Int8,
        last_transaction_timestamp -> Timestamp,
    }
}

diesel::table! {
    current_nft_marketplace_listings (listing_id) {
        #[max_length = 128]
        listing_id -> Varchar,
        #[max_length = 66]
        token_data_id -> Varchar,
        #[max_length = 66]
        collection_id -> Nullable<Varchar>,
        #[max_length = 66]
        fee_schedule_id -> Nullable<Varchar>,
        #[max_length = 66]
        seller -> Nullable<Varchar>,
        price -> Nullable<Int8>,
        token_amount -> Nullable<Int8>,
        #[max_length = 66]
        token_standard -> Nullable<Varchar>,
        is_deleted -> Bool,
        #[max_length = 66]
        coin_type -> Nullable<Varchar>,
        marketplace -> Varchar,
        contract_address -> Varchar,
        entry_function_id_str -> Varchar,
        last_transaction_version -> Int8,
        last_transaction_timestamp -> Timestamp,
    }
}

diesel::table! {
    current_nft_marketplace_token_offers (offer_id) {
        #[max_length = 128]
        offer_id -> Varchar,
        #[max_length = 66]
        token_data_id -> Varchar,
        #[max_length = 66]
        collection_id -> Nullable<Varchar>,
        #[max_length = 66]
        fee_schedule_id -> Nullable<Varchar>,
        #[max_length = 66]
        buyer -> Nullable<Varchar>,
        price -> Nullable<Int8>,
        token_amount -> Nullable<Int8>,
        token_name -> Nullable<Varchar>,
        is_deleted -> Bool,
        #[max_length = 66]
        token_standard -> Nullable<Varchar>,
        #[max_length = 66]
        coin_type -> Nullable<Varchar>,
        marketplace -> Varchar,
        contract_address -> Varchar,
        entry_function_id_str -> Varchar,
        last_transaction_version -> Int8,
        last_transaction_timestamp -> Timestamp,
    }
}

diesel::table! {
    nft_marketplace_activities (txn_version, index) {
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
        #[max_length = 66]
        token_standard -> Nullable<Varchar>,
        price -> Nullable<Int8>,
        token_amount -> Nullable<Int8>,
        #[max_length = 66]
        buyer -> Nullable<Varchar>,
        #[max_length = 66]
        seller -> Nullable<Varchar>,
        deadline -> Nullable<Varchar>,
        json_data -> Jsonb,
        marketplace -> Varchar,
        contract_address -> Varchar,
        entry_function_id_str -> Nullable<Varchar>,
        block_timestamp -> Timestamp,
        #[max_length = 66]
        fee_schedule_id -> Nullable<Varchar>,
        #[max_length = 66]
        coin_type -> Nullable<Varchar>,
        #[max_length = 128]
        listing_id -> Nullable<Varchar>,
        #[max_length = 128]
        offer_id -> Nullable<Varchar>,
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
    current_nft_marketplace_collection_offers,
    current_nft_marketplace_listings,
    current_nft_marketplace_token_offers,
    nft_marketplace_activities,
    processor_status,
);
