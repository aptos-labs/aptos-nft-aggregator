-- This file should undo anything in `up.sql`
-- Remove expiration_time from token offers table
ALTER TABLE current_nft_marketplace_token_offers
DROP COLUMN IF EXISTS expiration_time;

-- Remove expiration_time from collection offers table
ALTER TABLE current_nft_marketplace_collection_offers
DROP COLUMN IF EXISTS expiration_time;

-- Drop new column
ALTER TABLE nft_marketplace_activities
DROP COLUMN expiration_time;

-- Add expiration_time back to activities table
ALTER TABLE nft_marketplace_activities
ADD COLUMN expiration_time VARCHAR;