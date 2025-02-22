// @generated automatically by Diesel CLI.

diesel::table! {
    current_nft_marketplace_bids (offer_id, token_data_id) {
        #[max_length = 66]
        offer_id -> Varchar,
        #[max_length = 66]
        token_data_id -> Varchar,
        index -> Int8,
        #[max_length = 66]
        buyer -> Nullable<Varchar>,
        price -> Nullable<Numeric>,
        #[max_length = 66]
        creator_address -> Nullable<Varchar>,
        token_amount -> Nullable<Numeric>,
        token_name -> Nullable<Varchar>,
        collection_name -> Nullable<Varchar>,
        #[max_length = 66]
        collection_id -> Nullable<Varchar>,
        marketplace -> Varchar,
        contract_address -> Varchar,
        entry_function_id_str -> Nullable<Varchar>,
        is_deleted -> Bool,
        last_transaction_version -> Int8,
        last_transaction_timestamp -> Timestamp,
    }
}

diesel::table! {
    current_nft_marketplace_collection_bids (offer_id, collection_id) {
        #[max_length = 66]
        collection_id -> Varchar,
        index -> Int8,
        #[max_length = 66]
        offer_id -> Varchar,
        #[max_length = 66]
        buyer -> Nullable<Varchar>,
        price -> Nullable<Numeric>,
        #[max_length = 66]
        creator_address -> Nullable<Varchar>,
        token_amount -> Nullable<Numeric>,
        collection_name -> Nullable<Varchar>,
        marketplace -> Varchar,
        coin_type -> Nullable<Varchar>,
        contract_address -> Varchar,
        entry_function_id_str -> Nullable<Varchar>,
        expiration_time -> Nullable<Int8>,
        is_deleted -> Bool,
        last_transaction_version -> Int8,
        last_transaction_timestamp -> Timestamp,
    }
}

diesel::table! {
    current_nft_marketplace_listings (listing_id, token_data_id, index) {
        #[max_length = 66]
        listing_id -> Varchar,
        index -> Int8,
        #[max_length = 66]
        token_data_id -> Varchar,
        #[max_length = 66]
        creator_address -> Nullable<Varchar>,
        token_name -> Nullable<Varchar>,
        collection_name -> Nullable<Varchar>,
        #[max_length = 66]
        collection_id -> Nullable<Varchar>,
        price -> Nullable<Numeric>,
        token_amount -> Nullable<Numeric>,
        #[max_length = 66]
        seller -> Nullable<Varchar>,
        #[max_length = 66]
        token_standard -> Nullable<Varchar>,
        is_deleted -> Bool,
        marketplace -> Varchar,
        contract_address -> Varchar,
        entry_function_id_str -> Nullable<Varchar>,
        event_type -> Nullable<Varchar>,
        last_transaction_version -> Int8,
        last_transaction_timestamp -> Timestamp,
        inserted_at -> Timestamp,
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
        offer_or_listing_id -> Nullable<Varchar>,
        #[max_length = 66]
        token_data_id -> Nullable<Varchar>,
        token_name -> Nullable<Varchar>,
        #[max_length = 66]
        token_standard -> Nullable<Varchar>,
        price -> Nullable<Numeric>,
        token_amount -> Nullable<Numeric>,
        #[max_length = 66]
        buyer -> Nullable<Varchar>,
        #[max_length = 66]
        seller -> Nullable<Varchar>,
        json_data -> Jsonb,
        marketplace -> Varchar,
        contract_address -> Varchar,
        entry_function_id_str -> Nullable<Varchar>,
        transaction_timestamp -> Timestamp,
    }
}

diesel::table! {
    nft_marketplace_bids (txn_version, index) {
        txn_version -> Int8,
        index -> Int8,
        #[max_length = 66]
        offer_id -> Nullable<Varchar>,
        #[max_length = 66]
        token_data_id -> Nullable<Varchar>,
        #[max_length = 66]
        buyer -> Nullable<Varchar>,
        price -> Nullable<Numeric>,
        #[max_length = 66]
        creator_address -> Nullable<Varchar>,
        token_amount -> Nullable<Numeric>,
        token_name -> Nullable<Varchar>,
        collection_name -> Nullable<Varchar>,
        #[max_length = 66]
        collection_id -> Nullable<Varchar>,
        marketplace -> Varchar,
        contract_address -> Varchar,
        entry_function_id_str -> Nullable<Varchar>,
        event_type -> Varchar,
        transaction_timestamp -> Timestamp,
    }
}

diesel::table! {
    nft_marketplace_collection_bids (txn_version, index) {
        txn_version -> Int8,
        index -> Int8,
        #[max_length = 66]
        offer_id -> Nullable<Varchar>,
        #[max_length = 66]
        creator_address -> Nullable<Varchar>,
        collection_name -> Nullable<Varchar>,
        #[max_length = 66]
        collection_id -> Nullable<Varchar>,
        price -> Nullable<Numeric>,
        token_amount -> Nullable<Numeric>,
        #[max_length = 66]
        buyer -> Nullable<Varchar>,
        #[max_length = 66]
        seller -> Nullable<Varchar>,
        marketplace -> Varchar,
        contract_address -> Varchar,
        entry_function_id_str -> Nullable<Varchar>,
        event_type -> Varchar,
        transaction_timestamp -> Timestamp,
    }
}

diesel::table! {
    nft_marketplace_listings (txn_version, index) {
        #[max_length = 66]
        listing_id -> Nullable<Varchar>,
        txn_version -> Int8,
        index -> Int8,
        #[max_length = 66]
        creator_address -> Nullable<Varchar>,
        standard_event_type -> Nullable<Varchar>,
        token_name -> Nullable<Varchar>,
        #[max_length = 66]
        token_data_id -> Nullable<Varchar>,
        collection_name -> Nullable<Varchar>,
        #[max_length = 66]
        collection_id -> Nullable<Varchar>,
        price -> Nullable<Numeric>,
        token_amount -> Nullable<Numeric>,
        #[max_length = 66]
        seller -> Nullable<Varchar>,
        #[max_length = 66]
        token_standard -> Nullable<Varchar>,
        marketplace -> Varchar,
        contract_address -> Varchar,
        entry_function_id_str -> Nullable<Varchar>,
        event_type -> Nullable<Varchar>,
        transaction_timestamp -> Timestamp,
        inserted_at -> Timestamp,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    current_nft_marketplace_bids,
    current_nft_marketplace_collection_bids,
    current_nft_marketplace_listings,
    nft_marketplace_activities,
    nft_marketplace_bids,
    nft_marketplace_collection_bids,
    nft_marketplace_listings,
);
