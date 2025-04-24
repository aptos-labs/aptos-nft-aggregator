-- Your SQL goes here

-- Add bid_key field to nft_marketplace_activities table
ALTER TABLE nft_marketplace_activities
ADD COLUMN IF NOT EXISTS bid_key BIGINT;

-- Add bid_key field to current_nft_marketplace_token_offers table
ALTER TABLE current_nft_marketplace_token_offers
ADD COLUMN IF NOT EXISTS bid_key BIGINT;

-- Add bid_key field to current_nft_marketplace_collection_offers table
ALTER TABLE current_nft_marketplace_collection_offers
ADD COLUMN IF NOT EXISTS bid_key BIGINT;
