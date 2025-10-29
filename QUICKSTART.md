# Quickstart Guide

Get Corrently GrünstromIndex green energy forecasts in your Supabase database in 3 minutes.

## Prerequisites

- Supabase project (local or hosted)
- Corrently API key - **[Get your free API key](https://console.corrently.io/)** (2,000 requests/day)

## Setup (3 steps)

### 1. Create Foreign Server

**Option A: Secure Method (Vault) - Recommended**

```sql
-- Enable extensions
CREATE EXTENSION IF NOT EXISTS wrappers WITH SCHEMA extensions;
CREATE EXTENSION IF NOT EXISTS vault WITH SCHEMA vault CASCADE;

-- Store API key in Vault
INSERT INTO vault.secrets (secret)
VALUES ('your_corrently_api_key_here')  -- Replace with your JWT token from console.corrently.io
RETURNING id;
-- Copy the returned ID (e.g., 12345678-1234-1234-1234-123456789abc)

-- Create FDW wrapper
CREATE FOREIGN DATA WRAPPER IF NOT EXISTS wasm_wrapper
  HANDLER wasm_fdw_handler VALIDATOR wasm_fdw_validator;

-- Create server with Vault secret
CREATE SERVER corrently_server FOREIGN DATA WRAPPER wasm_wrapper OPTIONS (
    fdw_package_url 'https://github.com/powabase/supabase-fdw-corrently/releases/download/v0.2.1/corrently_fdw.wasm',
    fdw_package_name 'powabase:supabase-fdw-corrently',
    fdw_package_version '0.2.1',
    fdw_package_checksum 'a57c1a9e82447047b45a7b5098eb14d4903d4c8e980128a28b219920af4863fc',
    api_url 'https://api.corrently.io',
    api_key_id '12345678-1234-1234-1234-123456789abc'  -- Use the Vault secret ID from above
);
```

**Option B: Legacy Method (Plain Text) - Deprecated ⚠️**

```sql
CREATE EXTENSION IF NOT EXISTS wrappers WITH SCHEMA extensions;

CREATE FOREIGN DATA WRAPPER IF NOT EXISTS wasm_wrapper
  HANDLER wasm_fdw_handler VALIDATOR wasm_fdw_validator;

CREATE SERVER corrently_server FOREIGN DATA WRAPPER wasm_wrapper OPTIONS (
    fdw_package_url 'https://github.com/powabase/supabase-fdw-corrently/releases/download/v0.2.1/corrently_fdw.wasm',
    fdw_package_name 'powabase:supabase-fdw-corrently',
    fdw_package_version '0.2.1',
    fdw_package_checksum 'a57c1a9e82447047b45a7b5098eb14d4903d4c8e980128a28b219920af4863fc',
    api_url 'https://api.corrently.io',
    api_key 'your_corrently_api_key_here'  -- ⚠️ DEPRECATED: Plain text (not secure)
);
```

> **Note:** Option B will display a deprecation warning. See [README Security section](README.md#security-using-vault-for-api-keys-recommended) for migration instructions.

### 2. Create Foreign Table

See [README.md](README.md#quick-start) for the complete v0.2.1 foreign table schema (16 columns).

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

**Version:** v0.2.1 | **Binary:** ~106 KB | **Columns:** 16 | **Security:** Vault support for API keys
