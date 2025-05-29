-- This file should undo anything in `up.sql`

-- Remove bid_key field from nft_marketplace_activities table
ALTER TABLE nft_marketplace_activities
DROP COLUMN IF EXISTS bid_key;

-- Remove bid_key field from current_nft_marketplace_token_offers table
ALTER TABLE current_nft_marketplace_token_offers
DROP COLUMN IF EXISTS bid_key;

-- Remove bid_key field from current_nft_marketplace_collection_offers table
ALTER TABLE current_nft_marketplace_collection_offers
DROP COLUMN IF EXISTS bid_key;
