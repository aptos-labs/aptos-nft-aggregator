-- Your SQL goes here

-- Drop existing primary key constraints
ALTER TABLE current_nft_marketplace_listings 
DROP CONSTRAINT IF EXISTS current_nft_marketplace_listings_pkey;

ALTER TABLE current_nft_marketplace_token_offers
DROP CONSTRAINT IF EXISTS current_nft_marketplace_token_offers_pkey;

ALTER TABLE current_nft_marketplace_collection_offers
DROP CONSTRAINT IF EXISTS current_nft_marketplace_collection_offers_pkey;

ALTER TABLE nft_marketplace_activities
DROP CONSTRAINT IF EXISTS nft_marketplace_activities_pkey;

-- Add primary key to activities table
ALTER TABLE nft_marketplace_activities
ADD CONSTRAINT nft_marketplace_activities_pkey 
PRIMARY KEY (txn_version, index);

-- Add new composite primary keys including marketplace
ALTER TABLE current_nft_marketplace_listings
ADD CONSTRAINT current_nft_marketplace_listings_pkey 
PRIMARY KEY (token_data_id, marketplace);

ALTER TABLE current_nft_marketplace_token_offers 
ADD CONSTRAINT current_nft_marketplace_token_offers_pkey
PRIMARY KEY (token_data_id, buyer, marketplace);

ALTER TABLE current_nft_marketplace_collection_offers
ADD CONSTRAINT current_nft_marketplace_collection_offers_pkey
PRIMARY KEY (collection_offer_id, marketplace);
