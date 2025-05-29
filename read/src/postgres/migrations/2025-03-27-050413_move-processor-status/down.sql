-- This file should undo anything in `up.sql`
-- Drop the schema (only if empty)

-- Ensure the public tables exist before restoring data
ALTER TABLE processor_status DROP CONSTRAINT IF EXISTS processor_status_pkey;


CREATE TABLE IF NOT EXISTS public.processor_status AS TABLE processor_metadata.processor_status WITH NO DATA;

-- Restore data to public tables
INSERT INTO public.processor_status SELECT * FROM processor_metadata.processor_status;

-- Drop the tables in processor_metadata schema
DROP TABLE IF EXISTS processor_metadata.processor_status;
DROP TABLE IF EXISTS processor_metadata.ledger_infos;

DROP SCHEMA IF EXISTS processor_metadata;

DROP TABLE IF EXISTS backfill_processor_status;
