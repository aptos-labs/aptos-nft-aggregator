# NFT Aggregator

## Get Started

1. Copy from `config-example.yaml` to `config.yaml` and update with proper values.
2. Run the processor:

```bash
cargo run --release -- -c config.yaml
```

### Configuration File (`config.yaml`) Explanation

The `config.yaml` file is used to configure the NFT aggregator. Below is an explanation of each field:

- **health_check_port**: Port number for health check endpoint (e.g., 8080)


!Note that the config will be updated with the latest SDK version soon.

- **server_config**:
  - **channel_size**: The size of the channel buffer used for processing transactions (default: 100)
  - **db_config**:
    - **type**: The type of database configuration (currently "postgres_config")
    - **connection_string**: PostgreSQL connection string. **Replace with your own.**
  - **transaction_stream_config**:
    - **starting_version**: The starting version of the transaction stream
    - **indexer_grpc_data_service_address**: The gRPC address (e.g., "https://grpc.mainnet.aptoslabs.com:443")
    - **auth_token**: The authentication token. **Replace with your own.**
      Get your token from https://developers.aptoslabs.com/
    - **request_name_header**: The name header for gRPC requests

- **nft_marketplace_configs**:
  - **marketplaces**: A list of marketplace configurations, each containing:
    - **name**: Marketplace identifier (e.g., "topaz", "tradeport", "bluemove")
    - **event_types**: List of event type configurations:
      - **type**: Event category ("listing", "token_offer", or "collection_offer"), these are the standard types that are supported by the processor.
      - **cancel**: Event type for cancellation events
      - **fill**: Event type for fill/buy events
      - **place**: Event type for place/list events
    - **tables**: Configuration for database tables and their columns:
      - **nft_marketplace_activities**: Main activity table configuration
        - **columns**: Column mappings for extracting data:
          - **collection_id**: Collection identifier
          - **token_data_id**: Token data identifier
          - **token_name**: Name of the token
          - **creator_address**: Creator's address
          - **collection_name**: Name of the collection
          - **price**: Price of the NFT
          - **buyer**: Buyer's address
          - **seller**: Seller's address
          - **token_amount**: Amount of tokens
          - **listing_id**: Listing identifier
          - **offer_id**: Offer identifier
          - **expiration_time**: Offer/listing expiration time
      - **current_nft_marketplace_listings**: Current listings table (optional)
      - **current_nft_marketplace_token_offers**: Current token offers table (optional)
      - **current_nft_marketplace_collection_offers**: Current collection offers table (optional)

Note: The current tables (`current_nft_marketplace_listings`, `current_nft_marketplace_token_offers`, 
`current_nft_marketplace_collection_offers`) will automatically inherit columns from the 
`nft_marketplace_activities` table by default. You only need to specify columns in these tables if you 
want to override or add additional data extraction specific to those tables. This is particularly 
useful when you need to extract additional data from `write_set_changes` for specific event types.


Each column configuration can include:
- **path**: JSON path array for extracting values from event data
- **source**: Data source ("events" by default, or "write_set_changes")
- **resource_type**: Required for `write_set_changes`, specifies the resource type (e.g., "0x4::token::Token")
- **event_type**: Optional, specifies which event type requires this field

### Data Processing

The processor handles two types of data:

1. **Events**: Processed by the EventRemapper, which:
   - Matches events to marketplace configurations
   - Extracts data using configured JSON paths
   - Creates NFT marketplace activities
   - Sets token standard (v1 or v2)
   - Generates token_data_id and collection_id if needed

2. **WriteSetChanges**: Processed by the ResourceMapper, which:
   - Matches token_data_id or collection_id to existing activities based on the `resource_type` field of the write_set_changes
   - Updates activities with additional data from resources
   - Handles V2 token standard specific data
      
### Running the Processor

To run the processor, ensure that you have Rust installed and the necessary dependencies. Use the provided command to start the processor with the specified configuration file.

```bash
cargo run --release -- -c config.yaml
```

This command will compile and run the processor in release mode, using the `config.yaml` file for configuration.

### Additional Information

- Ensure that the database specified in the `connection_string` is accessible and properly configured.
- The `auth_token` should be kept secure and not exposed in public repositories.
- Adjust the `starting_version` and `request_ending_version` to control the range of transactions processed.

For further customization and troubleshooting, refer to the source code and comments within the configuration file.





