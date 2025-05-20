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
PRIMARY KEY (txn_version, index, marketplace);

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

-- Drop columns if they exist (one per statement)
ALTER TABLE nft_marketplace_activities DROP COLUMN IF EXISTS token_standard;

ALTER TABLE current_nft_marketplace_listings DROP COLUMN IF EXISTS token_standard;
ALTER TABLE current_nft_marketplace_listings ADD COLUMN IF NOT EXISTS standard_event_type VARCHAR DEFAULT 'unknown';

ALTER TABLE current_nft_marketplace_listings 
ALTER COLUMN standard_event_type SET NOT NULL;

ALTER TABLE current_nft_marketplace_token_offers DROP COLUMN IF EXISTS token_standard;
ALTER TABLE current_nft_marketplace_token_offers ADD COLUMN IF NOT EXISTS standard_event_type VARCHAR DEFAULT 'unknown';
ALTER TABLE current_nft_marketplace_token_offers 
ALTER COLUMN standard_event_type SET NOT NULL;

ALTER TABLE current_nft_marketplace_collection_offers DROP COLUMN IF EXISTS token_standard;
ALTER TABLE current_nft_marketplace_collection_offers ADD COLUMN IF NOT EXISTS standard_event_type VARCHAR DEFAULT 'unknown';
ALTER TABLE current_nft_marketplace_collection_offers 
ALTER COLUMN standard_event_type SET NOT NULL;

ALTER TABLE current_nft_marketplace_collection_offers ADD COLUMN IF NOT EXISTS token_data_id VARCHAR(66);

ALTER TABLE current_nft_marketplace_listings ADD COLUMN IF NOT EXISTS token_name VARCHAR;