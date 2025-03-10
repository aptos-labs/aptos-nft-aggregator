-- This file should undo anything in `up.sql`
ALTER TABLE nft_marketplace_activities 
    ALTER COLUMN price TYPE NUMERIC USING (price::NUMERIC);

ALTER TABLE nft_marketplace_activities 
    ALTER COLUMN token_amount TYPE NUMERIC USING (token_amount::NUMERIC);

ALTER TABLE nft_marketplace_activities 
    DROP COLUMN fee_schedule_id;

ALTER TABLE nft_marketplace_activities 
    DROP COLUMN coin_type;

ALTER TABLE nft_marketplace_activities 
    DROP COLUMN listing_id;

ALTER TABLE nft_marketplace_activities 
    DROP COLUMN offer_id;

ALTER TABLE nft_marketplace_activities 
    DROP COLUMN collection_offer_id;
    
DROP TABLE IF EXISTS current_nft_marketplace_listings;
DROP TABLE IF EXISTS current_nft_marketplace_token_offers;
DROP TABLE IF EXISTS current_nft_marketplace_collection_offers;

-- Drop indexes for current_nft_marketplace_listings
DROP INDEX IF EXISTS idx_current_listings_marketplace;
DROP INDEX IF EXISTS idx_current_listings_token_data;

-- Drop indexes for current_nft_marketplace_token_offers
DROP INDEX IF EXISTS idx_current_token_offers_marketplace;
DROP INDEX IF EXISTS idx_current_token_offers_token_data;

-- Drop indexes for current_nft_marketplace_collection_offers
DROP INDEX IF EXISTS idx_current_collection_offers_marketplace;
