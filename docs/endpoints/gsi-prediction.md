# gsi_prediction Endpoint

## Purpose

The `gsi_prediction` endpoint provides hourly green energy forecasts for German locations using the Corrently GrünstromIndex API. It returns comprehensive metrics including renewable energy percentages, CO2 emissions, dynamic pricing, and detailed breakdowns of solar and wind contributions.

**Use Cases:**
- EV charging optimization (schedule during high green energy periods)
- Industrial load shifting (minimize carbon footprint)
- Dynamic pricing analysis (correlate prices with renewable availability)
- Carbon footprint tracking (monitor CO2 per kWh consumed)
- Energy trading (identify negative price periods)
- Smart grid integration (demand response based on renewable availability)

**Data Characteristics:**
- ~113 hourly forecasts (~4.7 days ahead)
- Real-time updates from Corrently API
- Geographic scope: Germany only (requires German postal codes)
- Query time: ~300-400ms

---

## Parameters

### Required Parameters

| Parameter | Type | Description | Example | Notes |
|-----------|------|-------------|---------|-------|
| `zip` | TEXT | German postal code (Postleitzahl) | `'69168'` | **Required in WHERE clause**. Must be valid 5-digit German PLZ. |

### Optional Parameters

| Parameter | Type | Description | Default | Example | Notes |
|-----------|------|-------------|---------|---------|-------|
| `hours` | INTEGER | Limit number of forecast hours | ~113 | `24` | Use in WHERE clause to reduce response size. Range: 1-113. |

### Server Options (configured at server level)

| Option | Description | Required | Example |
|--------|-------------|----------|---------|
| `api_key` | Corrently API JWT token | Yes | `eyJhbGci...` |
| `api_url` | Corrently API base URL | Yes | `https://api.corrently.io` |

---

## Return Columns

All 17 columns returned per forecast hour:

### Timestamp Columns

| Column | SQL Type | Description | Units | Example | Notes |
|--------|----------|-------------|-------|---------|-------|
| `epochtime` | BIGINT | Unix timestamp | seconds | 1761372000 | Forecast period start |
| `timestamp` | BIGINT | Unix timestamp | milliseconds | 1761372000000 | Same as epochtime × 1000 |
| `timeframe_start` | BIGINT | Forecast period start | milliseconds | 1761372000000 | Parsed from nested JSON |
| `timeframe_end` | BIGINT | Forecast period end | milliseconds | 1761375600000 | End of 1-hour period |
| `iat` | BIGINT | Issued-at timestamp | milliseconds | 1761378477642 | When forecast was created |

**Note:** Timeframe columns are parsed from nested JSON object: `timeframe.start` and `timeframe.end`

### Green Energy Metrics

| Column | SQL Type | Description | Units | Range | Example |
|--------|----------|-------------|-------|-------|---------|
| `gsi` | NUMERIC | GrünstromIndex | 0-100 scale | 0-100 | 26.6 |
| `eevalue` | BIGINT | Total renewable energy | percent | 0-100 | 28 |
| `ewind` | BIGINT | Wind energy | percent | 0-100 | 15 |
| `esolar` | BIGINT | Solar energy | percent | 0-100 | 8 |
| `enwind` | BIGINT | Net wind energy | percent | 0-100 | 12 |
| `ensolar` | BIGINT | Net solar energy | percent | 0-100 | 6 |
| `sci` | BIGINT | Smart City Index | 0-100 scale | 0-100 | 25 |

**Notes:**
- `gsi`: Primary green energy indicator (0 = low renewable, 100 = high renewable)
- `eevalue`: Total renewable energy percentage in the grid
- `ewind`, `esolar`: Gross wind and solar contributions
- `enwind`, `ensolar`: Net contributions (after consumption)
- `sci`: Metric for smart city energy optimization

### Pricing & CO2 Columns

| Column | SQL Type | Description | Units | Example | Notes |
|--------|----------|-------------|-------|---------|-------|
| `energyprice` | NUMERIC | Dynamic energy price | EUR/kWh | -0.014 | **Can be negative!** Surplus renewable energy |
| `co2_avg` | NUMERIC | Average CO2 baseline | g/kWh | 279.5 | Reference baseline |
| `co2_g_standard` | BIGINT | CO2 for standard energy mix | g/kWh | 233 | Typical grid mix |
| `co2_g_oekostrom` | BIGINT | CO2 for green energy mix | g/kWh | 49 | Using renewable energy |

**Notes:**
- `energyprice`: Parsed from string to numeric (API returns as string)
- Negative prices indicate surplus renewable energy (great time to consume!)
- `co2_g_standard` vs `co2_g_oekostrom`: Compare standard vs green energy emissions

### Metadata Columns

| Column | SQL Type | Description | Example | Notes |
|--------|----------|-------------|---------|-------|
| `zip` | TEXT | German postal code | `'69168'` | Same as query parameter |

---

## Examples

### Example 1: Basic Forecast Query

**Purpose:** Get next 10 hours of green energy forecast for Heidelberg

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

**Expected Output:**

| forecast_time       | green_index | renewable_pct | price_eur_kwh | co2_grams_kwh |
|---------------------|-------------|---------------|---------------|---------------|
| 2025-10-25 14:00:00 | 26.6        | 28            | -0.014        | 233           |
| 2025-10-25 15:00:00 | 32.1        | 34            | -0.021        | 215           |
| 2025-10-25 16:00:00 | 45.8        | 48            | -0.008        | 184           |

**Insights:**
- Negative prices indicate surplus renewable energy
- Higher GSI = greener energy
- Query time: ~300ms

---

### Example 2: Find Optimal EV Charging Windows

**Purpose:** Schedule EV charging during high green energy and low prices

```sql
SELECT
  TO_TIMESTAMP(timestamp / 1000) as time,
  gsi,
  eevalue as renewable_pct,
  energyprice,
  CASE
    WHEN gsi > 70 AND energyprice < 0 THEN 'Excellent'
    WHEN gsi > 50 AND energyprice < 0.05 THEN 'Good'
    WHEN gsi > 30 THEN 'OK'
    ELSE 'Poor'
  END as charging_rating
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168'
  AND gsi > 50
ORDER BY gsi DESC, energyprice ASC
LIMIT 10;
```

**Expected Output:**

| time                | gsi  | renewable_pct | energyprice | charging_rating |
|---------------------|------|---------------|-------------|-----------------|
| 2025-10-25 18:00:00 | 78.5 | 83            | -0.025      | Excellent       |
| 2025-10-25 19:00:00 | 72.1 | 76            | -0.018      | Excellent       |
| 2025-10-25 17:00:00 | 65.3 | 69            | 0.002       | Good            |

**Insights:**
- Best charging times have high GSI (>70) and negative prices
- Schedule charging automation based on `charging_rating`
- Can expand window by lowering GSI threshold

---

### Example 3: 24-Hour Forecast with Hours Limit

**Purpose:** Get only next 24 hours to reduce response size

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

**Expected Output:**

| time                | gsi  | renewable_pct | wind_pct | solar_pct |
|---------------------|------|---------------|----------|-----------|
| 2025-10-25 14:00:00 | 32.1 | 34            | 18       | 12        |
| 2025-10-25 15:00:00 | 38.5 | 41            | 22       | 15        |
| ...                 | ...  | ...           | ...      | ...       |

**Insights:**
- `hours = 24` reduces API response size
- Faster query execution (~250ms vs ~350ms)
- Good for dashboards showing "today + tomorrow"

---

### Example 4: Carbon Footprint Analysis

**Purpose:** Compare CO2 emissions between standard and green energy mix

```sql
SELECT
  COUNT(*) as forecast_hours,
  ROUND(AVG(co2_g_standard), 0) as avg_co2_standard,
  ROUND(AVG(co2_g_oekostrom), 0) as avg_co2_green,
  ROUND(AVG(co2_g_standard - co2_g_oekostrom), 0) as avg_savings_g_kwh,
  ROUND((AVG(co2_g_standard - co2_g_oekostrom) / AVG(co2_g_standard)::numeric) * 100, 1) as savings_pct
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168';
```

**Expected Output:**

| forecast_hours | avg_co2_standard | avg_co2_green | avg_savings_g_kwh | savings_pct |
|----------------|------------------|---------------|-------------------|-------------|
| 113            | 215              | 48            | 167               | 77.7        |

**Insights:**
- Green energy saves ~77% CO2 on average
- Use to calculate carbon footprint of energy consumption
- Can track over time for sustainability reporting

---

### Example 5: Renewable Energy Breakdown

**Purpose:** Analyze solar and wind contributions to renewable energy

```sql
SELECT
  TO_TIMESTAMP(timestamp / 1000) as time,
  eevalue as total_renewable_pct,
  ewind as wind_pct,
  esolar as solar_pct,
  ROUND((ewind::numeric / NULLIF(eevalue, 0)) * 100, 1) as wind_share_of_renewable,
  ROUND((esolar::numeric / NULLIF(eevalue, 0)) * 100, 1) as solar_share_of_renewable
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168'
  AND eevalue > 0
ORDER BY timestamp
LIMIT 20;
```

**Expected Output:**

| time                | total_renewable_pct | wind_pct | solar_pct | wind_share_of_renewable | solar_share_of_renewable |
|---------------------|---------------------|----------|-----------|-------------------------|--------------------------|
| 2025-10-25 14:00:00 | 28                  | 18       | 8         | 64.3                    | 28.6                     |
| 2025-10-25 15:00:00 | 34                  | 22       | 10        | 64.7                    | 29.4                     |

**Insights:**
- See which renewable source dominates at different times
- Wind typically higher in evening/night
- Solar peaks during midday

---

### Example 6: Negative Energy Price Analysis

**Purpose:** Identify periods with surplus renewable energy (negative prices)

```sql
SELECT
  TO_TIMESTAMP(timestamp / 1000) as time,
  gsi,
  eevalue as renewable_pct,
  energyprice,
  CASE
    WHEN energyprice < -0.02 THEN 'High Surplus'
    WHEN energyprice < 0 THEN 'Surplus'
    WHEN energyprice = 0 THEN 'Zero'
    ELSE 'Positive'
  END as price_category,
  co2_g_standard
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168'
ORDER BY energyprice ASC
LIMIT 10;
```

**Expected Output:**

| time                | gsi  | renewable_pct | energyprice | price_category | co2_g_standard |
|---------------------|------|---------------|-------------|----------------|----------------|
| 2025-10-25 18:00:00 | 78.5 | 83            | -0.042      | High Surplus   | 112            |
| 2025-10-25 17:00:00 | 72.1 | 76            | -0.028      | High Surplus   | 128            |
| 2025-10-25 19:00:00 | 65.3 | 69            | -0.014      | Surplus        | 145            |

**Insights:**
- Negative prices = excess renewable energy in grid
- Great time for energy-intensive operations
- Correlates with high GSI and low CO2

---

### Example 7: Timeframe Validation

**Purpose:** Verify forecast periods are 1-hour intervals

```sql
SELECT
  TO_TIMESTAMP(timeframe_start / 1000) as period_start,
  TO_TIMESTAMP(timeframe_end / 1000) as period_end,
  (timeframe_end - timeframe_start) / 1000 / 60 as duration_minutes,
  gsi
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168'
LIMIT 5;
```

**Expected Output:**

| period_start        | period_end          | duration_minutes | gsi  |
|---------------------|---------------------|------------------|------|
| 2025-10-25 14:00:00 | 2025-10-25 15:00:00 | 60               | 26.6 |
| 2025-10-25 15:00:00 | 2025-10-25 16:00:00 | 60               | 32.1 |

**Insights:**
- Confirms 1-hour forecast periods
- Demonstrates nested JSON parsing (`timeframe.start`, `timeframe.end`)

---

### Example 8: Aggregation Statistics

**Purpose:** Get comprehensive forecast statistics for overview

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
  ROUND(AVG(co2_g_oekostrom), 0) as avg_co2_green,
  MAX(eevalue) as peak_renewable_pct
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168';
```

**Expected Output:**

| forecast_hours | min_green_index | max_green_index | avg_green_index | min_price | max_price | avg_price | avg_co2_standard | avg_co2_green | peak_renewable_pct |
|----------------|-----------------|-----------------|-----------------|-----------|-----------|-----------|------------------|---------------|--------------------|
| 113            | 18.2            | 85.7            | 45.3            | -0.042    | 0.082     | 0.0125    | 215              | 48            | 88                 |

**Insights:**
- Full forecast horizon overview
- Wide range of green energy availability (18-86%)
- Price variation: negative to positive
- Use for dashboard summary statistics

---

## Performance Notes

### Query Performance

| Metric | Value | Notes |
|--------|-------|-------|
| **API Latency** | 200-300ms | Corrently API response time |
| **WASM Overhead** | 50-100ms | JSON parsing + row conversion |
| **Total Query Time** | 300-400ms | End-to-end execution |
| **First query (cold)** | ~400ms | Initial WASM load |
| **Subsequent queries** | ~300ms | WASM cached |

### Response Characteristics

| Metric | Value | Notes |
|--------|-------|-------|
| **Response size** | ~52 KB | JSON payload from API |
| **Forecast objects** | ~113 | Variable, API-dependent |
| **Rows returned** | ~113 | One row per forecast hour |
| **Forecast horizon** | ~113 hours | ~4.7 days ahead |
| **Update frequency** | Real-time | API updates continuously |

### Optimization Tips

1. **Use `hours` parameter** to reduce response size:
   ```sql
   WHERE zip = '69168' AND hours = 24  -- ~250ms vs ~350ms
   ```

2. **Add LIMIT for exploration**:
   ```sql
   LIMIT 10  -- Faster result processing
   ```

3. **Create materialized views** for frequently accessed data:
   ```sql
   CREATE MATERIALIZED VIEW my_forecast AS
   SELECT * FROM fdw_corrently.gsi_prediction
   WHERE zip = '69168';

   -- Refresh hourly
   REFRESH MATERIALIZED VIEW my_forecast;
   ```

4. **Use indexes on materialized views**:
   ```sql
   CREATE INDEX ON my_forecast (timestamp);
   CREATE INDEX ON my_forecast (gsi);
   ```

---

## Troubleshooting

### Issue: NULL values in results

**Symptoms:** All columns return NULL

**Causes:**
1. WASM binary not accessible
2. Checksum mismatch
3. API authentication failure

**Solutions:**
```sql
-- Verify server configuration
SELECT * FROM pg_foreign_server WHERE srvname = 'corrently_server';

-- Check options
SELECT srvoptions FROM pg_foreign_server WHERE srvname = 'corrently_server';

-- Verify checksum matches
-- Expected: 0747c2f6e9da61d27581b30716d9faa5204044419a4f796d5fb943e23143da02

-- Test API directly
-- curl "https://api.corrently.io/v2.0/gsi/prediction?zip=69168&token=YOUR_API_KEY"
```

---

### Issue: "zip parameter is required" error

**Symptoms:** Error message when running query

**Cause:** Missing `WHERE zip = '...'` clause

**Solution:**
```sql
-- ❌ This fails
SELECT * FROM fdw_corrently.gsi_prediction LIMIT 5;

-- ✅ This works
SELECT * FROM fdw_corrently.gsi_prediction
WHERE zip = '69168' LIMIT 5;
```

---

### Issue: No data returned

**Symptoms:** Query succeeds but returns 0 rows

**Causes:**
1. Invalid German postal code
2. API rate limit exceeded (2,000 requests/day)
3. Network connectivity issues

**Solutions:**
```sql
-- Verify valid German postal code (5 digits)
-- Test with known good codes: '69168', '10117', '80331'

-- Check API response directly:
-- curl "https://api.corrently.io/v2.0/gsi/prediction?zip=69168&token=YOUR_API_KEY"

-- Check rate limit headers in Supabase logs
```

---

### Issue: Slow query performance

**Symptoms:** Query takes > 1 second

**Causes:**
1. No `hours` parameter (returning full ~113 hours)
2. API latency
3. Network issues

**Solutions:**
```sql
-- Reduce dataset size
WHERE zip = '69168' AND hours = 24

-- Use LIMIT for exploration
LIMIT 10

-- Create materialized view for frequent access
CREATE MATERIALIZED VIEW berlin_24h AS
SELECT * FROM fdw_corrently.gsi_prediction
WHERE zip = '10117' AND hours = 24;
```

---

### Issue: Authentication errors

**Symptoms:** API returns authentication error

**Cause:** Invalid or missing API key

**Solutions:**
```sql
-- Get API key from https://console.corrently.io/

-- Update server options
ALTER SERVER corrently_server
OPTIONS (SET api_key 'your_new_api_key_here');

-- Verify key is JWT token (starts with 'eyJ')
```

---

### Issue: energyprice shows unexpected values

**Symptoms:** Negative prices or unusually high/low values

**Explanation:** This is **expected behavior**!

- **Negative prices** indicate surplus renewable energy (great time to consume)
- **Very low prices** (<0.01 EUR/kWh) indicate high renewable availability
- **Higher prices** (>0.05 EUR/kWh) indicate lower renewable availability

**Example:**
```sql
SELECT
  TO_TIMESTAMP(timestamp / 1000) as time,
  energyprice,
  gsi,
  eevalue as renewable_pct
FROM fdw_corrently.gsi_prediction
WHERE zip = '69168'
ORDER BY energyprice;

-- Negative prices are valid!
-- -0.042 EUR/kWh = surplus renewable energy
```

---

## API Constraints

### Geographic Scope

- **Germany only** - Requires valid German postal codes (PLZ)
- Covers all German regions (16 states)
- 5-digit postal code format

### Rate Limiting

- **Free tier:** 2,000 requests/day
- **Response headers:** `X-RateLimit-Remaining`, `X-RateLimit-Limit`
- **Error code:** 429 (Too Many Requests)

### Data Availability

- **Forecast only** - No historical data
- **Forecast horizon:** ~113 hours (~4.7 days)
- **Update frequency:** Real-time
- **Granularity:** Hourly

---

## Related Documentation

- **[QUICKSTART.md](../../QUICKSTART.md)** - 3-minute setup guide
- **[README.md](../../README.md)** - Project overview
- **[API Specification](../../phase1-research/API_SPECIFICATION.md)** - Complete API reference
- **[test_fdw.sql](../../test_fdw.sql)** - Comprehensive test suite

---

**Built with Corrently GrünstromIndex API v2.0** • **Powered by Supabase WASM FDW**
