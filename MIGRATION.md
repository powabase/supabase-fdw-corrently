# Migration Guide: v0.1.0 → v0.2.0

## Overview

Version 0.2.0 introduces **breaking changes** to standardize column names and improve PostgreSQL compatibility. All column names have been standardized, and temporal fields now use native PostgreSQL `TIMESTAMP WITH TIME ZONE` types instead of `BIGINT` (Unix timestamps).

**Migration Effort:** ~30 minutes (update queries + test)
**Benefits:** Cleaner SQL, better AI agent support, consistent with other Powabase data sources

---

## Summary of Changes

### 1. Column Renames (17 columns → 16 columns)

- **Removed:** `epochtime` (redundant with `timestamp`)
- **Renamed:** All 16 remaining columns for clarity and standards compliance
- **Type Changes:** 4 temporal columns now use `TIMESTAMP WITH TIME ZONE`

### 2. Query Simplification

**Before (v0.1.0):**
```sql
WHERE zip = '69168'
  AND TO_TIMESTAMP(timestamp / 1000) > NOW() - INTERVAL '24 hours'
```

**After (v0.2.0):**
```sql
WHERE postal_code = '69168'
  AND forecast_start_time > NOW() - INTERVAL '24 hours'
```

Native `TIMESTAMP WITH TIME ZONE` enables direct time comparisons without conversion functions!

---

## Complete Column Mapping

| v0.1.0 (Old) | v0.2.0 (New) | Type Change | Category |
|--------------|--------------|-------------|----------|
| ~~`epochtime`~~ | *(removed)* | — | Redundant |
| `timestamp` | `forecast_start_time` | BIGINT → TIMESTAMPTZ | Temporal |
| `timeframe_start` | `forecast_period_start` | BIGINT → TIMESTAMPTZ | Temporal |
| `timeframe_end` | `forecast_period_end` | BIGINT → TIMESTAMPTZ | Temporal |
| `iat` | `forecast_created_at` | BIGINT → TIMESTAMPTZ | Temporal |
| `gsi` | `green_energy_index` | NUMERIC (no change) | Energy |
| `eevalue` | `renewable_energy_pct` | BIGINT (no change) | Energy |
| `ewind` | `wind_energy_pct` | BIGINT (no change) | Energy |
| `esolar` | `solar_energy_pct` | BIGINT (no change) | Energy |
| `enwind` | `net_wind_energy_pct` | BIGINT (no change) | Energy |
| `ensolar` | `net_solar_energy_pct` | BIGINT (no change) | Energy |
| `sci` | `smart_city_index` | BIGINT (no change) | Energy |
| `energyprice` | `energy_price_eur_kwh` | NUMERIC (no change) | Pricing |
| `co2_avg` | `co2_baseline_g_kwh` | NUMERIC (no change) | CO2 |
| `co2_g_standard` | `standard_mix_co2_g_kwh` | BIGINT (no change) | CO2 |
| `co2_g_oekostrom` | `green_mix_co2_g_kwh` | BIGINT (no change) | CO2 |
| `zip` | `postal_code` | TEXT (no change) | Geographic |

**Total:** 17 columns (v0.1.0) → 16 columns (v0.2.0)

---

## Migration Steps

### Step 1: Update Foreign Server (Required)

Update the WASM binary URL to v0.2.0:

```sql
ALTER SERVER corrently_server OPTIONS (
  SET fdw_package_url 'https://github.com/powabase/supabase-fdw-corrently/releases/download/v0.2.0/corrently_fdw.wasm',
  SET fdw_package_version '0.2.0',
  SET fdw_package_checksum '6f182a640568669afa6294641aa074bb13a332b146516ae199505ff470d94b18'
);
```

### Step 2: Drop and Recreate Foreign Table

**CRITICAL:** The foreign table schema has changed. You must drop and recreate it.

```sql
-- Drop existing table
DROP FOREIGN TABLE IF EXISTS fdw_corrently.gsi_prediction;

-- Recreate with v0.2.0 schema
CREATE FOREIGN TABLE fdw_corrently.gsi_prediction (
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
```

### Step 3: Update Your Queries

Use the "Query Examples" section below to update your application queries.

### Step 4: Test

Run a simple query to verify the migration:

```sql
SELECT
  forecast_start_time,
  green_energy_index,
  renewable_energy_pct,
  energy_price_eur_kwh,
  postal_code
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '69168'
LIMIT 5;
```

**Expected:** Timestamps display as `'2025-10-28 14:00:00+00'` (not as Unix milliseconds).

---

## Query Examples: Before & After

### Example 1: Basic Forecast Query

**Before (v0.1.0):**
```sql
SELECT
  TO_TIMESTAMP(timestamp / 1000) as forecast_time,
  gsi as green_index,
  eevalue as renewable_pct,
  energyprice as price_eur_kwh,
  co2_g_standard as co2_grams_kwh
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168'
LIMIT 10;
```

**After (v0.2.0):**
```sql
SELECT
  forecast_start_time,
  green_energy_index,
  renewable_energy_pct,
  energy_price_eur_kwh,
  standard_mix_co2_g_kwh
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '69168'
LIMIT 10;
```

**Changes:**
- ✅ Removed `TO_TIMESTAMP()` conversion (timestamps are native!)
- ✅ Renamed columns for clarity
- ✅ Updated `WHERE` clause (`zip` → `postal_code`)

---

### Example 2: EV Charging Optimization

**Before (v0.1.0):**
```sql
SELECT
  TO_TIMESTAMP(timestamp / 1000) as time,
  gsi,
  eevalue as renewable_pct,
  energyprice,
  CASE
    WHEN gsi > 70 AND energyprice < 0 THEN 'Excellent'
    WHEN gsi > 50 AND energyprice < 0.05 THEN 'Good'
    ELSE 'OK'
  END as charging_rating
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168' AND gsi > 50
ORDER BY gsi DESC, energyprice ASC
LIMIT 10;
```

**After (v0.2.0):**
```sql
SELECT
  forecast_start_time,
  green_energy_index,
  renewable_energy_pct,
  energy_price_eur_kwh,
  CASE
    WHEN green_energy_index > 70 AND energy_price_eur_kwh < 0 THEN 'Excellent'
    WHEN green_energy_index > 50 AND energy_price_eur_kwh < 0.05 THEN 'Good'
    ELSE 'OK'
  END as charging_rating
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '69168' AND green_energy_index > 50
ORDER BY green_energy_index DESC, energy_price_eur_kwh ASC
LIMIT 10;
```

---

### Example 3: Time-Based Filtering (Major Improvement!)

**Before (v0.1.0):**
```sql
SELECT
  TO_TIMESTAMP(timestamp / 1000) as time,
  gsi,
  energyprice
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168'
  AND timestamp / 1000 < EXTRACT(EPOCH FROM NOW() + INTERVAL '48 hours')
ORDER BY timestamp
LIMIT 48;
```

**After (v0.2.0):**
```sql
SELECT
  forecast_start_time,
  green_energy_index,
  energy_price_eur_kwh
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '69168'
  AND forecast_start_time < NOW() + INTERVAL '48 hours'
ORDER BY forecast_start_time
LIMIT 48;
```

**Changes:**
- ✅ **Native TIMESTAMP operations!** No more `EXTRACT(EPOCH ...)` gymnastics
- ✅ Direct `< NOW() + INTERVAL` comparisons
- ✅ Cleaner `ORDER BY` (no conversion needed)

---

### Example 4: Aggregation Statistics

**Before (v0.1.0):**
```sql
SELECT
  COUNT(*) as forecast_hours,
  MIN(gsi) as min_green_index,
  MAX(gsi) as max_green_index,
  ROUND(AVG(gsi), 1) as avg_green_index,
  MIN(energyprice) as min_price,
  MAX(energyprice) as max_price,
  ROUND(AVG(energyprice), 4) as avg_price,
  ROUND(AVG(co2_g_standard), 0) as avg_co2_standard,
  ROUND(AVG(co2_g_oekostrom), 0) as avg_co2_green
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168';
```

**After (v0.2.0):**
```sql
SELECT
  COUNT(*) as forecast_hours,
  MIN(green_energy_index) as min_green_index,
  MAX(green_energy_index) as max_green_index,
  ROUND(AVG(green_energy_index), 1) as avg_green_index,
  MIN(energy_price_eur_kwh) as min_price,
  MAX(energy_price_eur_kwh) as max_price,
  ROUND(AVG(energy_price_eur_kwh), 4) as avg_price,
  ROUND(AVG(standard_mix_co2_g_kwh), 0) as avg_co2_standard,
  ROUND(AVG(green_mix_co2_g_kwh), 0) as avg_co2_green
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '69168';
```

---

### Example 5: Carbon Footprint Analysis

**Before (v0.1.0):**
```sql
SELECT
  AVG(co2_g_standard) as avg_co2_standard,
  AVG(co2_g_oekostrom) as avg_co2_green,
  AVG(co2_g_standard - co2_g_oekostrom) as avg_savings_g_kwh
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168';
```

**After (v0.2.0):**
```sql
SELECT
  AVG(standard_mix_co2_g_kwh) as avg_co2_standard,
  AVG(green_mix_co2_g_kwh) as avg_co2_green,
  AVG(standard_mix_co2_g_kwh - green_mix_co2_g_kwh) as avg_savings_g_kwh
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '69168';
```

---

### Example 6: Renewable Energy Breakdown

**Before (v0.1.0):**
```sql
SELECT
  TO_TIMESTAMP(timestamp / 1000) as time,
  eevalue as total_renewable_pct,
  ewind as wind_pct,
  esolar as solar_pct,
  ROUND((ewind::numeric / NULLIF(eevalue, 0)) * 100, 1) as wind_share
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168'
ORDER BY timestamp
LIMIT 20;
```

**After (v0.2.0):**
```sql
SELECT
  forecast_start_time,
  renewable_energy_pct,
  wind_energy_pct,
  solar_energy_pct,
  ROUND((wind_energy_pct::numeric / NULLIF(renewable_energy_pct, 0)) * 100, 1) as wind_share
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '69168'
ORDER BY forecast_start_time
LIMIT 20;
```

---

### Example 7: Forecast Period Duration Calculation

**Before (v0.1.0):**
```sql
SELECT
  TO_TIMESTAMP(timeframe_start / 1000) as period_start,
  TO_TIMESTAMP(timeframe_end / 1000) as period_end,
  (timeframe_end - timeframe_start) / 1000 / 60 as duration_minutes
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168'
LIMIT 5;
```

**After (v0.2.0):**
```sql
SELECT
  forecast_period_start,
  forecast_period_end,
  EXTRACT(EPOCH FROM (forecast_period_end - forecast_period_start)) / 60 as duration_minutes
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '69168'
LIMIT 5;
```

**Changes:**
- ✅ **Native TIMESTAMP subtraction!** PostgreSQL calculates the interval
- ✅ Clearer column names (`forecast_period_*` vs `timeframe_*`)

---

## Compatibility Notes

### Removed Field: `epochtime`

The `epochtime` column (Unix timestamp in **seconds**) has been removed because it was redundant with `timestamp` (milliseconds). Both represented the same instant in time.

**Migration:**
- If you used `epochtime`, replace it with `forecast_start_time`
- No conversion needed! `forecast_start_time` is now a native `TIMESTAMP WITH TIME ZONE`

**Before (v0.1.0):**
```sql
SELECT epochtime, TO_TIMESTAMP(epochtime) as time
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168';
```

**After (v0.2.0):**
```sql
SELECT forecast_start_time
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '69168';
```

---

## Backwards Compatibility

**There is NO backwards compatibility.** v0.2.0 is a **hard break** from v0.1.0.

### Why Not Provide Compatibility Views?

1. **WASM FDW Limitation:** Foreign tables cannot have aliased columns
2. **Clean Migration:** A one-time update is cleaner than maintaining dual schemas
3. **Standards Alignment:** Full compliance with Powabase database design standards

### Migration Timeline Recommendation

- **Development:** Migrate immediately (low impact)
- **Staging:** Test within 1 week
- **Production:** Plan for a maintenance window (queries will break until updated)

---

## Testing Checklist

After migrating, verify these scenarios:

- [ ] Basic query returns 10 rows with `postal_code = '69168'`
- [ ] Timestamps display as `'2025-10-28 14:00:00+00'` (not Unix milliseconds)
- [ ] Time filtering works: `WHERE forecast_start_time > NOW() - INTERVAL '24 hours'`
- [ ] Aggregations return correct statistics (MIN, MAX, AVG)
- [ ] Error handling: Query without `postal_code` fails with clear error message
- [ ] Existing application queries updated and tested

---

## Rollback Plan

If you encounter issues and need to rollback to v0.1.0:

```sql
-- Rollback server to v0.1.0
ALTER SERVER corrently_server OPTIONS (
  SET fdw_package_url 'https://github.com/powabase/supabase-fdw-corrently/releases/download/v0.1.0/corrently_fdw.wasm',
  SET fdw_package_version '0.1.0',
  SET fdw_package_checksum '0747c2f6e9da61d27581b30716d9faa5204044419a4f796d5fb943e23143da02'
);

-- Drop v0.2.0 table
DROP FOREIGN TABLE IF EXISTS fdw_corrently.gsi_prediction;

-- Recreate v0.1.0 table
CREATE FOREIGN TABLE fdw_corrently.gsi_prediction (
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
```

---

## Support

- **GitHub Issues:** [Report migration problems](https://github.com/powabase/supabase-fdw-corrently/issues)
- **Documentation:** See [README.md](README.md) for v0.2.0 usage examples

---

## Why This Change?

**Standards Compliance:** Follows PostgreSQL best practices for:
- Consistent, descriptive column naming
- Native PostgreSQL types for better query optimization
- AI-friendly column names for automated query generation
- Explicit units in column names (e.g., `_eur_kwh`, `_g_kwh`, `_pct`)

**Long-Term Benefits:**
- Easier to join Corrently data with other Powabase sources
- Better PostgreSQL query planner optimizations with native types
- Simplified SQL (no `TO_TIMESTAMP()` conversions)
- Future-proof for cross-source queries and analytics

---

**Migration Guide Version:** 1.0
**Created:** 2025-10-28
**FDW Version:** v0.1.0 → v0.2.0
