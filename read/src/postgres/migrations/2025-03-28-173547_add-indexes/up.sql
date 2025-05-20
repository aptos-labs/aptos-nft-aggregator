
SELECT 1;
-- -- Fast filtering and ordering
-- CREATE INDEX idx_collection_event_ts ON nft_marketplace_activities (collection_id, standard_event_type, block_timestamp DESC);
-- CREATE INDEX idx_token_id ON nft_marketplace_activities (token_data_id);
-- CREATE INDEX idx_buyer ON nft_marketplace_activities (buyer);
-- CREATE INDEX idx_seller ON nft_marketplace_activities (seller);
-- CREATE INDEX idx_listing_id ON nft_marketplace_activities (listing_id);
-- CREATE INDEX idx_offer_id ON nft_marketplace_activities (offer_id);
-- CREATE INDEX idx_timestamp ON nft_marketplace_activities (block_timestamp DESC);

-- -- current_nft_marketplace_listings
-- CREATE INDEX idx_current_nft_marketplace_listings_token_data_id ON current_nft_marketplace_listings (token_data_id);
-- CREATE INDEX idx_current_nft_marketplace_listings_collection_id ON current_nft_marketplace_listings (collection_id);
-- CREATE INDEX idx_current_nft_marketplace_listings_collection_id_price ON current_nft_marketplace_listings (collection_id, price);
-- CREATE INDEX idx_current_nft_marketplace_listings_seller ON current_nft_marketplace_listings (seller);

-- -- current_nft_marketplace_token_offers
-- CREATE INDEX idx_current_nft_marketplace_token_offers_token_data_id ON current_nft_marketplace_token_offers (token_data_id);
-- CREATE INDEX idx_current_nft_marketplace_token_offers_price ON current_nft_marketplace_token_offers (price);
-- CREATE INDEX idx_current_nft_marketplace_token_offers_buyer ON current_nft_marketplace_token_offers (buyer);

-- -- current_nft_marketplace_collection_offers
-- CREATE INDEX idx_current_nft_marketplace_collection_offers_token_data_id ON current_nft_marketplace_collection_offers (token_data_id);
-- CREATE INDEX idx_current_nft_marketplace_collection_offers_collection_id ON current_nft_marketplace_collection_offers (collection_id);
-- CREATE INDEX idx_current_nft_marketplace_collection_offers_collection_offer_id_token_data_id ON current_nft_marketplace_collection_offers (collection_offer_id, token_data_id);