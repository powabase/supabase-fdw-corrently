# gsi_prediction Endpoint

## Purpose

The `gsi_prediction` endpoint provides hourly green energy forecasts for German locations using the Corrently GrünstromIndex API. It returns comprehensive metrics including renewable energy percentages, CO2 emissions, dynamic pricing, and detailed breakdowns of solar and wind contributions.

**Version:** v0.2.0 (with native TIMESTAMP WITH TIME ZONE types)

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
| `postal_code` | TEXT | German postal code (Postleitzahl) | `'69168'` | **Required in WHERE clause**. Must be valid 5-digit German PLZ. |

### Optional Parameters

| Parameter | Type | Description | Default | Example | Notes |
|-----------|------|-------------|---------|---------|-------|
| `hours` | INTEGER | Limit number of forecast hours | ~113 | `24` | Use in WHERE clause to reduce response size. Range: 1-113. |

### Server Options

Configured at server level (see [README.md](../../README.md#quick-start) for setup):

| Option | Description | Required | Example |
|--------|-------------|----------|---------|
| `api_key` | Corrently API JWT token | Yes | `eyJhbGci...` |
| `api_url` | Corrently API base URL | Yes | `https://api.corrently.io` |

---

## Return Columns (v0.2.0)

All 16 columns returned per forecast hour. For complete column mapping from v0.1.0, see [MIGRATION.md](../../MIGRATION.md#complete-column-mapping).

### Temporal Columns (TIMESTAMP WITH TIME ZONE)

| Column | SQL Type | Description | Example | Notes |
|--------|----------|-------------|---------|-------|
| `forecast_start_time` | TIMESTAMPTZ | Forecast period start | `2025-10-28 14:00:00+00` | Native timestamp (no conversion needed!) |
| `forecast_period_start` | TIMESTAMPTZ | Period start time | `2025-10-28 14:00:00+00` | Parsed from nested JSON |
| `forecast_period_end` | TIMESTAMPTZ | Period end time | `2025-10-28 15:00:00+00` | 1-hour forecast window |
| `forecast_created_at` | TIMESTAMPTZ | When forecast was created | `2025-10-28 13:47:57+00` | Forecast issue timestamp |

**Note:** v0.2.0 uses native PostgreSQL timestamps instead of Unix milliseconds (BIGINT).

### Energy Metrics

| Column | SQL Type | Description | Units | Range | Example |
|--------|----------|-------------|-------|-------|---------|
| `green_energy_index` | NUMERIC | GrünstromIndex | 0-100 scale | 0-100 | 26.6 |
| `renewable_energy_pct` | BIGINT | Total renewable energy | percent | 0-100 | 28 |
| `wind_energy_pct` | BIGINT | Wind energy | percent | 0-100 | 15 |
| `solar_energy_pct` | BIGINT | Solar energy | percent | 0-100 | 8 |
| `net_wind_energy_pct` | BIGINT | Net wind energy | percent | 0-100 | 12 |
| `net_solar_energy_pct` | BIGINT | Net solar energy | percent | 0-100 | 6 |
| `smart_city_index` | BIGINT | Smart City Index | 0-100 scale | 0-100 | 25 |

**Notes:**
- `green_energy_index`: Primary indicator (0 = low renewable, 100 = high renewable)
- `renewable_energy_pct`: Total renewable energy in the grid
- `net_*`: After consumption (lower than gross values)

### Pricing & CO2 Columns

| Column | SQL Type | Description | Units | Example | Notes |
|--------|----------|-------------|-------|---------|-------|
| `energy_price_eur_kwh` | NUMERIC | Dynamic energy price | EUR/kWh | -0.014 | **Can be negative!** |
| `co2_baseline_g_kwh` | NUMERIC | Average CO2 baseline | g/kWh | 279.5 | Reference baseline |
| `standard_mix_co2_g_kwh` | BIGINT | CO2 for standard mix | g/kWh | 233 | Typical grid mix |
| `green_mix_co2_g_kwh` | BIGINT | CO2 for green mix | g/kWh | 49 | Using renewable energy |

**Notes:**
- Negative prices indicate surplus renewable energy
- Compare `standard_mix_co2_g_kwh` vs `green_mix_co2_g_kwh` for savings

### Metadata Columns

| Column | SQL Type | Description | Example |
|--------|----------|-------------|---------|
| `postal_code` | TEXT | German postal code | `'69168'` |

---

## Query Examples

For complete query examples including advanced use cases, see [README.md](../../README.md#usage-examples).

### Basic Forecast Query (v0.2.0)

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

**Key v0.2.0 Improvement:** No `TO_TIMESTAMP()` conversion needed - timestamps are native!

**Expected Output:**

| forecast_start_time | green_energy_index | renewable_energy_pct | energy_price_eur_kwh | standard_mix_co2_g_kwh |
|---------------------|-------------------|----------------------|----------------------|------------------------|
| 2025-10-28 14:00:00+00 | 26.6 | 28 | -0.014 | 233 |
| 2025-10-28 15:00:00+00 | 32.1 | 34 | -0.021 | 215 |

### Time-Based Filtering (Native TIMESTAMP Operations)

```sql
SELECT forecast_start_time, green_energy_index, energy_price_eur_kwh
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '69168'
  AND forecast_start_time < NOW() + INTERVAL '48 hours'
ORDER BY forecast_start_time
LIMIT 48;
```

**v0.2.0 Advantage:** Direct interval comparisons without `EXTRACT(EPOCH ...)` conversions!

### Forecast Period Duration Calculation

```sql
SELECT
  forecast_period_start,
  forecast_period_end,
  EXTRACT(EPOCH FROM (forecast_period_end - forecast_period_start)) / 60 as duration_minutes,
  green_energy_index
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '69168'
LIMIT 5;
```

**Expected Output:**

| forecast_period_start | forecast_period_end | duration_minutes | green_energy_index |
|-----------------------|---------------------|------------------|-------------------|
| 2025-10-28 14:00:00+00 | 2025-10-28 15:00:00+00 | 60 | 26.6 |

**v0.2.0 Advantage:** Native TIMESTAMP subtraction returns PostgreSQL intervals!

---

## Performance

| Metric | Value | Notes |
|--------|-------|-------|
| **API Latency** | 200-300ms | Corrently API response time |
| **WASM Overhead** | 50-100ms | JSON parsing + row conversion |
| **Total Query Time** | 300-400ms | End-to-end execution |
| **Response Size** | ~52 KB | JSON payload from API |
| **Rows Returned** | ~113 | One row per forecast hour |
| **Forecast Horizon** | ~4.7 days | Variable, API-dependent |

### Optimization Tips

See [README.md](../../README.md#performance) for detailed optimization strategies.

**Quick tips:**
1. Use `WHERE postal_code = '...' AND hours = 24` to reduce response size
2. Add `LIMIT` for exploration queries
3. Create materialized views for frequently accessed data

---

## API Constraints

### Geographic Scope
- Germany only (valid German postal codes required)
- 5-digit PLZ format

### Rate Limiting
- Free tier: 2,000 requests/day
- Authentication required (JWT token)

### Data Availability
- Forecast only (no historical data)
- ~113 hours forecast horizon (~4.7 days)
- Hourly granularity
- Real-time updates

---

## Troubleshooting

### Common Issues

**Missing postal_code parameter:**
```sql
-- ❌ This fails
SELECT * FROM fdw_corrently.gsi_prediction LIMIT 5;

-- ✅ This works
SELECT * FROM fdw_corrently.gsi_prediction
WHERE postal_code = '69168' LIMIT 5;
```

**NULL values in results:**
- Check WASM binary checksum: `6f182a640568669afa6294641aa074bb13a332b146516ae199505ff470d94b18`
- Verify API key is valid JWT token
- Test API directly: `curl "https://api.corrently.io/v2.0/gsi/prediction?zip=69168&token=YOUR_API_KEY"`

**Negative prices are EXPECTED:**
- Negative `energy_price_eur_kwh` indicates surplus renewable energy
- This is a feature, not a bug!
- Great opportunity for energy consumption

For more troubleshooting, see [QUICKSTART.md](../../QUICKSTART.md#troubleshooting).

---

## Related Documentation

- **[README.md](../../README.md)** - Complete project overview and usage examples
- **[MIGRATION.md](../../MIGRATION.md)** - v0.1.0 → v0.2.0 upgrade guide with column mapping
- **[QUICKSTART.md](../../QUICKSTART.md)** - 3-minute setup guide

---

**Version:** v0.2.0 | **Binary Size:** 106 KB | **Columns:** 16 (native TIMESTAMP WITH TIME ZONE)
