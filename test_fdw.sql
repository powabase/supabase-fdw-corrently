-- Corrently GrünstromIndex WASM FDW Test Suite
-- Version: 0.2.0
-- Generated: 2025-10-28
-- API: Corrently v2.0 (https://api.corrently.io)
--
-- BREAKING CHANGES in v0.2.0:
-- - Standardized column names (e.g., zip → postal_code, gsi → green_energy_index)
-- - Temporal fields now use TIMESTAMP WITH TIME ZONE (native PostgreSQL types)
-- - Removed redundant epochtime field (use forecast_start_time)
-- - See MIGRATION.md for complete upgrade guide

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
    fdw_package_url 'https://github.com/powabase/supabase-fdw-corrently/releases/download/v0.2.0/corrently_fdw.wasm',
    fdw_package_name 'powabase:supabase-fdw-corrently',
    fdw_package_version '0.2.0',
    fdw_package_checksum '6f182a640568669afa6294641aa074bb13a332b146516ae199505ff470d94b18',
    api_url 'https://api.corrently.io',
    api_key 'your_corrently_api_key_here'
  );

-- Create schema
CREATE SCHEMA IF NOT EXISTS fdw_corrently;

-- Create foreign table (v0.2.0 - standardized column names)
CREATE FOREIGN TABLE IF NOT EXISTS fdw_corrently.gsi_prediction (
    forecast_start_time timestamp with time zone,
    forecast_period_start timestamp with time zone,
    forecast_period_end timestamp with time zone,
    green_energy_index numeric,
    renewable_energy_pct bigint,
    wind_energy_pct bigint,
    solar_energy_pct bigint,
    net_wind_energy_pct bigint,
    net_solar_energy_pct bigint,
    smart_city_index bigint,
    energy_price_eur_kwh numeric,
    co2_baseline_g_kwh numeric,
    standard_mix_co2_g_kwh bigint,
    green_mix_co2_g_kwh bigint,
    postal_code text,
    forecast_created_at timestamp with time zone
)
SERVER corrently_server
OPTIONS (object 'gsi_prediction');

-- ============================================
-- BASIC TESTS
-- ============================================

\echo '\n=== Test 1: Basic query (Heidelberg 69168) ==='
\timing on
SELECT
  forecast_start_time,
  green_energy_index,
  renewable_energy_pct,
  energy_price_eur_kwh,
  postal_code
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '69168'
LIMIT 10;
-- Expected: 10 rows, ~300ms, all columns populated, timestamps in TIMESTAMP WITH TIME ZONE format

\echo '\n=== Test 2: 24-hour forecast with hours parameter ==='
SELECT
  forecast_start_time,
  green_energy_index,
  renewable_energy_pct,
  energy_price_eur_kwh,
  standard_mix_co2_g_kwh
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '69168' AND hours = 24
ORDER BY forecast_start_time
LIMIT 24;
-- Expected: ~24 rows, chronologically ordered

\echo '\n=== Test 3: Berlin (10117) ==='
SELECT
  forecast_start_time,
  green_energy_index,
  smart_city_index,
  standard_mix_co2_g_kwh,
  green_mix_co2_g_kwh,
  postal_code
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '10117'
LIMIT 5;
-- Expected: 5 rows with postal_code='10117'

\echo '\n=== Test 4: ZIP 30455 ==='
SELECT
  forecast_start_time,
  green_energy_index,
  renewable_energy_pct,
  wind_energy_pct,
  solar_energy_pct
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '30455'
LIMIT 5;
-- Expected: 5 rows with postal_code='30455'

\echo '\n=== Test 5: ZIP 30926 ==='
SELECT
  forecast_start_time,
  green_energy_index,
  energy_price_eur_kwh
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '30926'
LIMIT 5;
-- Expected: 5 rows with postal_code='30926'

-- ============================================
-- COLUMN VALIDATION
-- ============================================

\echo '\n=== Test 6: All 16 columns validation (v0.2.0) ==='
SELECT
  forecast_start_time,
  forecast_period_start,
  forecast_period_end,
  green_energy_index,
  renewable_energy_pct,
  wind_energy_pct,
  solar_energy_pct,
  net_wind_energy_pct,
  net_solar_energy_pct,
  smart_city_index,
  energy_price_eur_kwh,
  co2_baseline_g_kwh,
  standard_mix_co2_g_kwh,
  green_mix_co2_g_kwh,
  postal_code,
  forecast_created_at
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '69168'
LIMIT 3;
-- Expected: 3 rows, ALL 16 columns populated, no NULLs (epochtime removed in v0.2.0)

\echo '\n=== Test 7: Timeframe validation (forecast periods) ==='
SELECT
  forecast_period_start,
  forecast_period_end,
  EXTRACT(EPOCH FROM (forecast_period_end - forecast_period_start)) / 60 as duration_minutes,
  green_energy_index
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '69168'
LIMIT 5;
-- Expected: duration_minutes = 60 (1-hour periods), native TIMESTAMP operations

\echo '\n=== Test 8: String parsing (energy_price_eur_kwh conversion) ==='
SELECT
  forecast_start_time,
  green_energy_index,
  energy_price_eur_kwh,
  CASE
    WHEN energy_price_eur_kwh < 0 THEN 'Surplus (negative)'
    WHEN energy_price_eur_kwh = 0 THEN 'Zero'
    ELSE 'Positive'
  END as price_category
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '69168'
ORDER BY energy_price_eur_kwh
LIMIT 10;
-- Expected: May include negative prices, all valid numbers

-- ============================================
-- EDGE CASES
-- ============================================

\echo '\n=== Test 9: Missing postal_code parameter (should fail) ==='
-- This should return an error
SELECT * FROM fdw_corrently.gsi_prediction LIMIT 1;
-- Expected error: "postal_code parameter is required in WHERE clause"

-- ============================================
-- PERFORMANCE & AGGREGATION
-- ============================================

\echo '\n=== Test 10: Full forecast count ==='
SELECT COUNT(*) as total_forecast_hours
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '69168';
-- Expected: ~113 rows, < 3 seconds

\echo '\n=== Test 11: Aggregation statistics ==='
SELECT
  COUNT(*) as forecast_hours,
  MIN(green_energy_index) as min_green_index,
  MAX(green_energy_index) as max_green_index,
  AVG(green_energy_index) as avg_green_index,
  MIN(energy_price_eur_kwh) as min_price,
  MAX(energy_price_eur_kwh) as max_price,
  AVG(energy_price_eur_kwh) as avg_price,
  AVG(standard_mix_co2_g_kwh) as avg_co2_standard,
  AVG(green_mix_co2_g_kwh) as avg_co2_green
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '69168';
-- Expected: Single row with aggregated values

\echo '\n=== Test 12: Time-based filtering with native TIMESTAMP (next 48 hours) ==='
SELECT
  forecast_start_time,
  green_energy_index,
  energy_price_eur_kwh
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '69168'
  AND forecast_start_time < NOW() + INTERVAL '48 hours'
ORDER BY forecast_start_time
LIMIT 48;
-- Expected: ~48 rows, chronologically ordered, using native TIMESTAMP operations

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
