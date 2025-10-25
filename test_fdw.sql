-- Corrently GrünstromIndex WASM FDW Test Suite
-- Generated: 2025-10-25
-- API: Corrently v2.0 (https://api.corrently.io)
-- Phase 4: Testing & Validation

-- ============================================
-- SETUP
-- ============================================

-- Enable wrappers extension
CREATE EXTENSION IF NOT EXISTS wrappers;

-- Create WASM foreign data wrapper (may already exist)
-- CREATE FOREIGN DATA WRAPPER wasm_wrapper
--   HANDLER wasm_fdw_handler
--   VALIDATOR wasm_fdw_validator;

-- Create foreign server
-- NOTE: Update fdw_package_checksum if binary changes
CREATE SERVER IF NOT EXISTS corrently_server
  FOREIGN DATA WRAPPER wasm_wrapper
  OPTIONS (
    fdw_package_url 'https://github.com/powabase/supabase-fdw-corrently/releases/download/v0.1.0/corrently_fdw.wasm',
    fdw_package_name 'powabase:supabase-fdw-corrently',
    fdw_package_version '0.1.0',
    fdw_package_checksum '0747c2f6e9da61d27581b30716d9faa5204044419a4f796d5fb943e23143da02',
    api_url 'https://api.corrently.io',
    api_key 'your_corrently_api_key_here'
  );

-- Create schema
CREATE SCHEMA IF NOT EXISTS fdw_corrently;

-- Create foreign table
CREATE FOREIGN TABLE IF NOT EXISTS fdw_corrently.gsi_prediction (
    epochtime bigint,
    timestamp bigint,
    timeframe_start bigint,
    timeframe_end bigint,
    gsi numeric,
    eevalue bigint,
    ewind bigint,
    esolar bigint,
    enwind bigint,
    ensolar bigint,
    sci bigint,
    energyprice numeric,
    co2_avg numeric,
    co2_g_standard bigint,
    co2_g_oekostrom bigint,
    zip text,
    iat bigint
)
SERVER corrently_server
OPTIONS (object 'gsi_prediction');

-- ============================================
-- BASIC TESTS
-- ============================================

\echo '\n=== Test 1: Basic query (Heidelberg 69168) ==='
\timing on
SELECT
  epochtime,
  TO_TIMESTAMP(timestamp / 1000) as forecast_time,
  gsi as green_index,
  eevalue as renewable_pct,
  energyprice as price_eur_kwh,
  zip
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168'
LIMIT 10;
-- Expected: 10 rows, ~300ms, all columns populated

\echo '\n=== Test 2: 24-hour forecast with hours parameter ==='
SELECT
  TO_TIMESTAMP(timestamp / 1000) as time,
  gsi,
  eevalue,
  energyprice,
  co2_g_standard
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168' AND hours = 24
ORDER BY timestamp
LIMIT 24;
-- Expected: ~24 rows, chronologically ordered

\echo '\n=== Test 3: Berlin (10117) ==='
SELECT
  TO_TIMESTAMP(timestamp / 1000) as time,
  gsi,
  sci as smart_city_index,
  co2_g_standard,
  co2_g_oekostrom,
  zip
FROM fdw_corrently.gsi_prediction
WHERE zip = '10117'
LIMIT 5;
-- Expected: 5 rows with zip='10117'

\echo '\n=== Test 4: ZIP 30455 ==='
SELECT
  TO_TIMESTAMP(timestamp / 1000) as time,
  gsi,
  eevalue,
  ewind,
  esolar
FROM fdw_corrently.gsi_prediction
WHERE zip = '30455'
LIMIT 5;
-- Expected: 5 rows with zip='30455'

\echo '\n=== Test 5: ZIP 30926 ==='
SELECT
  TO_TIMESTAMP(timestamp / 1000) as time,
  gsi,
  energyprice
FROM fdw_corrently.gsi_prediction
WHERE zip = '30926'
LIMIT 5;
-- Expected: 5 rows with zip='30926'

-- ============================================
-- COLUMN VALIDATION
-- ============================================

\echo '\n=== Test 6: All 17 columns validation ==='
SELECT
  epochtime,
  timestamp,
  timeframe_start,
  timeframe_end,
  gsi,
  eevalue,
  ewind,
  esolar,
  enwind,
  ensolar,
  sci,
  energyprice,
  co2_avg,
  co2_g_standard,
  co2_g_oekostrom,
  zip,
  iat
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168'
LIMIT 3;
-- Expected: 3 rows, ALL 17 columns populated, no NULLs

\echo '\n=== Test 7: Timeframe validation (nested JSON parsing) ==='
SELECT
  TO_TIMESTAMP(timeframe_start / 1000) as period_start,
  TO_TIMESTAMP(timeframe_end / 1000) as period_end,
  (timeframe_end - timeframe_start) / 1000 / 60 as duration_minutes,
  gsi
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168'
LIMIT 5;
-- Expected: duration_minutes = 60 (1-hour periods)

\echo '\n=== Test 8: String parsing (energyprice conversion) ==='
SELECT
  TO_TIMESTAMP(timestamp / 1000) as time,
  gsi,
  energyprice,
  CASE
    WHEN energyprice < 0 THEN 'Surplus (negative)'
    WHEN energyprice = 0 THEN 'Zero'
    ELSE 'Positive'
  END as price_category
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168'
ORDER BY energyprice
LIMIT 10;
-- Expected: May include negative prices, all valid numbers

-- ============================================
-- EDGE CASES
-- ============================================

\echo '\n=== Test 9: Missing zip parameter (should fail) ==='
-- This should return an error
SELECT * FROM fdw_corrently.gsi_prediction LIMIT 1;
-- Expected error: "zip parameter is required in WHERE clause"

-- ============================================
-- PERFORMANCE & AGGREGATION
-- ============================================

\echo '\n=== Test 10: Full forecast count ==='
SELECT COUNT(*) as total_forecast_hours
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168';
-- Expected: ~113 rows, < 3 seconds

\echo '\n=== Test 11: Aggregation statistics ==='
SELECT
  COUNT(*) as forecast_hours,
  MIN(gsi) as min_green_index,
  MAX(gsi) as max_green_index,
  AVG(gsi) as avg_green_index,
  MIN(energyprice) as min_price,
  MAX(energyprice) as max_price,
  AVG(energyprice) as avg_price,
  AVG(co2_g_standard) as avg_co2_standard,
  AVG(co2_g_oekostrom) as avg_co2_green
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168';
-- Expected: Single row with aggregated values

\echo '\n=== Test 12: Time-based filtering (next 48 hours) ==='
SELECT
  TO_TIMESTAMP(timestamp / 1000) as time,
  gsi,
  energyprice
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168'
  AND timestamp / 1000 < EXTRACT(EPOCH FROM NOW() + INTERVAL '48 hours')
ORDER BY timestamp
LIMIT 48;
-- Expected: ~48 rows, chronologically ordered

\timing off

-- ============================================
-- CLEANUP (Optional)
-- ============================================

-- Uncomment to remove test objects:
-- DROP FOREIGN TABLE IF EXISTS fdw_corrently.gsi_prediction;
-- DROP SERVER IF EXISTS corrently_server CASCADE;
-- DROP SCHEMA IF EXISTS fdw_corrently CASCADE;

\echo '\n=== Test Suite Complete ==='
\echo 'Review results above for any failures or unexpected values'
