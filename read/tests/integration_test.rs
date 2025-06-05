use aptos_indexer_processor_sdk::{
    postgres::subconfigs::postgres_config::PostgresConfig,
    testing_framework::{
        database::{PostgresTestDatabase, TestDatabase},
        sdk_test_context::{remove_inserted_at, SdkTestContext},
    },
    traits::processor_trait::ProcessorTrait,
};
use assert_json_diff::assert_json_eq;
use diesel::{pg::PgConnection, Connection};
use nft_aggregator::{
    config::{
        marketplace_config::NFTMarketplaceConfig,
        processor_mode::{ProcessorMode, TestingConfig},
        DbConfig, IndexerProcessorConfig,
    },
    models::nft_models::{
        CurrentNFTMarketplaceCollectionOffer, CurrentNFTMarketplaceListing,
        CurrentNFTMarketplaceTokenOffer, NftMarketplaceActivity,
    },
    processor::Processor,
};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

// Constants
pub const DEFAULT_OUTPUT_FOLDER: &str = "tests/expected_db_output_files";

fn load_data(conn: &mut PgConnection) -> anyhow::Result<HashMap<String, serde_json::Value>> {
    use diesel::prelude::*;
    use nft_aggregator::schema::{
        current_nft_marketplace_collection_offers, current_nft_marketplace_listings,
        current_nft_marketplace_token_offers, nft_marketplace_activities,
    };

    let mut result = HashMap::new();

    // Load activities
    let activities: Vec<NftMarketplaceActivity> = nft_marketplace_activities::table
        .order_by((
            nft_marketplace_activities::txn_version,
            nft_marketplace_activities::index,
            nft_marketplace_activities::marketplace,
        ))
        .load::<NftMarketplaceActivity>(conn)
        .map_err(|e| anyhow::anyhow!("Failed to load activities: {}", e))?;
    result.insert(
        "nft_marketplace_activities".to_string(),
        serde_json::to_value(activities)?,
    );

    // Load current listings
    let listings: Vec<CurrentNFTMarketplaceListing> = current_nft_marketplace_listings::table
        .order_by((
            current_nft_marketplace_listings::token_data_id,
            current_nft_marketplace_listings::marketplace,
        ))
        .load::<CurrentNFTMarketplaceListing>(conn)
        .map_err(|e| anyhow::anyhow!("Failed to load listings: {}", e))?;
    result.insert(
        "current_nft_marketplace_listings".to_string(),
        serde_json::to_value(listings)?,
    );

    // Load token offers
    let token_offers: Vec<CurrentNFTMarketplaceTokenOffer> =
        current_nft_marketplace_token_offers::table
            .order_by((
                current_nft_marketplace_token_offers::token_data_id,
                current_nft_marketplace_token_offers::buyer,
                current_nft_marketplace_token_offers::marketplace,
            ))
            .load::<CurrentNFTMarketplaceTokenOffer>(conn)
            .map_err(|e| anyhow::anyhow!("Failed to load token offers: {}", e))?;
    result.insert(
        "current_nft_marketplace_token_offers".to_string(),
        serde_json::to_value(token_offers)?,
    );

    // Load collection offers
    let collection_offers: Vec<CurrentNFTMarketplaceCollectionOffer> =
        current_nft_marketplace_collection_offers::table
            .order_by((
                current_nft_marketplace_collection_offers::collection_offer_id,
                current_nft_marketplace_collection_offers::marketplace,
            ))
            .load::<CurrentNFTMarketplaceCollectionOffer>(conn)
            .map_err(|e| anyhow::anyhow!("Failed to load collection offers: {}", e))?;
    result.insert(
        "current_nft_marketplace_collection_offers".to_string(),
        serde_json::to_value(collection_offers)?,
    );

    Ok(result)
}

// Configuration Helper Functions
fn build_test_nft_marketplace_config(marketplace_name: &str) -> NFTMarketplaceConfig {
    let config_path = PathBuf::from(format!(
        "tests/test_config/{marketplace_name}_test_marketplace_config.yaml"
    ));
    let config_str = std::fs::read_to_string(&config_path)
        .unwrap_or_else(|e| panic!("Failed to read config file: {e}"));
    serde_yaml::from_str(&config_str).unwrap_or_else(|e| panic!("Failed to parse config file: {e}"))
}

fn setup_nft_processor_config(
    test_context: &SdkTestContext,
    db_url: &str,
    marketplace_name: &str,
) -> (IndexerProcessorConfig, &'static str) {
    let transaction_stream_config = test_context.create_transaction_stream_config();
    let postgres_config = PostgresConfig {
        connection_string: db_url.to_string(),
        db_pool_size: 100,
    };

    let db_config = DbConfig::PostgresConfig(postgres_config);
    let processor_config = IndexerProcessorConfig {
        transaction_stream_config: transaction_stream_config.clone(),
        db_config,
        processor_mode: ProcessorMode::Testing(TestingConfig {
            override_starting_version: transaction_stream_config.starting_version.unwrap(),
            ending_version: transaction_stream_config.request_ending_version,
        }),
        nft_marketplace_config: build_test_nft_marketplace_config(marketplace_name),
    };

    let processor_name = processor_config.nft_marketplace_config.get_name();
    (processor_config, processor_name)
}

// Test Environment Setup Functions
pub async fn setup_test_environment(
    transactions: &[&[u8]],
) -> (PostgresTestDatabase, SdkTestContext) {
    let mut db = PostgresTestDatabase::new();
    db.setup().await.unwrap();

    let mut test_context = SdkTestContext::new(transactions);
    if test_context.init_mock_grpc().await.is_err() {
        panic!("Failed to initialize mock grpc");
    };

    (db, test_context)
}

// JSON Processing Helper Functions
pub fn read_and_parse_json(path: &str) -> anyhow::Result<Value> {
    match fs::read_to_string(path) {
        Ok(content) => match serde_json::from_str::<Value>(&content) {
            Ok(json) => Ok(json),
            Err(e) => {
                eprintln!("[ERROR] Failed to parse JSON at {path}: {e}");
                Err(anyhow::anyhow!("Failed to parse JSON: {e}"))
            },
        },
        Err(e) => {
            eprintln!("[ERROR] Failed to read file at {path}: {e}");
            Err(anyhow::anyhow!("Failed to read file: {e}"))
        },
    }
}

pub fn remove_transaction_timestamp(value: &mut Value) {
    if let Some(array) = value.as_array_mut() {
        for item in array.iter_mut() {
            if let Some(obj) = item.as_object_mut() {
                obj.remove("transaction_timestamp");
            }
        }
    }
}

pub fn validate_json(
    db_values: &mut HashMap<String, Value>,
    txn_version: u64,
    processor_name: &str,
    output_path: String,
    file_name: Option<String>,
) -> anyhow::Result<()> {
    for (table_name, db_value) in db_values.iter_mut() {
        let expected_file_path = match file_name.clone() {
            Some(custom_name) => PathBuf::from(&output_path)
                .join(processor_name)
                .join(custom_name.clone())
                .join(format!("{table_name}.json")),
            None => Path::new(&output_path)
                .join(processor_name)
                .join(txn_version.to_string())
                .join(format!("{table_name}.json")),
        };

        let mut expected_json = match read_and_parse_json(expected_file_path.to_str().unwrap()) {
            Ok(json) => json,
            Err(e) => {
                eprintln!(
                    "[ERROR] Error handling JSON for processor {processor_name} table {table_name} and transaction version {txn_version}: {e}"
                );
                panic!("Failed to read and parse JSON for table: {table_name}");
            },
        };

        remove_inserted_at(db_value);
        remove_transaction_timestamp(db_value);
        remove_inserted_at(&mut expected_json);
        remove_transaction_timestamp(&mut expected_json);
        println!("Diffing table: {table_name}, diffing version: {txn_version}");
        assert_json_eq!(db_value, expected_json);
    }
    Ok(())
}

// Processor Test Helper Functions
pub async fn run_processor_test<F>(
    test_context: &mut SdkTestContext,
    processor: impl ProcessorTrait,
    load_data: F,
    db_url: String,
    generate_file_flag: bool,
    output_path: String,
    custom_file_name: Option<String>,
) -> anyhow::Result<HashMap<String, Value>>
where
    F: Fn(&mut PgConnection) -> anyhow::Result<HashMap<String, Value>> + Send + Sync + 'static,
{
    let txn_versions: Vec<i64> = test_context
        .get_test_transaction_versions()
        .into_iter()
        .map(|v| v as i64)
        .collect();

    let db_values = test_context
        .run(
            &processor,
            generate_file_flag,
            output_path.clone(),
            custom_file_name,
            move || {
                let mut conn = PgConnection::establish(&db_url).unwrap_or_else(|e| {
                    eprintln!("[ERROR] Failed to establish DB connection: {e:?}");
                    panic!("Failed to establish DB connection: {e:?}");
                });

                let db_values = match load_data(&mut conn) {
                    Ok(db_data) => db_data,
                    Err(e) => {
                        eprintln!("[ERROR] Failed to load data {e:?}");
                        return Err(e);
                    },
                };

                if db_values.is_empty() {
                    eprintln!("[WARNING] No data found for versions: {txn_versions:?}");
                }

                Ok(db_values)
            },
        )
        .await?;
    Ok(db_values)
}

// Transaction Processing Helper Functions
async fn process_transactions(
    db: &mut PostgresTestDatabase,
    txns: &[&[u8]],
    transaction_name: &str,
    generate_flag: bool,
    output_path: &str,
    should_validate: bool,
    marketplace_name: &str,
) {
    let mut test_context = SdkTestContext::new(txns);
    if test_context.init_mock_grpc().await.is_err() {
        panic!("Failed to initialize mock grpc");
    }

    let db_url = db.get_db_url();
    let (processor_config, processor_name) =
        setup_nft_processor_config(&test_context, &db_url, marketplace_name);

    let nft_processor = Processor::new(processor_config)
        .await
        .expect("Failed to create NFTProcessor");

    match run_processor_test(
        &mut test_context,
        nft_processor,
        load_data,
        db_url,
        generate_flag,
        output_path.to_string(),
        Some(transaction_name.to_string()),
    )
    .await
    {
        Ok(mut db_value) => {
            if should_validate {
                let _ = validate_json(
                    &mut db_value,
                    test_context.get_request_start_version(),
                    processor_name,
                    output_path.to_string(),
                    Some(transaction_name.to_string()),
                );
            }
        },
        Err(e) => {
            panic!("Test failed on {transaction_name} due to processor error: {e}");
        },
    }
}

#[cfg(test)]
mod nft_processor_tests {
    use super::*;
    use aptos_indexer_processor_sdk::testing_framework::cli_parser::get_test_config;
    use aptos_indexer_test_transactions::json_transactions::generated_transactions::{
        IMPORTED_MAINNET_TXNS_2277018899_TRADEPORT_V2_ACCEPT_TOKEN_DELIST_SAME_TOKEN_DATA_ID,
        IMPORTED_MAINNET_TXNS_2296098846_TRADEPORT_V2_ACCEPT_TOKEN_DELIST2,
        IMPORTED_MAINNET_TXNS_2296149225_TRADEPORT_V2_ACCEPT_TOKEN_DELIST,
        IMPORTED_MAINNET_TXNS_2298838662_TRADEPORT_V2_FILL_OFFER,
        IMPORTED_MAINNET_TXNS_2313248448_WAPAL_FILL_OFFER,
        IMPORTED_MAINNET_TXNS_2381742315_WAPAL_CANCEL_LISTING,
        IMPORTED_MAINNET_TXNS_2381810159_WAPAL_CANCEL_OFFER,
        IMPORTED_MAINNET_TXNS_2382219668_WAPAL_FILL_COLLECTION_OFFER,
        IMPORTED_MAINNET_TXNS_2382221134_WAPAL_FILL_LISTING,
        IMPORTED_MAINNET_TXNS_2382251863_WAPAL_PLACE_LISTING,
        IMPORTED_MAINNET_TXNS_2382313982_WAPAL_PLACE_OFFER,
        IMPORTED_MAINNET_TXNS_2382373209_WAPAL_PLACE_COLLECTION_OFFER,
        IMPORTED_MAINNET_TXNS_2382373978_WAPAL_CANCEL_COLLECTION_OFFER,
        IMPORTED_MAINNET_TXNS_2386021136_TRADEPORT_V2_FILL_COLLECTION_OFFER,
        IMPORTED_MAINNET_TXNS_2386133936_TRADEPORT_V2_PLACE_OFFER,
        IMPORTED_MAINNET_TXNS_2386142672_TRADEPORT_V2_CANCEL_OFFER,
        IMPORTED_MAINNET_TXNS_2386455218_TRADEPORT_V2_FILL_LISTING,
        IMPORTED_MAINNET_TXNS_2386716658_TRADEPORT_V2_CANCEL_LISTING,
        IMPORTED_MAINNET_TXNS_2386809975_TRADEPORT_V2_PLACE_LISTING,
        IMPORTED_MAINNET_TXNS_2386889884_TRADEPORT_V2_CANCEL_COLLECTION_OFFER,
        IMPORTED_MAINNET_TXNS_2386891051_TRADEPORT_V2_PLACE_COLLECTION_OFFER,
    };

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_wapal_place_offer() {
        process_single_batch_txns(
            &[IMPORTED_MAINNET_TXNS_2382313982_WAPAL_PLACE_OFFER],
            Some("wapal_place_offer_test".to_string()),
            "wapal",
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_wapal_cancel_offer() {
        process_single_batch_txns(
            &[IMPORTED_MAINNET_TXNS_2381810159_WAPAL_CANCEL_OFFER],
            Some("wapal_cancel_offer_test".to_string()),
            "wapal",
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_wapal_fill_offer() {
        process_single_batch_txns(
            &[IMPORTED_MAINNET_TXNS_2313248448_WAPAL_FILL_OFFER],
            Some("wapal_fill_offer_test".to_string()),
            "wapal",
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_wapal_cancel_listing() {
        process_single_batch_txns(
            &[IMPORTED_MAINNET_TXNS_2381742315_WAPAL_CANCEL_LISTING],
            Some("wapal_cancel_listing_test".to_string()),
            "wapal",
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_wapal_place_listing() {
        process_single_batch_txns(
            &[IMPORTED_MAINNET_TXNS_2382251863_WAPAL_PLACE_LISTING],
            Some("wapal_place_listing_test".to_string()),
            "wapal",
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_wapal_fill_listing() {
        process_single_batch_txns(
            &[IMPORTED_MAINNET_TXNS_2382221134_WAPAL_FILL_LISTING],
            Some("wapal_fill_listing_test".to_string()),
            "wapal",
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_wapal_place_collection_offer() {
        process_single_batch_txns(
            &[IMPORTED_MAINNET_TXNS_2382373209_WAPAL_PLACE_COLLECTION_OFFER],
            Some("wapal_place_collection_offer_test".to_string()),
            "wapal",
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_wapal_cancel_collection_offer() {
        process_single_batch_txns(
            &[IMPORTED_MAINNET_TXNS_2382373978_WAPAL_CANCEL_COLLECTION_OFFER],
            Some("wapal_cancel_collection_offer_test".to_string()),
            "wapal",
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_wapal_fill_collection_offer() {
        process_single_batch_txns(
            &[IMPORTED_MAINNET_TXNS_2382219668_WAPAL_FILL_COLLECTION_OFFER],
            Some("wapal_fill_collection_offer_test".to_string()),
            "wapal",
        )
        .await;
    }

    // Tradeport V2 Tests
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_tradeport_v2_place_listing() {
        process_single_batch_txns(
            &[IMPORTED_MAINNET_TXNS_2386809975_TRADEPORT_V2_PLACE_LISTING],
            Some("tradeport_v2_place_listing_test".to_string()),
            "tradeport_v2",
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_tradeport_v2_cancel_listing() {
        process_single_batch_txns(
            &[IMPORTED_MAINNET_TXNS_2386716658_TRADEPORT_V2_CANCEL_LISTING],
            Some("tradeport_v2_cancel_listing_test".to_string()),
            "tradeport_v2",
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_tradeport_v2_fill_listing() {
        process_single_batch_txns(
            &[IMPORTED_MAINNET_TXNS_2386455218_TRADEPORT_V2_FILL_LISTING],
            Some("tradeport_v2_fill_listing_test".to_string()),
            "tradeport_v2",
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_tradeport_v2_place_offer() {
        process_single_batch_txns(
            &[IMPORTED_MAINNET_TXNS_2386133936_TRADEPORT_V2_PLACE_OFFER],
            Some("tradeport_v2_place_offer_test".to_string()),
            "tradeport_v2",
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_tradeport_v2_cancel_offer() {
        process_single_batch_txns(
            &[IMPORTED_MAINNET_TXNS_2386142672_TRADEPORT_V2_CANCEL_OFFER],
            Some("tradeport_v2_cancel_offer_test".to_string()),
            "tradeport_v2",
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_tradeport_v2_fill_offer() {
        process_single_batch_txns(
            &[IMPORTED_MAINNET_TXNS_2298838662_TRADEPORT_V2_FILL_OFFER],
            Some("tradeport_v2_fill_offer_test".to_string()),
            "tradeport_v2",
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_tradeport_v2_place_collection_offer() {
        process_single_batch_txns(
            &[IMPORTED_MAINNET_TXNS_2386891051_TRADEPORT_V2_PLACE_COLLECTION_OFFER],
            Some("tradeport_v2_place_collection_offer_test".to_string()),
            "tradeport_v2",
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_tradeport_v2_cancel_collection_offer() {
        process_single_batch_txns(
            &[IMPORTED_MAINNET_TXNS_2386889884_TRADEPORT_V2_CANCEL_COLLECTION_OFFER],
            Some("tradeport_v2_cancel_collection_offer_test".to_string()),
            "tradeport_v2",
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_tradeport_v2_fill_collection_offer() {
        process_single_batch_txns(
            &[IMPORTED_MAINNET_TXNS_2386021136_TRADEPORT_V2_FILL_COLLECTION_OFFER],
            Some("tradeport_v2_fill_collection_offer_test".to_string()),
            "tradeport_v2",
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_wapal_place_offer_and_cancel_offer() {
        sequential_multi_transaction_helper_function(
            &[&[IMPORTED_MAINNET_TXNS_2382313982_WAPAL_PLACE_OFFER], &[
                IMPORTED_MAINNET_TXNS_2381810159_WAPAL_CANCEL_OFFER,
            ]],
            "wapal_place_offer_and_cancel_offer_test",
            "wapal",
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_sequential_two_tradeport_v2_accept_token_delist_events() {
        sequential_multi_transaction_helper_function(
            &[
                &[IMPORTED_MAINNET_TXNS_2296149225_TRADEPORT_V2_ACCEPT_TOKEN_DELIST],
                &[IMPORTED_MAINNET_TXNS_2296098846_TRADEPORT_V2_ACCEPT_TOKEN_DELIST2],
            ],
            "sequential_two_tradeport_v2_accept_token_delist_events_test",
            "tradeport_v2",
        )
        .await;
    }

    // This test case is to to help verifying the output of the test case below, because this gets overriden.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_tradeport_v2_accept_token_delist() {
        process_single_batch_txns(
            &[IMPORTED_MAINNET_TXNS_2296098846_TRADEPORT_V2_ACCEPT_TOKEN_DELIST2],
            Some("tradeport_v2_accept_token_delist_test".to_string()),
            "tradeport_v2",
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_sequential_two_tradeport_v2_accept_token_delist_events_same_token_data_id() {
        sequential_multi_transaction_helper_function(
            &[&[IMPORTED_MAINNET_TXNS_2296098846_TRADEPORT_V2_ACCEPT_TOKEN_DELIST2], &[
                IMPORTED_MAINNET_TXNS_2277018899_TRADEPORT_V2_ACCEPT_TOKEN_DELIST_SAME_TOKEN_DATA_ID,
            ]],
            "sequential_two_tradeport_v2_accept_token_delist_events_same_token_data_id_test",
            "tradeport_v2",
        )
        .await;
    }

    async fn process_single_batch_txns(
        txns: &[&[u8]],
        test_case_name: Option<String>,
        marketplace_name: &str,
    ) {
        let (generate_flag, custom_output_path) = get_test_config();
        let output_path = custom_output_path.unwrap_or_else(|| DEFAULT_OUTPUT_FOLDER.to_string());

        let mut db = PostgresTestDatabase::new();
        db.setup().await.unwrap();

        process_transactions(
            &mut db,
            txns,
            &test_case_name.unwrap_or_default(),
            generate_flag,
            &output_path,
            true,
            marketplace_name,
        )
        .await;
    }

    /// Tests processing of two transactions sequentially
    /// Validates handling of multiple transactions with shared context
    async fn sequential_multi_transaction_helper_function(
        txn_batches: &[&[&[u8]]],
        output_name: &str,
        marketplace_name: &str,
    ) {
        let (generate_flag, custom_output_path) = get_test_config();
        let output_path = custom_output_path.unwrap_or_else(|| DEFAULT_OUTPUT_FOLDER.to_string());

        let mut db = PostgresTestDatabase::new();
        db.setup().await.unwrap();

        for (i, txn_batch) in txn_batches.iter().enumerate() {
            let is_last = i == txn_batches.len() - 1;
            process_transactions(
                &mut db,
                txn_batch,
                output_name,
                is_last && generate_flag,
                &output_path,
                is_last,
                marketplace_name,
            )
            .await;
        }
    }
}
