-- This file should undo anything in `up.sql`
DROP TABLE IF EXISTS nft_marketplace_activities;
DROP INDEX IF EXISTS idx_nft_marketplace_activities;
DROP TABLE IF EXISTS processor_status;