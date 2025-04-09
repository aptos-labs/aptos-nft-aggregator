-- Your SQL goes here
-- Add expiration_time to token offers table
ALTER TABLE current_nft_marketplace_token_offers
ADD COLUMN expiration_time TIMESTAMP;

-- Add expiration_time to collection offers table
ALTER TABLE current_nft_marketplace_collection_offers
ADD COLUMN expiration_time TIMESTAMP;

ALTER TABLE nft_marketplace_activities 
DROP COLUMN expiration_time;

-- 2. Add new column with correct type
ALTER TABLE nft_marketplace_activities 
ADD COLUMN expiration_time TIMESTAMP;