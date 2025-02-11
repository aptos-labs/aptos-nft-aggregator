// @generated automatically by Diesel CLI.

diesel::table! {
    ledger_infos (chain_id) {
        chain_id -> Int8,
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

diesel::table! {
    nft_activities (id) {
        id -> Int8,
        account_address -> Varchar,
        transaction_version -> Int8,
    }
}

diesel::allow_tables_to_appear_in_same_query!(ledger_infos, processor_status, nft_activities,);
