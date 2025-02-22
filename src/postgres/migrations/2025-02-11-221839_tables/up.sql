-- actvities table
CREATE TABLE IF NOT EXISTS nft_marketplace_activities (
  txn_version BIGINT,
  index BIGINT,
  raw_event_type VARCHAR NOT NULL,
  standard_event_type VARCHAR NOT NULL,
  creator_address VARCHAR(66),
  collection_id VARCHAR(66),
  collection_name VARCHAR,
  offer_or_listing_id VARCHAR(66),
  token_data_id VARCHAR(66),
  token_name VARCHAR,
  token_standard VARCHAR(66),
  price NUMERIC,
  token_amount NUMERIC,
  buyer VARCHAR(66),
  seller VARCHAR(66),
  json_data JSONB NOT NULL,
  marketplace VARCHAR NOT NULL,
  contract_address VARCHAR NOT NULL,
  entry_function_id_str VARCHAR,  -- we removed the limit on the length of the entry function id string. 
  transaction_timestamp TIMESTAMP NOT NULL,
  PRIMARY KEY (txn_version, index)
);
-- current nft marketplace bids table
CREATE TABLE IF NOT EXISTS current_nft_marketplace_bids (
  offer_id VARCHAR(66) NOT NULL,
  token_data_id VARCHAR(66) NOT NULL,
  index BIGINT,
  buyer VARCHAR(66),
  price NUMERIC,
  creator_address VARCHAR(66),
  token_amount NUMERIC,
  token_name VARCHAR,
  collection_name VARCHAR,
  collection_id VARCHAR(66),
  marketplace VARCHAR NOT NULL,
  contract_address VARCHAR NOT NULL,
  entry_function_id_str VARCHAR, 
  is_deleted BOOLEAN NOT NULL,
  last_transaction_version BIGINT NOT NULL,
  last_transaction_timestamp TIMESTAMP NOT NULL,
  PRIMARY KEY (offer_id, token_data_id)
);
-- nft marketplace bids table
CREATE TABLE IF NOT EXISTS nft_marketplace_bids (
  offer_id VARCHAR(66),
  txn_version BIGINT,
  index BIGINT,
  token_data_id VARCHAR(66),
  buyer VARCHAR(66),
  price NUMERIC,
  creator_address VARCHAR(66),
  token_amount NUMERIC,
  token_name VARCHAR,
  collection_name VARCHAR,
  collection_id VARCHAR(66),
  marketplace VARCHAR NOT NULL,
  contract_address VARCHAR NOT NULL,
  entry_function_id_str VARCHAR, 
  event_type VARCHAR NOT NULL,
  transaction_timestamp TIMESTAMP NOT NULL,
  PRIMARY KEY (txn_version, index)
);
-- current nft marketplace collection bids table
CREATE TABLE IF NOT EXISTS current_nft_marketplace_collection_bids (
  offer_id VARCHAR(66) NOT NULL,
  collection_id VARCHAR(66) NOT NULL,
  index BIGINT,
  buyer VARCHAR(66),
  price NUMERIC,
  creator_address VARCHAR(66),
  token_amount NUMERIC,
  collection_name VARCHAR,
  marketplace VARCHAR NOT NULL,
  coin_type VARCHAR,
  contract_address VARCHAR NOT NULL,
  entry_function_id_str VARCHAR, 
  expiration_time BIGINT,
  is_deleted BOOLEAN NOT NULL,
  last_transaction_version BIGINT NOT NULL,
  last_transaction_timestamp TIMESTAMP NOT NULL,
  PRIMARY KEY (offer_id, collection_id)
);
-- nft marketplace collection bids table
CREATE TABLE IF NOT EXISTS nft_marketplace_collection_bids (
  txn_version BIGINT,
  offer_id VARCHAR(66),
  index BIGINT,
  creator_address VARCHAR(66),
  collection_name VARCHAR,
  collection_id VARCHAR(66),
  price NUMERIC,
  token_amount NUMERIC,
  buyer VARCHAR(66),
  seller VARCHAR(66),
  marketplace VARCHAR NOT NULL,
  contract_address VARCHAR NOT NULL,
  entry_function_id_str VARCHAR,  
  event_type VARCHAR NOT NULL,
  transaction_timestamp TIMESTAMP NOT NULL,
  PRIMARY KEY (txn_version, index)
);
-- current nft marketplace listings table
CREATE TABLE IF NOT EXISTS current_nft_marketplace_listings (
  listing_id VARCHAR(66) NOT NULL,
  index BIGINT,
  token_data_id VARCHAR(66) NOT NULL,
  creator_address VARCHAR(66),
  token_name VARCHAR,
  collection_name VARCHAR,
  collection_id VARCHAR(66),
  price NUMERIC,
  token_amount NUMERIC,
  seller VARCHAR(66),
  token_standard VARCHAR(66),
  is_deleted BOOLEAN NOT NULL,
  marketplace VARCHAR NOT NULL,
  contract_address VARCHAR NOT NULL,
  entry_function_id_str VARCHAR, 
  event_type VARCHAR,
  last_transaction_version BIGINT NOT NULL,
  last_transaction_timestamp TIMESTAMP NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  PRIMARY KEY (listing_id, token_data_id)
);
-- nft marketplace listings table
CREATE TABLE IF NOT EXISTS nft_marketplace_listings (
  listing_id VARCHAR(66),
  txn_version BIGINT,
  index BIGINT,
  creator_address VARCHAR(66),
  standard_event_type VARCHAR,
  token_name VARCHAR,
  token_data_id VARCHAR(66),
  collection_name VARCHAR,
  collection_id VARCHAR(66),
  price NUMERIC,
  token_amount NUMERIC,
  seller VARCHAR(66),
  token_standard VARCHAR(66),
  marketplace VARCHAR NOT NULL,
  contract_address VARCHAR NOT NULL,
  entry_function_id_str VARCHAR, 
  event_type VARCHAR,
  transaction_timestamp TIMESTAMP NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  PRIMARY KEY (txn_version, index)
);