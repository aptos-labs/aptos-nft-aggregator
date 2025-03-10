-- actvities table
CREATE TABLE IF NOT EXISTS nft_marketplace_activities (
  txn_version BIGINT,
  index BIGINT,
  raw_event_type VARCHAR NOT NULL,
  standard_event_type VARCHAR NOT NULL,
  creator_address VARCHAR(66),
  collection_id VARCHAR(66),
  collection_name VARCHAR,
  token_data_id VARCHAR(66),
  token_name VARCHAR,
  token_standard VARCHAR(66),
  price NUMERIC,
  token_amount NUMERIC,
  buyer VARCHAR(66),
  seller VARCHAR(66),
  deadline VARCHAR,
  json_data JSONB NOT NULL,
  marketplace VARCHAR NOT NULL,
  contract_address VARCHAR NOT NULL,
  entry_function_id_str VARCHAR,  -- we removed the limit on the length of the entry function id string. 
  block_timestamp TIMESTAMP NOT NULL,
  PRIMARY KEY (txn_version, index)
);

-- Uncomment this and run separately to create the index
-- CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_nft_marketplace_activities ON nft_marketplace_activities USING btree (txn_version, index);

-- Processor status table
CREATE TABLE processor_status (
  processor VARCHAR(100) PRIMARY KEY NOT NULL,
  last_success_version BIGINT NOT NULL,
  last_updated TIMESTAMP NOT NULL DEFAULT NOW(),
  last_transaction_timestamp TIMESTAMP
);