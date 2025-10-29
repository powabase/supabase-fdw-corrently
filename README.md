# Corrently GrünstromIndex WASM FDW

WebAssembly Foreign Data Wrapper for PostgreSQL enabling SQL queries against the Corrently GrünstromIndex API.

## Overview

This wrapper allows you to query hourly green energy forecasts from [Corrently GrünstromIndex](https://corrently.io) using standard SQL:

```sql
SELECT * FROM fdw_corrently.gsi_prediction
WHERE postal_code = '69168'
LIMIT 10;
```

A standalone WASM FDW that can be used with any Supabase project.

**🚀 Want to get started immediately?** See [QUICKSTART.md](QUICKSTART.md) for a 3-minute setup guide.

## Features

- ✅ **1 Production Endpoint** - gsi_prediction (hourly green energy forecasts)
- ✅ **16 Standardized Columns** - Complete forecast metrics (green energy index, CO2, pricing, renewable breakdown)
- ✅ **Native PostgreSQL Types** - TIMESTAMP WITH TIME ZONE for temporal fields (v0.2.0)
- ✅ **Standards-Compliant** - Follows PostgreSQL naming conventions and type best practices
- ✅ **WHERE Clause Pushdown** - Efficient API parameter translation (postal_code, hours)
- ✅ **~113 Hourly Forecasts** - 4.7 days ahead forecast horizon
- ✅ **WASM-Based** - Works on hosted Supabase (no native extensions needed)
- ✅ **Sub-1-Second Response** - ~300-400ms query execution
- ✅ **Cleaner SQL Queries** - No TO_TIMESTAMP() conversions needed!

## Available Endpoint

| Endpoint | Rows | Use Case | Version |
|----------|------|----------|---------|
| **gsi_prediction** | ~113 | 🌱 Hourly green energy forecasting with CO2 and pricing data | **v0.2.0** |

**⚠️ Breaking Changes in v0.2.0:** All column names standardized, temporal fields now use `TIMESTAMP WITH TIME ZONE`. See [MIGRATION.md](MIGRATION.md) for upgrade guide.

## Quick Start

**For Users:** Just want to use the FDW? See **[QUICKSTART.md](QUICKSTART.md)** for a 3-minute setup guide.

**For Developers:** Building from source? See below.

### Building from Source

**Prerequisites:**
- Rust (stable)
- cargo-component 0.21.1
- Supabase CLI ≥ 1.187.10

**Build:**
```bash
git clone https://github.com/powabase/supabase-fdw-corrently.git
cd supabase-fdw-corrently
cargo component build --release --target wasm32-unknown-unknown
# Output: target/wasm32-unknown-unknown/release/corrently_fdw.wasm
```

**Deploy:** See [QUICKSTART.md](QUICKSTART.md) for deployment instructions.

```sql
-- Enable wrappers extension
CREATE EXTENSION IF NOT EXISTS wrappers WITH SCHEMA extensions;

-- Create WASM FDW wrapper
CREATE FOREIGN DATA WRAPPER IF NOT EXISTS wasm_wrapper
  HANDLER wasm_fdw_handler
  VALIDATOR wasm_fdw_validator;

-- Create foreign server
CREATE SERVER corrently_server
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
CREATE SCHEMA fdw_corrently;

-- Create foreign table (v0.2.0 schema)
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

## Usage Examples

### Basic Forecast Query

Get hourly green energy forecast for Heidelberg (v0.2.0):

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

**Expected Output:**
| forecast_start_time | green_energy_index | renewable_energy_pct | energy_price_eur_kwh | standard_mix_co2_g_kwh |
|---------------------|-------------------|----------------------|----------------------|------------------------|
| 2025-10-28 14:00:00+00 | 26.6 | 28 | -0.014 | 233 |
| 2025-10-28 15:00:00+00 | 32.1 | 34 | -0.021 | 215 |

**Note:** Timestamps are now native `TIMESTAMP WITH TIME ZONE` - no `TO_TIMESTAMP()` conversion needed!

### Find Optimal EV Charging Windows

Schedule charging during high green energy availability and low prices:

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
WHERE postal_code = '69168'
  AND green_energy_index > 50
ORDER BY green_energy_index DESC, energy_price_eur_kwh ASC
LIMIT 10;
```

### 24-Hour Forecast with Hours Limit

Get only the next 24 hours of forecasts:

```sql
SELECT
  forecast_start_time,
  green_energy_index,
  renewable_energy_pct,
  wind_energy_pct,
  solar_energy_pct
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '10117' AND hours = 24
ORDER BY forecast_start_time
LIMIT 24;
```

### Carbon Footprint Analysis

Compare CO2 emissions between standard and green energy mix:

```sql
SELECT
  AVG(standard_mix_co2_g_kwh) as avg_co2_standard,
  AVG(green_mix_co2_g_kwh) as avg_co2_green,
  AVG(standard_mix_co2_g_kwh - green_mix_co2_g_kwh) as avg_savings_g_kwh,
  ROUND((AVG(standard_mix_co2_g_kwh - green_mix_co2_g_kwh) / AVG(standard_mix_co2_g_kwh)::numeric) * 100, 1) as savings_pct
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '69168';
```

### Renewable Energy Breakdown

Analyze solar and wind contributions to renewable energy:

```sql
SELECT
  forecast_start_time,
  renewable_energy_pct,
  wind_energy_pct,
  solar_energy_pct,
  ROUND((wind_energy_pct::numeric / NULLIF(renewable_energy_pct, 0)) * 100, 1) as wind_share_of_renewable,
  ROUND((solar_energy_pct::numeric / NULLIF(renewable_energy_pct, 0)) * 100, 1) as solar_share_of_renewable
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '69168'
  AND renewable_energy_pct > 0
ORDER BY forecast_start_time
LIMIT 20;
```

### Negative Energy Price Analysis

Identify periods with surplus renewable energy (negative prices):

```sql
SELECT
  forecast_start_time,
  green_energy_index,
  renewable_energy_pct,
  energy_price_eur_kwh,
  CASE
    WHEN energy_price_eur_kwh < 0 THEN 'Surplus (negative)'
    WHEN energy_price_eur_kwh = 0 THEN 'Zero'
    ELSE 'Positive'
  END as price_category
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '69168'
ORDER BY energy_price_eur_kwh ASC
LIMIT 10;
```

### Aggregation Statistics

Get comprehensive forecast statistics:

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

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    SQL Query (v0.2.0)                    │
│  SELECT * FROM fdw_corrently.gsi_prediction             │
│  WHERE postal_code = '69168'                            │
└──────────────────────┬──────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────┐
│              PostgreSQL / Supabase                       │
│         (Identifies foreign table)                       │
└──────────────────────┬──────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────┐
│            WASM FDW Wrapper (v0.2.0)                     │
│  1. Extracts WHERE clause: postal_code = '69168'       │
│  2. Builds API request with token (maps to zip param)   │
│  3. Executes HTTP GET to Corrently API                  │
│  4. Parses JSON response (forecast array)               │
│  5. Flattens 113 forecast objects to 113 SQL rows      │
│  6. Converts nested timeframe objects                   │
│  7. Converts milliseconds → TIMESTAMP WITH TIME ZONE    │
│  8. Standardizes column names                           │
└──────────────────────┬──────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────┐
│           Corrently GrünstromIndex API                   │
│  GET /v2.0/gsi/prediction?zip=69168&token=...           │
│  Returns: ~52 KB JSON with 113 forecast objects         │
└─────────────────────────────────────────────────────────┘
```

## Why WASM?

Hosted Supabase instances cannot install native PostgreSQL extensions. WASM FDW enables custom foreign data wrappers through:

1. **Dynamic loading from URL** - Load from GitHub releases, no database restart
2. **Sandboxed execution** - Security through WebAssembly isolation
3. **No database restart required** - Hot-load new FDW wrappers
4. **Near-native performance** - Compiled WASM executes efficiently

## Documentation

**Getting Started:**
- **[QUICKSTART.md](QUICKSTART.md)** - 3-minute setup guide ⭐
- **[MIGRATION.md](MIGRATION.md)** - Upgrade guide from v0.1.0 → v0.2.0 ⚠️
- **[API Signup](https://console.corrently.io/)** - Get your free Corrently API key

**Reference:**
- **[gsi_prediction Endpoint](docs/endpoints/gsi-prediction.md)** - Complete endpoint documentation
- **[API Specification](phase1-research/API_SPECIFICATION.md)** - Corrently API v2.0 reference

**Development:**
- **[CLAUDE.md](CLAUDE.md)** - AI assistant development guide

### Project Structure

```
supabase-fdw-corrently/
├── src/
│   └── lib.rs                    # Main FDW implementation (490 lines)
├── wit/
│   └── world.wit                 # WASM interface definitions
├── .github/
│   └── workflows/
│       └── release.yml           # Automated build & release (Phase 6)
├── Cargo.toml                    # Rust configuration
├── README.md                     # This file
├── QUICKSTART.md                 # 3-minute setup guide
├── CLAUDE.md                     # AI development guide
├── setup_fdw.sql                 # Foreign server/table creation
├── test_fdw.sql                  # Comprehensive test suite (12 queries)
├── phase1-research/              # API research and specification
├── docs/
│   └── endpoints/
│       └── gsi-prediction.md     # Endpoint reference
└── PHASE4_HANDOFF.md            # Testing results documentation
```

### Key Implementation Files

- **src/lib.rs** - Core FDW logic (init, begin_scan, iter_scan, end_scan)
- **wit/world.wit** - WebAssembly Interface Type (WIT) definitions
- **Cargo.toml** - Dependencies and build configuration with size optimizations

## Key Architecture Decisions

- **Standards-Compliant Naming (v0.2.0)** - All columns use clear, descriptive names with explicit units (e.g., `_eur_kwh`, `_g_kwh`, `_pct`)
- **Native Temporal Types (v0.2.0)** - TIMESTAMP WITH TIME ZONE for all temporal fields (milliseconds → microseconds conversion in WASM)
- **Single-Endpoint Binary** - Focused WASM wrapper for gsi_prediction
- **Array Flattening** - Corrently returns ~113 forecast objects, flattened to ~113 SQL rows
- **Nested JSON Parsing** - Safe `.get()` access for nested timeframe objects
- **String Parsing** - energy_price_eur_kwh field requires string-to-numeric conversion
- **OpenWeather + Energy Charts Hybrid** - Combines authentication patterns with array handling
- **Host Version ^0.1.0** - Critical requirement for Supabase Wrappers compatibility

## Performance

- **API Latency:** 200-300ms per request
- **WASM Overhead:** 50-100ms (parsing and row conversion)
- **Total Query Time:** ~300-400ms
- **Response Size:** ~52 KB JSON (113 forecast objects)
- **Data Points:** ~113 rows per query (can limit with hours parameter)
- **Binary Size:** 106 KB (optimized for fast download)
- **Forecast Horizon:** ~113 hours (4.7 days ahead)

## Geographic Scope & Limitations

**Geographic Scope:**
- **Germany only** - Requires valid German postal codes (PLZ)
- Covers all German regions (5-digit postal codes)

**Current Limitations:**
- No historical data (forecasts only)
- Rate limit: 2,000 requests/day (authenticated tier)
- `import_foreign_schema()` not yet implemented (manual table creation required)
- Requires API key signup at [console.corrently.io](https://console.corrently.io/)

**API Constraints:**
- Valid German postal code required in WHERE clause
- Optional hours parameter (1-113)
- JWT token authentication required

## Use Cases

1. **EV Charging Optimization** - Schedule charging during high GSI (green energy) periods
2. **Industrial Load Shifting** - Minimize carbon footprint by timing energy-intensive operations
3. **Dynamic Pricing Analysis** - Correlate energy prices with renewable availability
4. **Carbon Footprint Tracking** - Monitor and report CO2 emissions per kWh consumed
5. **Smart Grid Integration** - Enable demand response based on renewable energy availability
6. **Energy Trading** - Identify negative price periods (surplus renewable energy)

## Contributing

Contributions are welcome! Please:

1. Read [CLAUDE.md](CLAUDE.md) for development guidelines
2. Test locally with Supabase CLI before creating PR
3. Update endpoint documentation for schema changes
4. Ensure WASM binary size stays < 150 KB
5. Verify zero WASI CLI imports (`wasm-tools component wit` should show none)
6. Follow Supabase v2 API patterns
7. Run test suite (`test_fdw.sql`) before submitting

## License

Apache 2.0 (matches Supabase Wrappers framework)

## Related Projects

- [Supabase Wrappers](https://github.com/supabase/wrappers) - WASM FDW framework
- [Corrently GrünstromIndex](https://corrently.io) - Green energy data source
- [Powabase](https://github.com/powabase) - Renewable energy data platform using FDW wrappers
- [Energy Charts FDW](https://github.com/powabase/powabase-fdw-energy-charts) - Multi-endpoint reference implementation

## Support

- **Documentation:** See `docs/` folder and [QUICKSTART.md](QUICKSTART.md)
- **Issues:** [GitHub Issues](https://github.com/powabase/supabase-fdw-corrently/issues)
- **API Documentation:** [Corrently API Docs](https://corrently.io/books/grunstromindex)
- **API Signup:** [Get Free API Key](https://console.corrently.io/)
- **Supabase WASM FDW:** [Official Guide](https://supabase.com/blog/postgres-foreign-data-wrappers-with-wasm)

---

**Built with Rust, WebAssembly, and Supabase** • **Powered by Corrently GrünstromIndex API**
