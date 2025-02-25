# NFT Aggregator

## Get Started

1. Copy from `config-example.yaml` to `config.yaml` and update with proper values.
2. Run the processor:

```bash
cargo run --release -- -c config.yaml
```

### Configuration File (`config.yaml`) Explanation

The `config.yaml` file is used to configure the NFT aggregator. Below is an explanation of each field:

- **server_config**:
  - **channel_size**: The size of the channel buffer used for processing events. This affects how many events can be queued for processing at any given time. we use Default value of around 100-200, recommend to keep it as is.
  - **db_config**:
    - **type**: The type of database configuration. Currently set to "postgres_config" for PostgreSQL.
    - **connection_string**: The connection string used to connect to the PostgreSQL database. **Replace the values with your own.**
  - **transaction_stream_config**:
    - **starting_version**: The starting version of the transaction stream to process.
    - **request_ending_version**: The ending version of the transaction stream to process.
    - **indexer_grpc_data_service_address**: The gRPC address of the indexer data service. 
    - **auth_token**: The authentication token used to access the gRPC service. **Replace the value with your own.**
      You can get the auth token from https://developers.aptoslabs.com/
    - **request_name_header**: The name header used in requests to the gRPC service.

- **nft_marketplace_configs**:
  - **marketplace_configs**: A list of configurations for each marketplace. Each marketplace configuration includes:
    - **marketplace_name**: The name of the marketplace (e.g., "topaz", "tradeport").
    - **event_config**: Configuration for extracting data from events using Json Path.
      - **collection_id**: JSON path to extract the collection ID.
      - **token_name**: JSON path to extract the token name.
      - **creator_address**: JSON path to extract the creator's address.
      - **collection_name**: JSON path to extract the collection name.
      - **price**: JSON path to extract the price of the NFT.
      - **token_amount**: JSON path to extract the amount of tokens.
      - **buyer**: JSON path to extract the buyer's address.
      - **seller**: JSON path to extract the seller's address.
      - **deadline**: JSON path to extract the deadline for offers (specific to some marketplaces).
      - **token_inner**: JSON path to extract inner token data (specific to V2 contracts).
      - **collection_inner**: JSON path to extract inner collection data (specific to V2 contracts).
    - **listing_config**: Configuration for listing events:
      - **cancel_event**: Event type for canceling a listing.
      - **fill_event**: Event type for filling a listing.
      - **place_event**: Event type for placing a listing.
      - **collection_name**: (Optional) JSON path to extract the collection name.
      - **buyer**: (Optional) JSON path to extract the buyer's address.
      - **seller**: (Optional) JSON path to extract the seller's address.
    - **offer_config**: Configuration for offer events:
      - **cancel_event**: Event type for canceling an offer.
      - **fill_event**: Event type for filling an offer.
      - **place_event**: Event type for placing an offer.
      - **buyer**: (Optional) JSON path to extract the buyer's address.
      - **seller**: (Optional) JSON path to extract the seller's address.
    - **collection_offer_config**: Configuration for collection offer events, we defined its own struct for better flexibility.
      - **cancel_event**: Event type for canceling a collection offer.
      - **fill_event**: Event type for filling a collection offer.
      - **place_event**: Event type for placing a collection offer.
        - find more details in the [`CollectionEventParams`](src/processors/marketplace_config.rs) struct.
      
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





