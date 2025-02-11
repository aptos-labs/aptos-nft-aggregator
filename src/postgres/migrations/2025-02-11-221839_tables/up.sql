-- actvities table
CREATE TABLE IF NOT EXISTS nft_marketplace_activities (
  transaction_version BIGINT,
  index BIGINT,
  raw_event_type VARCHAR NOT NULL,
  standard_event_type VARCHAR NOT NULL,
  creator_address VARCHAR(66),
  collection_id VARCHAR(66),
  collection_name VARCHAR,
  token_data_id VARCHAR(66),
  token_id VARCHAR(66),
  token_name VARCHAR,
  price NUMERIC,
  token_amount NUMERIC,
  buyer VARCHAR(66),
  seller VARCHAR(66),
  json_data JSONB NOT NULL,
  marketplace VARCHAR NOT NULL,
  contract_address VARCHAR(66) NOT NULL,
  entry_function_id_str VARCHAR NOT NULL,
  transaction_timestamp TIMESTAMP NOT NULL,
  PRIMARY KEY (transaction_version, index)
);
-- current nft marketplace bids table
CREATE TABLE IF NOT EXISTS current_nft_marketplace_bids (
  token_id VARCHAR(66),
  token_data_id VARCHAR(66),
  buyer VARCHAR(66),
  price NUMERIC,
  creator_address VARCHAR(66),
  token_amount NUMERIC,
  token_name VARCHAR,
  collection_name VARCHAR,
  collection_id VARCHAR(66),
  marketplace VARCHAR NOT NULL,
  contract_address VARCHAR(66) NOT NULL,
  entry_function_id_str VARCHAR NOT NULL,
  is_deleted BOOLEAN NOT NULL,
  last_transaction_version BIGINT NOT NULL,
  last_transaction_timestamp TIMESTAMP NOT NULL,
  PRIMARY KEY (token_data_id, buyer, price)
);
-- nft marketplace bids table
CREATE TABLE IF NOT EXISTS nft_marketplace_bids (
  transaction_version BIGINT,
  index BIGINT,
  token_id VARCHAR(66),
  token_data_id VARCHAR(66),
  buyer VARCHAR(66),
  price NUMERIC,
  creator_address VARCHAR(66),
  token_amount NUMERIC,
  token_name VARCHAR,
  collection_name VARCHAR,
  collection_id VARCHAR(66),
  marketplace VARCHAR NOT NULL,
  contract_address VARCHAR(66) NOT NULL,
  entry_function_id_str VARCHAR NOT NULL,
  event_type VARCHAR NOT NULL,
  transaction_timestamp TIMESTAMP NOT NULL,
  PRIMARY KEY (transaction_version, index)
);
-- current nft marketplace collection bids table
CREATE TABLE IF NOT EXISTS current_nft_marketplace_collection_bids (
  collection_id VARCHAR(66),
  buyer VARCHAR(66),
  price NUMERIC,
  creator_address VARCHAR(66),
  token_amount NUMERIC,
  collection_name VARCHAR,
  marketplace VARCHAR NOT NULL,
  contract_address VARCHAR(66) NOT NULL,
  entry_function_id_str VARCHAR NOT NULL,
  is_deleted BOOLEAN NOT NULL,
  last_transaction_version BIGINT NOT NULL,
  last_transaction_timestamp TIMESTAMP NOT NULL,
  PRIMARY KEY (collection_id, buyer, price)
);
-- nft marketplace collection bids table
CREATE TABLE IF NOT EXISTS nft_marketplace_collection_bids (
  transaction_version BIGINT,
  index BIGINT,
  creator_address VARCHAR(66),
  collection_name VARCHAR,
  collection_id VARCHAR(66),
  price NUMERIC,
  token_amount NUMERIC,
  buyer VARCHAR(66),
  seller VARCHAR(66),
  marketplace VARCHAR NOT NULL,
  contract_address VARCHAR(66) NOT NULL,
  entry_function_id_str VARCHAR NOT NULL,
  event_type VARCHAR NOT NULL,
  transaction_timestamp TIMESTAMP NOT NULL,
  PRIMARY KEY (transaction_version, index)
);
-- current nft marketplace listings table
CREATE TABLE IF NOT EXISTS current_nft_marketplace_listings (
  token_id VARCHAR(66) PRIMARY KEY,
  token_data_id VARCHAR(66),
  creator_address VARCHAR(66),
  token_name VARCHAR,
  collection_name VARCHAR,
  collection_id VARCHAR(66),
  price NUMERIC,
  token_amount NUMERIC,
  seller VARCHAR(66),
  is_deleted BOOLEAN NOT NULL,
  marketplace VARCHAR NOT NULL,
  contract_address VARCHAR(66) NOT NULL,
  entry_function_id_str VARCHAR NOT NULL,
  last_transaction_version BIGINT NOT NULL,
  last_transaction_timestamp TIMESTAMP NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW()
);
-- nft marketplace listings table
CREATE TABLE IF NOT EXISTS nft_marketplace_listings (
  transaction_version BIGINT,
  index BIGINT,
  creator_address VARCHAR(66),
  token_name VARCHAR,
  token_data_id VARCHAR(66),
  collection_name VARCHAR,
  collection_id VARCHAR(66),
  price NUMERIC,
  token_amount NUMERIC,
  seller VARCHAR(66),
  buyer VARCHAR(66),
  marketplace VARCHAR NOT NULL,
  contract_address VARCHAR(66) NOT NULL,
  entry_function_id_str VARCHAR NOT NULL,
  event_type VARCHAR NOT NULL,
  transaction_timestamp TIMESTAMP NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  PRIMARY KEY (transaction_version, index)
);