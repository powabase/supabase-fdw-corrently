# Quickstart Guide

Get Corrently GrünstromIndex green energy forecasts in your Supabase database in 3 minutes.

## Prerequisites

- Existing Supabase project (local or hosted)
- Supabase CLI installed (for local testing)
- Corrently API key - **[Get your free API key](https://console.corrently.io/)** (2,000 requests/day)

## Step 1: Create Foreign Server (1 min)

Connect to your Supabase database and run:

```sql
-- Enable wrappers extension
CREATE EXTENSION IF NOT EXISTS wrappers WITH SCHEMA extensions;

-- Create WASM FDW wrapper
CREATE FOREIGN DATA WRAPPER IF NOT EXISTS wasm_wrapper
  HANDLER wasm_fdw_handler
  VALIDATOR wasm_fdw_validator;

-- Create Corrently server
CREATE SERVER corrently_server
  FOREIGN DATA WRAPPER wasm_wrapper
  OPTIONS (
    fdw_package_url 'https://github.com/powabase/supabase-fdw-corrently/releases/download/v0.1.0/corrently_fdw.wasm',
    fdw_package_name 'powabase:supabase-fdw-corrently',
    fdw_package_version '0.1.0',
    fdw_package_checksum '0747c2f6e9da61d27581b30716d9faa5204044419a4f796d5fb943e23143da02',
    api_url 'https://api.corrently.io',
    api_key 'your_corrently_api_key_here'  -- Replace with your API key from console.corrently.io
  );
```

### API Key Setup

1. Sign up at [console.corrently.io](https://console.corrently.io/)
2. Get your JWT token (looks like `eyJhbGci...`)
3. Replace `your_corrently_api_key_here` in the SQL above

**Free tier:** 2,000 requests/day

## Step 2: Create Foreign Table (1 min)

Create the foreign table with all 17 forecast columns:

```sql
-- Create schema
CREATE SCHEMA IF NOT EXISTS fdw_corrently;

-- Create gsi_prediction table
CREATE FOREIGN TABLE fdw_corrently.gsi_prediction (
  -- Timestamps
  epochtime bigint,              -- Unix timestamp (seconds)
  timestamp bigint,              -- Unix timestamp (milliseconds)
  timeframe_start bigint,        -- Period start (ms)
  timeframe_end bigint,          -- Period end (ms)

  -- Green Energy Metrics
  gsi numeric,                   -- GrünstromIndex (0-100)
  eevalue bigint,                -- Total renewable energy %
  ewind bigint,                  -- Wind energy %
  esolar bigint,                 -- Solar energy %
  enwind bigint,                 -- Net wind energy %
  ensolar bigint,                -- Net solar energy %
  sci bigint,                    -- Smart City Index (0-100)

  -- Pricing & CO2
  energyprice numeric,           -- EUR/kWh (can be negative!)
  co2_avg numeric,               -- Average CO2 baseline (g/kWh)
  co2_g_standard bigint,         -- CO2 standard mix (g/kWh)
  co2_g_oekostrom bigint,        -- CO2 green mix (g/kWh)

  -- Metadata
  zip text,                      -- German postal code
  iat bigint                     -- Issued-at timestamp (ms)
)
SERVER corrently_server
OPTIONS (object 'gsi_prediction');

-- Grant permissions
GRANT USAGE ON SCHEMA fdw_corrently TO postgres;
GRANT SELECT ON fdw_corrently.gsi_prediction TO postgres;
```

## Step 3: Query Data (1 min)

Run your first query! Get hourly green energy forecasts for Heidelberg (postal code 69168):

```sql
SELECT
  TO_TIMESTAMP(timestamp / 1000) as forecast_time,
  gsi as green_index,
  eevalue as renewable_pct,
  energyprice as price_eur_kwh,
  co2_g_standard as co2_grams_kwh
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168'
LIMIT 5;
```

**Expected output:**

| forecast_time       | green_index | renewable_pct | price_eur_kwh | co2_grams_kwh |
|---------------------|-------------|---------------|---------------|---------------|
| 2025-10-25 14:00:00 | 26.6        | 28            | -0.014        | 233           |
| 2025-10-25 15:00:00 | 32.1        | 34            | -0.021        | 215           |
| 2025-10-25 16:00:00 | 45.8        | 48            | -0.008        | 184           |
| ...                 | ...         | ...           | ...           | ...           |

**Query time:** ~300-400ms

**Rows returned:** ~113 (full forecast horizon: 4.7 days)

---

## Common Use Cases

### Find Optimal EV Charging Times

Schedule charging when green energy is high and prices are low:

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
WHERE zip = '69168'
  AND gsi > 50
ORDER BY gsi DESC, energyprice ASC
LIMIT 10;
```

### 24-Hour Forecast

Limit results to next 24 hours:

```sql
SELECT
  TO_TIMESTAMP(timestamp / 1000) as time,
  gsi,
  eevalue as renewable_pct,
  ewind as wind_pct,
  esolar as solar_pct
FROM fdw_corrently.gsi_prediction
WHERE zip = '10117' AND hours = 24
ORDER BY timestamp
LIMIT 24;
```

### Carbon Footprint Analysis

Compare CO2 emissions between standard and green energy mix:

```sql
SELECT
  AVG(co2_g_standard) as avg_co2_standard,
  AVG(co2_g_oekostrom) as avg_co2_green,
  ROUND(AVG(co2_g_standard - co2_g_oekostrom), 0) as avg_savings_g_kwh,
  ROUND((AVG(co2_g_standard - co2_g_oekostrom) / AVG(co2_g_standard)::numeric) * 100, 1) as savings_pct
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168';
```

### Identify Negative Price Periods

Find times with surplus renewable energy (negative prices):

```sql
SELECT
  TO_TIMESTAMP(timestamp / 1000) as time,
  gsi,
  eevalue as renewable_pct,
  energyprice,
  CASE
    WHEN energyprice < 0 THEN 'Surplus (negative)'
    ELSE 'Positive'
  END as price_category
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168'
  AND energyprice < 0
ORDER BY energyprice ASC;
```

---

## Available German Postal Codes

This FDW works with **any valid German postal code (PLZ)**. Examples:

| City         | Postal Code | Example Usage                      |
|--------------|-------------|------------------------------------|
| Heidelberg   | 69168       | `WHERE zip = '69168'`              |
| Berlin       | 10117       | `WHERE zip = '10117'`              |
| Hamburg      | 20095       | `WHERE zip = '20095'`              |
| Munich       | 80331       | `WHERE zip = '80331'`              |
| Hannover     | 30159       | `WHERE zip = '30159'`              |
| Frankfurt    | 60311       | `WHERE zip = '60311'`              |

**Note:** The `zip` parameter is **required** in the WHERE clause.

---

## Troubleshooting

### NULL values in results?

**Cause:** WASM binary not accessible or checksum mismatch

**Solution:**
1. Verify checksum matches: `0747c2f6e9da61d27581b30716d9faa5204044419a4f796d5fb943e23143da02`
2. Check WASM binary URL is accessible
3. For local testing, see "Local Development" below

### Missing zip parameter error?

**Error:** `zip parameter is required in WHERE clause`

**Solution:** Always include `WHERE zip = '12345'` in your queries:

```sql
-- ❌ This will fail
SELECT * FROM fdw_corrently.gsi_prediction LIMIT 5;

-- ✅ This works
SELECT * FROM fdw_corrently.gsi_prediction WHERE zip = '69168' LIMIT 5;
```

### Authentication errors?

**Cause:** Invalid or missing API key

**Solution:**
1. Get API key from [console.corrently.io](https://console.corrently.io/)
2. Update server options:
   ```sql
   ALTER SERVER corrently_server
   OPTIONS (SET api_key 'your_new_api_key_here');
   ```
3. Verify key is a JWT token (starts with `eyJ`)

### Slow queries?

**Expected:** ~300-400ms per query (includes API call to Corrently)

**Optimization tips:**
1. Use `hours` parameter to limit results:
   ```sql
   WHERE zip = '69168' AND hours = 24
   ```
2. Add `LIMIT` for exploration queries
3. Create materialized views for frequently accessed data:
   ```sql
   CREATE MATERIALIZED VIEW my_forecast AS
   SELECT * FROM fdw_corrently.gsi_prediction
   WHERE zip = '69168';

   -- Refresh periodically (e.g., hourly)
   REFRESH MATERIALIZED VIEW my_forecast;
   ```

### No data returned?

**Possible causes:**
1. Invalid German postal code (must be 5-digit valid PLZ)
2. API rate limit exceeded (2,000 requests/day)
3. Network connectivity issues

**Debugging:**
1. Test API directly:
   ```bash
   curl "https://api.corrently.io/v2.0/gsi/prediction?zip=69168&token=YOUR_API_KEY"
   ```
2. Check Supabase logs:
   - Local: `supabase logs --db`
   - Hosted: Dashboard → Database → Logs

### Permission denied?

**Solution:** Grant access to your role:

```sql
-- Grant schema access
GRANT USAGE ON SCHEMA fdw_corrently TO your_role_name;

-- Grant table access
GRANT SELECT ON fdw_corrently.gsi_prediction TO your_role_name;
```

---

## Local Development

For local Supabase testing:

### Start Supabase

```bash
# Start local Supabase instance
supabase start

# Connect to database
psql postgresql://postgres:postgres@127.0.0.1:54322/postgres
```

### Build and Serve WASM Locally

```bash
# Build WASM binary
cargo component build --release --target wasm32-unknown-unknown

# Serve via HTTP
cd target/wasm32-unknown-unknown/release
python3 -m http.server 8000 &

# Verify accessibility
curl -I http://localhost:8000/corrently_fdw.wasm
```

### Create Server with Local URL

```sql
CREATE SERVER corrently_server
  FOREIGN DATA WRAPPER wasm_wrapper
  OPTIONS (
    fdw_package_url 'http://host.docker.internal:8000/corrently_fdw.wasm',
    fdw_package_name 'powabase:supabase-fdw-corrently',
    fdw_package_version '0.1.0',
    fdw_package_checksum '0747c2f6e9da61d27581b30716d9faa5204044419a4f796d5fb943e23143da02',
    api_url 'https://api.corrently.io',
    api_key 'your_corrently_api_key_here'
  );
```

**Note:** Use `host.docker.internal` to access localhost from Docker containers.

---

## Performance Tips

### Use WHERE Clause Pushdown

Filter at the API level for better performance:

```sql
-- ✅ Good - API only returns 24 hours
SELECT * FROM fdw_corrently.gsi_prediction
WHERE zip = '69168' AND hours = 24;

-- ⚠️ Less efficient - API returns ~113 hours, filtering happens in PostgreSQL
SELECT * FROM fdw_corrently.gsi_prediction
WHERE zip = '69168'
LIMIT 24;
```

### Create Materialized Views

Cache frequently accessed forecasts:

```sql
CREATE MATERIALIZED VIEW berlin_forecast AS
SELECT
  TO_TIMESTAMP(timestamp / 1000) as time,
  gsi,
  eevalue as renewable_pct,
  energyprice,
  co2_g_standard
FROM fdw_corrently.gsi_prediction
WHERE zip = '10117';

-- Create index for faster queries
CREATE INDEX ON berlin_forecast (time);

-- Refresh hourly (set up with pg_cron or external scheduler)
REFRESH MATERIALIZED VIEW berlin_forecast;
```

### Add LIMIT for Exploration

Use LIMIT when exploring data:

```sql
-- Exploration query
SELECT * FROM fdw_corrently.gsi_prediction
WHERE zip = '69168'
LIMIT 5;
```

---

## Next Steps

- **More examples:** See [README.md](README.md) usage examples
- **Endpoint reference:** See [docs/endpoints/gsi-prediction.md](docs/endpoints/gsi-prediction.md)
- **Complete test suite:** See [test_fdw.sql](test_fdw.sql) (12 comprehensive tests)
- **API documentation:** [Corrently API Docs](https://corrently.io/books/grunstromindex)

---

## Version Info

**Current Version:** v0.1.0
**WASM Size:** 106 KB
**Endpoints:** 1 (gsi_prediction)
**Columns:** 17
**Supabase Wrappers:** v0.1.0+

---

## Additional Resources

- **GitHub Repository:** [powabase/supabase-fdw-corrently](https://github.com/powabase/supabase-fdw-corrently)
- **API Documentation:** [Corrently GrünstromIndex](https://corrently.io/books/grunstromindex)
- **Get API Key:** [console.corrently.io](https://console.corrently.io/)
- **Supabase WASM FDW:** [Official Guide](https://supabase.com/docs/guides/database/extensions/wrappers/wasm-fdw)
- **Report Issues:** [GitHub Issues](https://github.com/powabase/supabase-fdw-corrently/issues)

---

**Need help?** Check [GitHub Issues](https://github.com/powabase/supabase-fdw-corrently/issues) or see [full documentation](README.md).

**Ready to explore?** Try the queries above and adapt them to your German postal code!
