-- Your SQL goes here

-- current nft marketplace listings table
CREATE TABLE IF NOT EXISTS current_nft_marketplace_listings (
    token_data_id VARCHAR(66) PRIMARY KEY,   
    listing_id VARCHAR(128),
    collection_id VARCHAR(66),
    seller VARCHAR(66),
    price BIGINT NOT NULL,
    token_amount BIGINT,
    -- token_name VARCHAR, -- need to add
    token_standard VARCHAR(66),
    is_deleted BOOLEAN NOT NULL,
    marketplace VARCHAR NOT NULL,
    contract_address VARCHAR NOT NULL,
    last_transaction_version BIGINT NOT NULL,
    last_transaction_timestamp TIMESTAMP NOT NULL
);

-- current nft marketplace token offers table
CREATE TABLE IF NOT EXISTS current_nft_marketplace_token_offers (
    token_data_id VARCHAR(66),
    offer_id VARCHAR(128),
    marketplace VARCHAR NOT NULL,
    collection_id VARCHAR(66),
    buyer VARCHAR(66) NOT NULL,
    price BIGINT NOT NULL,
    token_amount BIGINT,
    token_name VARCHAR,
    is_deleted BOOLEAN NOT NULL,
    token_standard VARCHAR(66),
    contract_address VARCHAR NOT NULL,
    last_transaction_version BIGINT NOT NULL,
    last_transaction_timestamp TIMESTAMP NOT NULL,
    PRIMARY KEY (token_data_id, buyer)
);

-- current nft marketplace collection offers table
CREATE TABLE IF NOT EXISTS current_nft_marketplace_collection_offers (
    collection_offer_id VARCHAR(128) PRIMARY KEY,  
    collection_id VARCHAR(66),
    buyer VARCHAR(66) NOT NULL,
    price BIGINT NOT NULL,
    remaining_token_amount BIGINT,
    is_deleted BOOLEAN NOT NULL,
    token_standard VARCHAR(66),
    marketplace VARCHAR NOT NULL,            
    contract_address VARCHAR NOT NULL,
    last_transaction_version BIGINT NOT NULL,
    last_transaction_timestamp TIMESTAMP NOT NULL
);
