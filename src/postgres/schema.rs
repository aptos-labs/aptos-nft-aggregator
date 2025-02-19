// @generated automatically by Diesel CLI.

diesel::table! {
    current_nft_marketplace_bids (token_data_id, buyer, price) {
        #[max_length = 66]
        token_data_id -> Varchar,
        #[max_length = 66]
        buyer -> Varchar,
        price -> Numeric,
        #[max_length = 66]
        creator_address -> Nullable<Varchar>,
        token_amount -> Nullable<Numeric>,
        token_name -> Nullable<Varchar>,
        collection_name -> Nullable<Varchar>,
        #[max_length = 66]
        collection_id -> Nullable<Varchar>,
        marketplace -> Varchar,
        #[max_length = 66]
        contract_address -> Varchar,
        entry_function_id_str -> Varchar,
        is_deleted -> Bool,
        last_transaction_version -> Int8,
        last_transaction_timestamp -> Timestamp,
    }
}

diesel::table! {
    current_nft_marketplace_collection_bids (collection_id, buyer, price) {
        #[max_length = 66]
        collection_id -> Varchar,
        #[max_length = 66]
        buyer -> Varchar,
        price -> Numeric,
        #[max_length = 66]
        creator_address -> Nullable<Varchar>,
        token_amount -> Nullable<Numeric>,
        collection_name -> Nullable<Varchar>,
        marketplace -> Varchar,
        coin_type -> Nullable<Varchar>,
        #[max_length = 66]
        contract_address -> Varchar,
        entry_function_id_str -> Varchar,
        expiration_time -> Nullable<Int8>,
        is_deleted -> Bool,
        last_transaction_version -> Int8,
        last_transaction_timestamp -> Timestamp,
    }
}

diesel::table! {
    current_nft_marketplace_listings (token_data_id) {
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
        #[max_length = 66]
        contract_address -> Varchar,
        entry_function_id_str -> Varchar,
        #[max_length = 66]
        event_type -> Nullable<Varchar>,
        last_transaction_version -> Int8,
        last_transaction_timestamp -> Timestamp,
        inserted_at -> Timestamp,
    }
}

diesel::table! {
    nft_marketplace_activities (txn_version, event_index) {
        txn_version -> Int8,
        event_index -> Int8,
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
        price -> Nullable<Numeric>,
        token_amount -> Nullable<Numeric>,
        #[max_length = 66]
        token_standard -> Nullable<Varchar>,
        #[max_length = 66]
        buyer -> Nullable<Varchar>,
        #[max_length = 66]
        seller -> Nullable<Varchar>,
        json_data -> Jsonb,
        marketplace -> Varchar,
        #[max_length = 66]
        contract_address -> Varchar,
        entry_function_id_str -> Varchar,
        transaction_timestamp -> Timestamp,
    }
}

diesel::table! {
    nft_marketplace_bids (transaction_version, event_index) {
        transaction_version -> Int8,
        event_index -> Int8,
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
        #[max_length = 66]
        contract_address -> Varchar,
        entry_function_id_str -> Varchar,
        event_type -> Varchar,
        transaction_timestamp -> Timestamp,
    }
}

diesel::table! {
    nft_marketplace_collection_bids (transaction_version, event_index) {
        transaction_version -> Int8,
        event_index -> Int8,
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
        #[max_length = 66]
        contract_address -> Varchar,
        entry_function_id_str -> Varchar,
        event_type -> Varchar,
        transaction_timestamp -> Timestamp,
    }
}

diesel::table! {
    nft_marketplace_listings (transaction_version) {
        transaction_version -> Int8,
        #[max_length = 66]
        creator_address -> Nullable<Varchar>,
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
        #[max_length = 66]
        contract_address -> Varchar,
        entry_function_id_str -> Varchar,
        #[max_length = 66]
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
