-- This file should undo anything in `up.sql`
SELECT 1;
-- -- Drop indexes for nft_marketplace_activities
-- DROP INDEX IF EXISTS idx_collection_event_ts;
-- DROP INDEX IF EXISTS idx_token_id;
-- DROP INDEX IF EXISTS idx_buyer;
-- DROP INDEX IF EXISTS idx_seller;
-- DROP INDEX IF EXISTS idx_listing_id;
-- DROP INDEX IF EXISTS idx_offer_id;
-- DROP INDEX IF EXISTS idx_timestamp;

-- -- Drop indexes for current_nft_marketplace_listings
-- DROP INDEX IF EXISTS idx_current_nft_marketplace_listings_token_data_id;
-- DROP INDEX IF EXISTS idx_current_nft_marketplace_listings_collection_id;
-- DROP INDEX IF EXISTS idx_current_nft_marketplace_listings_collection_id_price;
-- DROP INDEX IF EXISTS idx_current_nft_marketplace_listings_seller;

-- -- Drop indexes for current_nft_marketplace_token_offers
-- DROP INDEX IF EXISTS idx_current_nft_marketplace_token_offers_token_data_id;
-- DROP INDEX IF EXISTS idx_current_nft_marketplace_token_offers_price;
-- DROP INDEX IF EXISTS idx_current_nft_marketplace_token_offers_buyer;

-- -- Drop indexes for current_nft_marketplace_collection_offers
-- DROP INDEX IF EXISTS idx_current_nft_marketplace_collection_offers_token_data_id;
-- DROP INDEX IF EXISTS idx_current_nft_marketplace_collection_offers_collection_id;
-- DROP INDEX IF EXISTS idx_current_nft_marketplace_collection_offers_collection_offer_id_token_data_id;
