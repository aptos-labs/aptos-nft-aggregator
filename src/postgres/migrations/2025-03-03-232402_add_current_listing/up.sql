-- Your SQL goes here

-- Update nft_marketplace_activities table
ALTER TABLE nft_marketplace_activities 
    ALTER COLUMN price TYPE BIGINT USING (price::BIGINT),
    ALTER COLUMN token_amount TYPE BIGINT USING (token_amount::BIGINT);

ALTER TABLE nft_marketplace_activities    
    ADD COLUMN fee_schedule_id VARCHAR(66),
    ADD COLUMN coin_type VARCHAR(66) DEFAULT '0x1::aptos_coin::AptosCoin',
    ADD COLUMN listing_id VARCHAR(128),      -- Increased length for marketplace prefix
    ADD COLUMN offer_id VARCHAR(128);        -- Combined field for both token and collection offers

-- current nft marketplace listings table
CREATE TABLE IF NOT EXISTS current_nft_marketplace_listings (
    listing_id VARCHAR(128) PRIMARY KEY,     -- Changed from token_data_id to listing_id as PK
    token_data_id VARCHAR(66) NOT NULL,      -- Keep this for token identification
    collection_id VARCHAR(66),
    fee_schedule_id VARCHAR(66),
    seller VARCHAR(66),
    price BIGINT,
    token_amount BIGINT,
    token_standard VARCHAR(66),
    is_deleted BOOLEAN NOT NULL,
    coin_type VARCHAR(66),
    marketplace VARCHAR NOT NULL,            -- Keep for querying but not as PK
    contract_address VARCHAR NOT NULL,
    entry_function_id_str VARCHAR NOT NULL,
    last_transaction_version BIGINT NOT NULL,
    last_transaction_timestamp TIMESTAMP NOT NULL
);

-- Create index for common queries
CREATE INDEX idx_current_listings_marketplace ON current_nft_marketplace_listings(marketplace);
CREATE INDEX idx_current_listings_token_data ON current_nft_marketplace_listings(token_data_id);

-- current nft marketplace token offers table
CREATE TABLE IF NOT EXISTS current_nft_marketplace_token_offers (
    offer_id VARCHAR(128) PRIMARY KEY,       -- Changed to match activity table
    token_data_id VARCHAR(66) NOT NULL,      -- Required for token offers
    collection_id VARCHAR(66),
    fee_schedule_id VARCHAR(66),
    buyer VARCHAR(66),
    price BIGINT,
    token_amount BIGINT,
    token_name VARCHAR,
    is_deleted BOOLEAN NOT NULL,
    token_standard VARCHAR(66),
    coin_type VARCHAR(66),
    marketplace VARCHAR NOT NULL,            
    contract_address VARCHAR NOT NULL,
    entry_function_id_str VARCHAR NOT NULL,
    last_transaction_version BIGINT NOT NULL,
    last_transaction_timestamp TIMESTAMP NOT NULL
);

-- Create index for common queries
CREATE INDEX idx_current_token_offers_marketplace ON current_nft_marketplace_token_offers(marketplace);
CREATE INDEX idx_current_token_offers_token_data ON current_nft_marketplace_token_offers(token_data_id);

-- current nft marketplace collection offers table
CREATE TABLE IF NOT EXISTS current_nft_marketplace_collection_offers (
    collection_offer_id VARCHAR(128) PRIMARY KEY,       -- Changed to match activity table
    collection_id VARCHAR(66),
    fee_schedule_id VARCHAR(66),
    buyer VARCHAR(66),
    price BIGINT,
    remaining_token_amount BIGINT,
    is_deleted BOOLEAN NOT NULL,
    token_standard VARCHAR(66),
    coin_type VARCHAR(66),
    marketplace VARCHAR NOT NULL,            
    contract_address VARCHAR NOT NULL,
    entry_function_id_str VARCHAR NOT NULL,
    last_transaction_version BIGINT NOT NULL,
    last_transaction_timestamp TIMESTAMP NOT NULL
);

Create index for common queries
CREATE INDEX idx_current_collection_offers_marketplace ON current_nft_marketplace_collection_offers(marketplace);