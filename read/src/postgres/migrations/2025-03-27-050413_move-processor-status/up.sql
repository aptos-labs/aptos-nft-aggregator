-- Recreate the processor_metadata schemas
CREATE SCHEMA IF NOT EXISTS processor_metadata;

-- Tracks latest processed version per processor
CREATE TABLE IF NOT EXISTS processor_metadata.processor_status (
  processor VARCHAR(100) UNIQUE PRIMARY KEY NOT NULL,
  last_success_version BIGINT NOT NULL,
  last_updated TIMESTAMP NOT NULL DEFAULT NOW(),
  last_transaction_timestamp TIMESTAMP NULL
);

-- ALTER TABLE processor_status ADD CONSTRAINT processor_status_pkey UNIQUE (processor);
-- ALTER TABLE processor_status ADD PRIMARY KEY (processor);


-- Tracks chain id
CREATE TABLE IF NOT EXISTS processor_metadata.ledger_infos (chain_id BIGINT UNIQUE PRIMARY KEY NOT NULL);

-- Copy data to processor_metadata tables
INSERT INTO processor_metadata.processor_status SELECT * FROM public.processor_status;


-- Create backfill_processor_status table
CREATE TABLE backfill_processor_status (
    backfill_alias VARCHAR(100) NOT NULL,
    backfill_status VARCHAR(50) NOT NULL,
    last_success_version BIGINT NOT NULL,
    last_updated TIMESTAMP NOT NULL DEFAULT NOW(),
    last_transaction_timestamp TIMESTAMP NULL,
    backfill_start_version BIGINT NOT NULL,
    backfill_end_version BIGINT NULL,
    PRIMARY KEY (backfill_alias)
);