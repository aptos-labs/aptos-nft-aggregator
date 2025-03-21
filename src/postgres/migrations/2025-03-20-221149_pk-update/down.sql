-- This file should undo anything in `up.sql`

-- Drop the new composite primary keys
ALTER TABLE current_nft_marketplace_listings 
DROP CONSTRAINT IF EXISTS current_nft_marketplace_listings_pkey;

ALTER TABLE current_nft_marketplace_token_offers
DROP CONSTRAINT IF EXISTS current_nft_marketplace_token_offers_pkey;

ALTER TABLE current_nft_marketplace_collection_offers
DROP CONSTRAINT IF EXISTS current_nft_marketplace_collection_offers_pkey;

-- Drop the activities table primary key
ALTER TABLE nft_marketplace_activities
DROP CONSTRAINT IF EXISTS nft_marketplace_activities_pkey;

-- Restore original primary keys
ALTER TABLE current_nft_marketplace_listings
ADD CONSTRAINT current_nft_marketplace_listings_pkey 
PRIMARY KEY (token_data_id);

ALTER TABLE current_nft_marketplace_token_offers
ADD CONSTRAINT current_nft_marketplace_token_offers_pkey 
PRIMARY KEY (token_data_id, buyer);

ALTER TABLE current_nft_marketplace_collection_offers
ADD CONSTRAINT current_nft_marketplace_collection_offers_pkey 
PRIMARY KEY (collection_offer_id);
