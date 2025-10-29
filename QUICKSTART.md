# Quickstart Guide

Get Corrently GrünstromIndex green energy forecasts in your Supabase database in 3 minutes.

## Prerequisites

- Supabase project (local or hosted)
- Corrently API key - **[Get your free API key](https://console.corrently.io/)** (2,000 requests/day)

## Setup (3 steps)

### 1. Create Foreign Server

```sql
CREATE EXTENSION IF NOT EXISTS wrappers WITH SCHEMA extensions;

CREATE FOREIGN DATA WRAPPER IF NOT EXISTS wasm_wrapper
  HANDLER wasm_fdw_handler VALIDATOR wasm_fdw_validator;

CREATE SERVER corrently_server FOREIGN DATA WRAPPER wasm_wrapper OPTIONS (
    fdw_package_url 'https://github.com/powabase/supabase-fdw-corrently/releases/download/v0.2.0/corrently_fdw.wasm',
    fdw_package_name 'powabase:supabase-fdw-corrently',
    fdw_package_version '0.2.0',
    fdw_package_checksum '6f182a640568669afa6294641aa074bb13a332b146516ae199505ff470d94b18',
    api_url 'https://api.corrently.io',
    api_key 'your_corrently_api_key_here'  -- Replace with your JWT token from console.corrently.io
);
```

### 2. Create Foreign Table

See [README.md](README.md#quick-start) for the complete v0.2.0 foreign table schema (16 columns).

### 3. Query Data

```sql
SELECT forecast_start_time, green_energy_index, renewable_energy_pct, energy_price_eur_kwh
FROM fdw_corrently.gsi_prediction
WHERE postal_code = '69168'  -- Heidelberg
LIMIT 10;
```

**Query time:** ~300-400ms | **Rows:** ~113 (4.7 days forecast)

## Next Steps

- **More examples:** See [README.md](README.md#usage-examples) for EV charging, carbon analysis, etc.
- **Migration from v0.1.0:** See [MIGRATION.md](MIGRATION.md) for upgrade guide
- **API reference:** See [docs/endpoints/gsi-prediction.md](docs/endpoints/gsi-prediction.md)
- **Troubleshooting:** Check [GitHub Issues](https://github.com/powabase/supabase-fdw-corrently/issues)

---

**Version:** v0.2.0 | **Binary:** 106 KB | **Columns:** 16 (TIMESTAMP WITH TIME ZONE for temporal fields)
