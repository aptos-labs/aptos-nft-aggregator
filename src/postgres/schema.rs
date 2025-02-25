// @generated automatically by Diesel CLI.

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
        price -> Nullable<Numeric>,
        token_amount -> Nullable<Numeric>,
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

diesel::allow_tables_to_appear_in_same_query!(nft_marketplace_activities, processor_status,);
